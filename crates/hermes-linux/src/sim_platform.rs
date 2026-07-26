//! In-process HermesPlatform double for host tests and controlled bring-up.
//!
//! Records isolation/MMIO/DMA calls. Isolation can be forced to fail so the
//! shared sequencer never reaches Online without a live domain.
//!
//! Single-threaded test use: methods take `&self` to match `HermesPlatform`
//! and mutate through `UnsafeCell`. Do not share across threads while calling.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use hermes_abi::hermes::HermesPciIdentity;
use hermes_core::{
    DmaPurpose, DmaRegion, HermesFault, HermesPlatform, MmioWindow, is_nvidia_turing_or_newer,
};

pub type SimDomain = u32;
pub type SimMmio = u32;
pub type SimDma = u32;

const MAX_DOMAINS: usize = 8;
const MAX_MMIO: usize = 16;
const MAX_DMA: usize = 16;
/// Sparse MMIO slots — GSP Falcon mailboxes sit near 0x11_0040, far past 4 KiB.
const SPARSE_MMIO: usize = 128;
/// Simulated BAR0 window large enough for published GSP register block.
const BAR0_LENGTH: u64 = 0x20_0000;
const DMA_CAP: usize = 4096;

#[derive(Clone, Copy)]
struct DomainRec {
    live: bool,
    #[allow(dead_code)]
    identity: HermesPciIdentity,
}

#[derive(Clone, Copy)]
struct SparseWord {
    live: bool,
    offset: u32,
    value: u32,
}

#[derive(Clone, Copy)]
struct MmioRec {
    live: bool,
    #[allow(dead_code)]
    domain: SimDomain,
    #[allow(dead_code)]
    bar: u8,
    length: u64,
    sparse: [SparseWord; SPARSE_MMIO],
    /// When true, HELLO cmd on MAILBOX0 auto-fills ACK on MAILBOX1 (silicon sim).
    auto_mailbox_ack: bool,
}

#[derive(Clone, Copy)]
struct DmaRec {
    live: bool,
    #[allow(dead_code)]
    domain: SimDomain,
    #[allow(dead_code)]
    device_address: u64,
    length: usize,
    #[allow(dead_code)]
    purpose: DmaPurpose,
    bytes: [u8; DMA_CAP],
}

struct SimState {
    domains: [DomainRec; MAX_DOMAINS],
    mmio: [MmioRec; MAX_MMIO],
    dma: [DmaRec; MAX_DMA],
    fail_isolation: bool,
    auto_mailbox_ack: bool,
}

/// Configurable platform used by unit tests and hermes-ctl bring-up probes.
pub struct SimPlatform {
    state: UnsafeCell<SimState>,
    next_domain: AtomicU32,
    next_mmio: AtomicU32,
    next_dma: AtomicU32,
    tick: AtomicU64,
    isolate_calls: AtomicU32,
    map_bar_calls: AtomicU32,
    dma_alloc_calls: AtomicU32,
    read32_calls: AtomicU32,
    write32_calls: AtomicU32,
}

// Safety: tests use one platform per thread; HermesPlatform requires Sync.
unsafe impl Sync for SimPlatform {}

impl Default for SimPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl SimPlatform {
    pub fn new() -> Self {
        let empty_id = HermesPciIdentity {
            segment: 0,
            bus: 0,
            slot: 0,
            function: 0,
            revision: 0,
            vendor_id: 0,
            device_id: 0,
            subsystem_vendor_id: 0,
            subsystem_device_id: 0,
            class_code: 0,
            subclass: 0,
            programming_interface: 0,
            reserved: 0,
        };
        let domains = [DomainRec {
            live: false,
            identity: empty_id,
        }; MAX_DOMAINS];
        let mmio = [MmioRec {
            live: false,
            domain: 0,
            bar: 0,
            length: 0,
            sparse: [SparseWord {
                live: false,
                offset: 0,
                value: 0,
            }; SPARSE_MMIO],
            auto_mailbox_ack: false,
        }; MAX_MMIO];
        let dma = [DmaRec {
            live: false,
            domain: 0,
            device_address: 0,
            length: 0,
            purpose: DmaPurpose::Firmware,
            bytes: [0; DMA_CAP],
        }; MAX_DMA];
        Self {
            state: UnsafeCell::new(SimState {
                domains,
                mmio,
                dma,
                fail_isolation: false,
                auto_mailbox_ack: false,
            }),
            next_domain: AtomicU32::new(1),
            next_mmio: AtomicU32::new(1),
            next_dma: AtomicU32::new(1),
            tick: AtomicU64::new(1),
            isolate_calls: AtomicU32::new(0),
            map_bar_calls: AtomicU32::new(0),
            dma_alloc_calls: AtomicU32::new(0),
            read32_calls: AtomicU32::new(0),
            write32_calls: AtomicU32::new(0),
        }
    }

    pub fn set_fail_isolation(&self, fail: bool) {
        // Safety: single-threaded test contract.
        unsafe {
            (*self.state.get()).fail_isolation = fail;
        }
    }

    /// Enable Falcon MAILBOX0 HELLO → MAILBOX1 ACK auto-response (silicon sim).
    pub fn set_auto_mailbox_ack(&self, enable: bool) {
        unsafe {
            (*self.state.get()).auto_mailbox_ack = enable;
        }
    }

    pub fn isolate_calls(&self) -> u32 {
        self.isolate_calls.load(Ordering::Relaxed)
    }

    pub fn map_bar_calls(&self) -> u32 {
        self.map_bar_calls.load(Ordering::Relaxed)
    }

    pub fn dma_alloc_calls(&self) -> u32 {
        self.dma_alloc_calls.load(Ordering::Relaxed)
    }

    pub fn read32_calls(&self) -> u32 {
        self.read32_calls.load(Ordering::Relaxed)
    }

    pub fn write32_calls(&self) -> u32 {
        self.write32_calls.load(Ordering::Relaxed)
    }

    fn state_mut(&self) -> &mut SimState {
        // Safety: single-threaded test contract for SimPlatform.
        unsafe { &mut *self.state.get() }
    }

    fn domain_slot(handle: SimDomain) -> Result<usize, HermesFault> {
        if handle == 0 || (handle as usize) > MAX_DOMAINS {
            return Err(HermesFault::DeviceIsolation);
        }
        Ok((handle as usize) - 1)
    }

    fn mmio_slot(handle: SimMmio) -> Result<usize, HermesFault> {
        if handle == 0 || (handle as usize) > MAX_MMIO {
            return Err(HermesFault::MmioOutOfRange);
        }
        Ok((handle as usize) - 1)
    }

    fn dma_slot(handle: SimDma) -> Result<usize, HermesFault> {
        if handle == 0 || (handle as usize) > MAX_DMA {
            return Err(HermesFault::DmaAccess);
        }
        Ok((handle as usize) - 1)
    }
}

fn sparse_write(slot: &mut MmioRec, offset: u32, value: u32) -> Result<(), HermesFault> {
    for e in &mut slot.sparse {
        if e.live && e.offset == offset {
            e.value = value;
            return Ok(());
        }
    }
    for e in &mut slot.sparse {
        if !e.live {
            *e = SparseWord {
                live: true,
                offset,
                value,
            };
            return Ok(());
        }
    }
    Err(HermesFault::MmioWrite)
}

impl HermesPlatform for SimPlatform {
    type Domain = SimDomain;
    type Mmio = SimMmio;
    type Dma = SimDma;

    fn isolate_device(&self, identity: HermesPciIdentity) -> Result<Self::Domain, HermesFault> {
        self.isolate_calls.fetch_add(1, Ordering::Relaxed);
        let st = self.state_mut();
        if st.fail_isolation {
            return Err(HermesFault::DeviceIsolation);
        }
        if identity.vendor_id != hermes_core::NVIDIA_VENDOR_ID {
            return Err(HermesFault::NotNvidia);
        }
        if !is_nvidia_turing_or_newer(identity.device_id) {
            return Err(HermesFault::UnsupportedArchitecture);
        }
        let id = self.next_domain.fetch_add(1, Ordering::Relaxed);
        if id as usize > MAX_DOMAINS {
            return Err(HermesFault::DeviceIsolation);
        }
        let idx = (id as usize) - 1;
        st.domains[idx] = DomainRec {
            live: true,
            identity,
        };
        Ok(id)
    }

    fn release_domain(&self, domain: Self::Domain) {
        if let Ok(idx) = Self::domain_slot(domain) {
            let st = self.state_mut();
            st.domains[idx].live = false;
        }
    }

    fn map_bar(
        &self,
        domain: Self::Domain,
        bar: u8,
        minimum_length: u64,
    ) -> Result<MmioWindow<Self::Mmio>, HermesFault> {
        self.map_bar_calls.fetch_add(1, Ordering::Relaxed);
        let st = self.state_mut();
        let didx = Self::domain_slot(domain)?;
        if !st.domains[didx].live {
            return Err(HermesFault::DeviceIsolation);
        }
        if bar > 5 {
            return Err(HermesFault::BarUnavailable);
        }
        let length = core::cmp::max(minimum_length, BAR0_LENGTH);
        let id = self.next_mmio.fetch_add(1, Ordering::Relaxed);
        if id as usize > MAX_MMIO {
            return Err(HermesFault::BarUnavailable);
        }
        let idx = (id as usize) - 1;
        let auto = st.auto_mailbox_ack;
        st.mmio[idx] = MmioRec {
            live: true,
            domain,
            bar,
            length,
            sparse: [SparseWord {
                live: false,
                offset: 0,
                value: 0,
            }; SPARSE_MMIO],
            auto_mailbox_ack: auto,
        };
        // Synthetic "device ready" status word at offset 0 used by bring-up smoke.
        st.mmio[idx].sparse[0] = SparseWord {
            live: true,
            offset: 0,
            value: 0x1,
        };
        Ok(MmioWindow {
            handle: id,
            bar,
            length,
        })
    }

    fn unmap_bar(&self, window: MmioWindow<Self::Mmio>) {
        if let Ok(idx) = Self::mmio_slot(window.handle) {
            self.state_mut().mmio[idx].live = false;
        }
    }

    fn read32(&self, window: MmioWindow<Self::Mmio>, offset: u32) -> Result<u32, HermesFault> {
        self.read32_calls.fetch_add(1, Ordering::Relaxed);
        let st = self.state_mut();
        let idx = Self::mmio_slot(window.handle)?;
        let slot = &st.mmio[idx];
        if !slot.live {
            return Err(HermesFault::MmioRead);
        }
        if offset as u64 + 4 > slot.length || offset % 4 != 0 {
            return Err(HermesFault::MmioOutOfRange);
        }
        for e in &slot.sparse {
            if e.live && e.offset == offset {
                return Ok(e.value);
            }
        }
        Ok(0)
    }

    fn write32(
        &self,
        window: MmioWindow<Self::Mmio>,
        offset: u32,
        value: u32,
    ) -> Result<(), HermesFault> {
        self.write32_calls.fetch_add(1, Ordering::Relaxed);
        let st = self.state_mut();
        let idx = Self::mmio_slot(window.handle)?;
        let slot = &mut st.mmio[idx];
        if !slot.live {
            return Err(HermesFault::MmioWrite);
        }
        if offset as u64 + 4 > slot.length || offset % 4 != 0 {
            return Err(HermesFault::MmioOutOfRange);
        }
        sparse_write(slot, offset, value)?;
        // Falcon HELLO auto-ack (MAILBOX0 @ 0x00110040, MAILBOX1 @ 0x00110044).
        const MB0: u32 = 0x0011_0040;
        const MB1: u32 = 0x0011_0044;
        const HELLO: u32 = 0x4845_4c4c;
        const ACK: u32 = 0x4143_4b21;
        if slot.auto_mailbox_ack && offset == MB0 && value == HELLO {
            sparse_write(slot, MB1, ACK)?;
        }
        Ok(())
    }

    fn io_fence(&self) -> Result<(), HermesFault> {
        Ok(())
    }

    fn allocate_dma(
        &self,
        domain: Self::Domain,
        length: usize,
        alignment: usize,
        purpose: DmaPurpose,
    ) -> Result<DmaRegion<Self::Dma>, HermesFault> {
        self.dma_alloc_calls.fetch_add(1, Ordering::Relaxed);
        let st = self.state_mut();
        let didx = Self::domain_slot(domain)?;
        if !st.domains[didx].live {
            return Err(HermesFault::DeviceIsolation);
        }
        if length == 0 || length > DMA_CAP {
            return Err(HermesFault::DmaAllocation);
        }
        if alignment == 0 || !alignment.is_power_of_two() {
            return Err(HermesFault::DmaAllocation);
        }
        let id = self.next_dma.fetch_add(1, Ordering::Relaxed);
        if id as usize > MAX_DMA {
            return Err(HermesFault::DmaAllocation);
        }
        let idx = (id as usize) - 1;
        let device_address = 0x8000_0000u64 + (u64::from(id) << 16);
        st.dma[idx] = DmaRec {
            live: true,
            domain,
            device_address,
            length,
            purpose,
            bytes: [0; DMA_CAP],
        };
        Ok(DmaRegion {
            handle: id,
            device_address,
            length,
            alignment,
            purpose,
        })
    }

    fn release_dma(&self, region: DmaRegion<Self::Dma>) {
        if let Ok(idx) = Self::dma_slot(region.handle) {
            self.state_mut().dma[idx].live = false;
        }
    }

    fn dma_write(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), HermesFault> {
        let st = self.state_mut();
        let idx = Self::dma_slot(region.handle)?;
        let slot = &mut st.dma[idx];
        if !slot.live {
            return Err(HermesFault::DmaAccess);
        }
        let end = offset.checked_add(bytes.len()).ok_or(HermesFault::DmaAddressOverflow)?;
        if end > slot.length {
            return Err(HermesFault::DmaAccess);
        }
        slot.bytes[offset..end].copy_from_slice(bytes);
        Ok(())
    }

    fn dma_read(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        bytes: &mut [u8],
    ) -> Result<(), HermesFault> {
        let st = self.state_mut();
        let idx = Self::dma_slot(region.handle)?;
        let slot = &st.dma[idx];
        if !slot.live {
            return Err(HermesFault::DmaAccess);
        }
        let end = offset.checked_add(bytes.len()).ok_or(HermesFault::DmaAddressOverflow)?;
        if end > slot.length {
            return Err(HermesFault::DmaAccess);
        }
        bytes.copy_from_slice(&slot.bytes[offset..end]);
        Ok(())
    }

    fn dma_publish(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        length: usize,
    ) -> Result<(), HermesFault> {
        let st = self.state_mut();
        let idx = Self::dma_slot(region.handle)?;
        let slot = &st.dma[idx];
        if !slot.live {
            return Err(HermesFault::DmaAccess);
        }
        let end = offset.checked_add(length).ok_or(HermesFault::DmaAddressOverflow)?;
        if end > slot.length {
            return Err(HermesFault::DmaAccess);
        }
        Ok(())
    }

    fn dma_acquire(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        length: usize,
    ) -> Result<(), HermesFault> {
        self.dma_publish(region, offset, length)
    }

    fn now_tick(&self) -> u64 {
        self.tick.fetch_add(1, Ordering::Relaxed)
    }

    fn relax(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::{NVIDIA_VENDOR_ID, pci_identity};

    #[test]
    fn isolation_mmio_dma_paths_work_and_fail_closed() {
        let plat = SimPlatform::new();
        let id = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let domain = plat.isolate_device(id).expect("isolate");
        assert_eq!(plat.isolate_calls(), 1);

        let bar = plat.map_bar(domain, 0, 4096).expect("map");
        assert_eq!(plat.map_bar_calls(), 1);
        assert_eq!(plat.read32(bar, 0).unwrap(), 0x1);
        plat.write32(bar, 4, 0xdead_beef).unwrap();
        assert_eq!(plat.read32(bar, 4).unwrap(), 0xdead_beef);
        assert!(plat.write32_calls() >= 1);

        let dma = plat
            .allocate_dma(domain, 256, 64, DmaPurpose::Firmware)
            .expect("dma");
        assert_eq!(plat.dma_alloc_calls(), 1);
        plat.dma_write(dma, 0, b"hermes").unwrap();
        let mut buf = [0u8; 6];
        plat.dma_read(dma, 0, &mut buf).unwrap();
        assert_eq!(&buf, b"hermes");
        plat.dma_publish(dma, 0, 6).unwrap();

        plat.release_dma(dma);
        plat.unmap_bar(bar);
        plat.release_domain(domain);
    }

    #[test]
    fn forced_isolation_failure_returns_device_isolation() {
        let plat = SimPlatform::new();
        plat.set_fail_isolation(true);
        let id = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        assert_eq!(
            plat.isolate_device(id),
            Err(HermesFault::DeviceIsolation)
        );
    }

    #[test]
    fn pre_turing_isolation_rejected() {
        let plat = SimPlatform::new();
        let volta = pci_identity(NVIDIA_VENDOR_ID, 0x1db6, 0x03, 0x00);
        assert_eq!(
            plat.isolate_device(volta),
            Err(HermesFault::UnsupportedArchitecture)
        );
    }
}
