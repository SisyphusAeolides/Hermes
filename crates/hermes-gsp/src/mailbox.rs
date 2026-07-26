//! GSP Falcon mailbox / command-queue RPC (OpenRM-shaped, fail-closed).
//!
//! Uses published TU102 GSP MMIO offsets. Does not invent ready: callers must
//! observe non-zero ready evidence from real MMIO or a test double.

use hermes_core::{HermesFault, HermesPlatform, MmioWindow};

use crate::regs::tu102::{
    NV_PGSP_FALCON_ENGINE, NV_PGSP_FALCON_ENGINE_RESET_FALSE, NV_PGSP_FALCON_ENGINE_RESET_TRUE,
    NV_PGSP_FALCON_MAILBOX0, NV_PGSP_FALCON_MAILBOX1,
};

/// Software sequence numbers for cmd/event rings (host-side tracking).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct MailboxSequence {
    pub cmd: u32,
    pub event: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MailboxError {
    Platform(HermesFault),
    NotReady,
    Protocol,
    ResetStuck,
}

impl From<HermesFault> for MailboxError {
    fn from(value: HermesFault) -> Self {
        Self::Platform(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MailboxSnapshot {
    pub mailbox0: u32,
    pub mailbox1: u32,
    pub engine: u32,
    pub ready_hint: bool,
}

/// Read Falcon mailbox pair + engine register.
pub fn snapshot_mailbox<P: HermesPlatform>(
    platform: &P,
    bar: MmioWindow<P::Mmio>,
) -> Result<MailboxSnapshot, MailboxError> {
    let mailbox0 = platform.read32(bar, NV_PGSP_FALCON_MAILBOX0)?;
    let mailbox1 = platform.read32(bar, NV_PGSP_FALCON_MAILBOX1)?;
    let engine = platform.read32(bar, NV_PGSP_FALCON_ENGINE)?;
    // Ready hint: engine not in reset and mailbox1 has a non-zero status word
    // (production decodes RM status; we only require observable activity).
    let ready_hint = (engine & NV_PGSP_FALCON_ENGINE_RESET_TRUE) == 0 && mailbox1 != 0;
    Ok(MailboxSnapshot {
        mailbox0,
        mailbox1,
        engine,
        ready_hint,
    })
}

/// Pulse Falcon engine reset (assert then deassert).
pub fn falcon_reset_pulse<P: HermesPlatform>(
    platform: &P,
    bar: MmioWindow<P::Mmio>,
) -> Result<(), MailboxError> {
    platform.write32(bar, NV_PGSP_FALCON_ENGINE, NV_PGSP_FALCON_ENGINE_RESET_TRUE)?;
    platform.io_fence()?;
    platform.write32(bar, NV_PGSP_FALCON_ENGINE, NV_PGSP_FALCON_ENGINE_RESET_FALSE)?;
    platform.io_fence()?;
    let eng = platform.read32(bar, NV_PGSP_FALCON_ENGINE)?;
    if eng & NV_PGSP_FALCON_ENGINE_RESET_TRUE != 0 {
        return Err(MailboxError::ResetStuck);
    }
    Ok(())
}

/// Post a 32-bit command token into MAILBOX0 and wait for MAILBOX1 response.
///
/// `wait_ready` polls until `mailbox1 != 0` or attempts exhausted. Fail-closed:
/// never fabricates a response.
pub fn rpc_post_u32<P: HermesPlatform>(
    platform: &P,
    bar: MmioWindow<P::Mmio>,
    command: u32,
    max_polls: u32,
) -> Result<u32, MailboxError> {
    // Clear response then post command.
    platform.write32(bar, NV_PGSP_FALCON_MAILBOX1, 0)?;
    platform.write32(bar, NV_PGSP_FALCON_MAILBOX0, command)?;
    platform.io_fence()?;

    for _ in 0..max_polls {
        let resp = platform.read32(bar, NV_PGSP_FALCON_MAILBOX1)?;
        if resp != 0 {
            return Ok(resp);
        }
        platform.relax();
    }
    Err(MailboxError::NotReady)
}

/// Evidence extracted from mailbox after firmware DMA stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MailboxEvidence {
    pub mailbox_ok: bool,
    pub ready_ok: bool,
    pub last_response: u32,
}

/// Drive a minimal boot handshake: reset → post HELLO → observe response.
pub const RPC_CMD_HELLO: u32 = 0x4845_4c4c; // 'HELL'
pub const RPC_RSP_ACK: u32 = 0x4143_4b21; // 'ACK!' — expected from test double

pub fn boot_handshake<P: HermesPlatform>(
    platform: &P,
    bar: MmioWindow<P::Mmio>,
    max_polls: u32,
) -> Result<MailboxEvidence, MailboxError> {
    falcon_reset_pulse(platform, bar)?;
    let snap = snapshot_mailbox(platform, bar)?;
    if snap.engine & NV_PGSP_FALCON_ENGINE_RESET_TRUE != 0 {
        return Err(MailboxError::ResetStuck);
    }
    match rpc_post_u32(platform, bar, RPC_CMD_HELLO, max_polls) {
        Ok(resp) => Ok(MailboxEvidence {
            mailbox_ok: true,
            ready_ok: resp != 0,
            last_response: resp,
        }),
        Err(MailboxError::NotReady) => Ok(MailboxEvidence {
            mailbox_ok: false,
            ready_ok: false,
            last_response: 0,
        }),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::cell::UnsafeCell;
    use hermes_abi::hermes::HermesPciIdentity;
    use hermes_core::{DmaPurpose, DmaRegion};

    struct Sparse {
        offs: [u32; 16],
        vals: [u32; 16],
        n: usize,
    }

    struct FakePlat {
        bar: UnsafeCell<Sparse>,
        auto_ack: bool,
    }

    // Single-threaded unit tests only.
    unsafe impl Sync for FakePlat {}

    impl FakePlat {
        fn new(auto_ack: bool) -> Self {
            Self {
                bar: UnsafeCell::new(Sparse {
                    offs: [0; 16],
                    vals: [0; 16],
                    n: 0,
                }),
                auto_ack,
            }
        }

        fn get(&self, offset: u32) -> u32 {
            let b = unsafe { &*self.bar.get() };
            for i in 0..b.n {
                if b.offs[i] == offset {
                    return b.vals[i];
                }
            }
            0
        }

        fn set(&self, offset: u32, value: u32) {
            let b = unsafe { &mut *self.bar.get() };
            for i in 0..b.n {
                if b.offs[i] == offset {
                    b.vals[i] = value;
                    return;
                }
            }
            if b.n < 16 {
                b.offs[b.n] = offset;
                b.vals[b.n] = value;
                b.n += 1;
            }
        }
    }

    impl HermesPlatform for FakePlat {
        type Domain = u32;
        type Mmio = u32;
        type Dma = u32;

        fn isolate_device(&self, _: HermesPciIdentity) -> Result<u32, HermesFault> {
            Ok(1)
        }
        fn release_domain(&self, _: u32) {}
        fn map_bar(&self, _: u32, _: u8, _: u64) -> Result<MmioWindow<u32>, HermesFault> {
            Ok(MmioWindow {
                handle: 0,
                bar: 0,
                length: 0x20_0000,
            })
        }
        fn unmap_bar(&self, _: MmioWindow<u32>) {}
        fn read32(&self, _: MmioWindow<u32>, offset: u32) -> Result<u32, HermesFault> {
            Ok(self.get(offset))
        }
        fn write32(&self, _: MmioWindow<u32>, offset: u32, value: u32) -> Result<(), HermesFault> {
            self.set(offset, value);
            if self.auto_ack && offset == NV_PGSP_FALCON_MAILBOX0 && value == RPC_CMD_HELLO {
                self.set(NV_PGSP_FALCON_MAILBOX1, RPC_RSP_ACK);
            }
            if offset == NV_PGSP_FALCON_ENGINE && value == NV_PGSP_FALCON_ENGINE_RESET_FALSE {
                self.set(offset, 0);
            }
            Ok(())
        }
        fn io_fence(&self) -> Result<(), HermesFault> {
            Ok(())
        }
        fn allocate_dma(
            &self,
            _: u32,
            length: usize,
            _: usize,
            purpose: DmaPurpose,
        ) -> Result<DmaRegion<u32>, HermesFault> {
            Ok(DmaRegion {
                handle: 1,
                device_address: 0x1000,
                length,
                alignment: 64,
                purpose,
            })
        }
        fn release_dma(&self, _: DmaRegion<u32>) {}
        fn dma_write(&self, _: DmaRegion<u32>, _: usize, _: &[u8]) -> Result<(), HermesFault> {
            Ok(())
        }
        fn dma_read(&self, _: DmaRegion<u32>, _: usize, _: &mut [u8]) -> Result<(), HermesFault> {
            Ok(())
        }
        fn dma_publish(&self, _: DmaRegion<u32>, _: usize, _: usize) -> Result<(), HermesFault> {
            Ok(())
        }
        fn dma_acquire(&self, _: DmaRegion<u32>, _: usize, _: usize) -> Result<(), HermesFault> {
            Ok(())
        }
        fn now_tick(&self) -> u64 {
            0
        }
        fn relax(&self) {}
    }

    #[test]
    fn handshake_acks_on_auto_plat() {
        let p = FakePlat::new(true);
        let bar = p.map_bar(1, 0, 4096).unwrap();
        let ev = boot_handshake(&p, bar, 8).unwrap();
        assert!(ev.mailbox_ok);
        assert!(ev.ready_ok);
        assert_eq!(ev.last_response, RPC_RSP_ACK);
    }

    #[test]
    fn handshake_not_ready_without_ack() {
        let p = FakePlat::new(false);
        let bar = p.map_bar(1, 0, 4096).unwrap();
        let ev = boot_handshake(&p, bar, 4).unwrap();
        assert!(!ev.mailbox_ok);
        assert!(!ev.ready_ok);
    }
}
