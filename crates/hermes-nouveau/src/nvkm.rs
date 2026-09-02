//! NVKM object hierarchy implemented from public interfaces: device →
//! subdev/engine → GSP.
//!
//! Mirrors Nouveau's layering without importing GPL DRM core.

use hermes_abi::hermes::HermesPciIdentity;

/// NVKM subdevice class identifiers used by GSP interrupt routing tables.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum SubdevType {
    Gsp = 1,
    Fb = 2,
    Bar = 3,
    Mmu = 4,
    Mc = 5,
    Bus = 6,
    Timer = 7,
    Instmem = 8,
    Ltc = 9,
    Fault = 10,
    Sec2 = 11,
    Fsp = 12,
}

/// Engine class identifiers (subset relevant to GSP-RM engine tables).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum EngineType {
    Gr = 1,
    Ce = 2,
    Nvdec = 3,
    Nvenc = 4,
    Nvjpg = 5,
    Ofa = 6,
    Fifo = 7,
    Disp = 8,
    Sw = 9,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectHandle {
    pub client: u32,
    pub parent: u32,
    pub object: u32,
    pub class_id: u32,
}

impl ObjectHandle {
    pub const fn root_client(client: u32) -> Self {
        Self {
            client,
            parent: 0,
            object: client,
            class_id: 0,
        }
    }
}

/// Device construction record — PCI identity + chip name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NvkmDevice {
    pub identity: HermesPciIdentity,
    pub chip_name: &'static str,
    pub cfg_nv_gsp_rm: bool,
}

impl NvkmDevice {
    pub const fn new(identity: HermesPciIdentity, chip_name: &'static str) -> Self {
        Self {
            identity,
            chip_name,
            // Nouveau defaults NvGspRm=true; Hermes keeps the same default
            // but still refuses Online without measured gates.
            cfg_nv_gsp_rm: true,
        }
    }
}

/// Subdevice lifecycle phase (oneinit/init/fini) — Nouveau subdev ops.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum SubdevPhase {
    Constructed = 0,
    OneInit = 1,
    Init = 2,
    Running = 3,
    Fini = 4,
    Destructed = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NvkmError {
    NoFirmware,
    GspDisabled,
    LoadFailed,
    BooterFailed,
    WprInactive,
    RpcTimeout,
    InvalidState,
    UnsupportedChip,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::{pci_identity, NVIDIA_VENDOR_ID};

    #[test]
    fn device_defaults_gsp_rm_on() {
        let id = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let d = NvkmDevice::new(id, "tu102");
        assert!(d.cfg_nv_gsp_rm);
    }
}
