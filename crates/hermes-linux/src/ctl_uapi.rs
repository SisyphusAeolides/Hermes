//! Userspace mirror of `linux/kmod/include/hermes_ctl_uapi.h`.
//!
//! Pure layout + helpers (no I/O). Kernel chardev open/ioctl live in kmod;
//! `hermes-ctl` probes nodes with these structures.

/// Matches `HERMES_CTL_STATUS_VERSION` in the kernel uAPI.
pub const HERMES_CTL_STATUS_VERSION: u32 = 2;

/// Ioctl type byte (`'H'`).
pub const HERMES_CTL_IOCTL_BASE: u8 = 0x48;
/// Status ioctl number (matches kmod `HERMES_CTL_IOCTL_STATUS`).
pub const HERMES_CTL_IOCTL_STATUS_NR: u8 = 0x10;

pub const HERMES_MOD_NVIDIA: u32 = 1 << 0;
pub const HERMES_MOD_MODESET: u32 = 1 << 1;
pub const HERMES_MOD_UVM: u32 = 1 << 2;
pub const HERMES_MOD_DRM: u32 = 1 << 3;
pub const HERMES_MOD_PEERMEM: u32 = 1 << 4;
pub const HERMES_MOD_ALL_OPEN_STACK: u32 = HERMES_MOD_NVIDIA
    | HERMES_MOD_MODESET
    | HERMES_MOD_UVM
    | HERMES_MOD_DRM
    | HERMES_MOD_PEERMEM;

/// Compose mask from companion presence (mirrors `hermes_ctl_module_mask_compose`).
pub fn hermes_ctl_module_mask_compose(
    modeset: bool,
    uvm: bool,
    drm: bool,
    peermem: bool,
) -> u32 {
    let mut m = HERMES_MOD_NVIDIA;
    if modeset {
        m |= HERMES_MOD_MODESET;
    }
    if uvm {
        m |= HERMES_MOD_UVM;
    }
    if drm {
        m |= HERMES_MOD_DRM;
    }
    if peermem {
        m |= HERMES_MOD_PEERMEM;
    }
    m
}

/// Packed status from `/dev/nvidiactl` (or host tests).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HermesCtlStatus {
    pub gsp_online: u32,
    pub phase: u32,
    pub version: u32,
    pub module_mask: u32,
}

impl HermesCtlStatus {
    pub fn fill(online: bool, phase: u32, module_mask: u32) -> Self {
        Self {
            gsp_online: if online { 1 } else { 0 },
            phase,
            version: HERMES_CTL_STATUS_VERSION,
            module_mask,
        }
    }

    pub fn is_online(&self) -> bool {
        self.gsp_online != 0
    }

    pub fn phase_label(&self) -> &'static str {
        match self.phase {
            0 => "OFFLINE",
            1 => "PROBED",
            2 => "FIRMWARED",
            3 => "QUEUED",
            4 => "NEGOTIATED",
            5 => "ONLINE",
            6 => "RECOVERING",
            7 => "QUARANTINED",
            _ => "UNKNOWN",
        }
    }

    pub fn modules_listed(&self) -> alloc::vec::Vec<&'static str> {
        let mut v = alloc::vec::Vec::new();
        if self.module_mask & HERMES_MOD_NVIDIA != 0 {
            v.push("nvidia");
        }
        if self.module_mask & HERMES_MOD_MODESET != 0 {
            v.push("nvidia-modeset");
        }
        if self.module_mask & HERMES_MOD_UVM != 0 {
            v.push("nvidia-uvm");
        }
        if self.module_mask & HERMES_MOD_DRM != 0 {
            v.push("nvidia-drm");
        }
        if self.module_mask & HERMES_MOD_PEERMEM != 0 {
            v.push("nvidia-peermem");
        }
        v
    }
}

/// Linux `_IOR(type, nr, sizeof(HermesCtlStatus))` for x86_64 / aarch64.
pub fn hermes_ctl_ioctl_status() -> u64 {
    // _IOC(_IOC_READ, type, nr, size)
    const IOC_READ: u64 = 2;
    const SIZE: u64 = core::mem::size_of::<HermesCtlStatus>() as u64;
    (IOC_READ << 30)
        | ((HERMES_CTL_IOCTL_BASE as u64) << 8)
        | (HERMES_CTL_IOCTL_STATUS_NR as u64)
        | (SIZE << 16)
}

/// DRM status uAPI mirror (`hermes_drm_uapi.h`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HermesDrmStatus {
    pub gsp_online: u32,
    pub connectors: u32,
    pub crtcs: u32,
    pub active_crtcs: u32,
    pub version: u32,
}

pub const HERMES_DRM_IOCTL_BASE: u8 = 0x48;
pub const HERMES_DRM_IOCTL_STATUS_NR: u8 = 0x01;
pub const HERMES_DRM_IOCTL_GET_EDID_NR: u8 = 0x05;
pub const HERMES_DRM_IOCTL_GET_PROP_NR: u8 = 0x06;
pub const HERMES_DRM_EDID_MAX: usize = 128;

pub fn hermes_drm_ioctl_status() -> u64 {
    const IOC_READ: u64 = 2;
    const SIZE: u64 = core::mem::size_of::<HermesDrmStatus>() as u64;
    (IOC_READ << 30)
        | ((HERMES_DRM_IOCTL_BASE as u64) << 8)
        | (HERMES_DRM_IOCTL_STATUS_NR as u64)
        | (SIZE << 16)
}

/// `_IOWR` for GET_EDID: connector_id + size + 128 data = 4+4+128 = 136.
pub fn hermes_drm_ioctl_get_edid() -> u64 {
    const IOC_READ: u64 = 2;
    const IOC_WRITE: u64 = 1;
    const SIZE: u64 = 136;
    ((IOC_READ | IOC_WRITE) << 30)
        | ((HERMES_DRM_IOCTL_BASE as u64) << 8)
        | (HERMES_DRM_IOCTL_GET_EDID_NR as u64)
        | (SIZE << 16)
}

/// `_IOWR` for GET_PROP: object_id + prop_id + value = 4+4+8 = 16.
pub fn hermes_drm_ioctl_get_prop() -> u64 {
    const IOC_READ: u64 = 2;
    const IOC_WRITE: u64 = 1;
    const SIZE: u64 = 16;
    ((IOC_READ | IOC_WRITE) << 30)
        | ((HERMES_DRM_IOCTL_BASE as u64) << 8)
        | (HERMES_DRM_IOCTL_GET_PROP_NR as u64)
        | (SIZE << 16)
}

/// Module load presence from sysfs path names (no Online claim).
pub fn module_sysfs_path(name: &str) -> alloc::string::String {
    alloc::format!("/sys/module/{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_fill_and_modules() {
        let st = HermesCtlStatus::fill(false, 0, HERMES_MOD_NVIDIA);
        assert!(!st.is_online());
        assert_eq!(st.version, HERMES_CTL_STATUS_VERSION);
        assert_eq!(st.modules_listed(), &["nvidia"]);
        let st2 = HermesCtlStatus::fill(
            true,
            5,
            HERMES_MOD_NVIDIA | HERMES_MOD_DRM | HERMES_MOD_UVM,
        );
        assert!(st2.is_online());
        assert_eq!(st2.phase_label(), "ONLINE");
        assert_eq!(st2.modules_listed().len(), 3);
    }

    #[test]
    fn companion_mask_compose_ors_soft_deps() {
        assert_eq!(
            hermes_ctl_module_mask_compose(false, false, false, false),
            HERMES_MOD_NVIDIA
        );
        assert_eq!(
            hermes_ctl_module_mask_compose(true, true, true, true),
            HERMES_MOD_ALL_OPEN_STACK
        );
        let partial = hermes_ctl_module_mask_compose(true, false, true, false);
        assert_eq!(
            partial,
            HERMES_MOD_NVIDIA | HERMES_MOD_MODESET | HERMES_MOD_DRM
        );
        let st = HermesCtlStatus::fill(false, 0, HERMES_MOD_ALL_OPEN_STACK);
        assert_eq!(st.modules_listed().len(), 5);
    }

    #[test]
    fn ioctl_numbers_non_zero_and_sized() {
        let ctl = hermes_ctl_ioctl_status();
        let drm = hermes_drm_ioctl_status();
        assert_ne!(ctl, 0);
        assert_ne!(drm, 0);
        // size field in bits 16..29 should be 16 for ctl status.
        assert_eq!((ctl >> 16) & 0x3fff, 16);
        assert_eq!((drm >> 16) & 0x3fff, 20);
    }
}
