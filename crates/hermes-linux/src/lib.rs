//! Linux drop-in personality for the NVIDIA open kernel module set.
//!
//! Hermes presents the same module and device node names operators expect
//! when replacing `nvidia` / `nvidia-settings`. Activation remains fail-closed:
//! exporting these names never implies the GPU is online.
//!
//! The shared GSP bring-up path lives in `hermes_gsp::run_bringup` and is
//! exercised here against `SimPlatform` (tests) and documented for the
//! out-of-tree kmod under `linux/kmod/`.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod ctl_uapi;
pub mod sim_platform;

pub use ctl_uapi::{
    hermes_ctl_ioctl_status, hermes_ctl_module_mask_compose, hermes_drm_ioctl_get_edid,
    hermes_drm_ioctl_get_prop, hermes_drm_ioctl_status, module_sysfs_path, HermesCtlStatus,
    HermesDrmStatus, HERMES_CTL_STATUS_VERSION, HERMES_MOD_ALL_OPEN_STACK, HERMES_MOD_DRM,
    HERMES_MOD_MODESET, HERMES_MOD_NVIDIA, HERMES_MOD_PEERMEM, HERMES_MOD_UVM,
};

use hermes_abi::hermes::HermesPciIdentity;
use hermes_core::{
    AdmittedDevice, HermesManifold, HermesPhase, admit_display_device,
};
use hermes_gsp::{BringupReport, BringupRequest, HardwareEvidence, run_bringup};

pub use sim_platform::SimPlatform;

/// Kernel module names matching open-gpu-kernel-modules.
pub mod modules {
    pub const NVIDIA: &str = "nvidia";
    pub const NVIDIA_MODESET: &str = "nvidia-modeset";
    pub const NVIDIA_UVM: &str = "nvidia-uvm";
    pub const NVIDIA_DRM: &str = "nvidia-drm";
    pub const NVIDIA_PEERMEM: &str = "nvidia-peermem";

    /// Hermes-owned aliases that load the same personality.
    pub const HERMES: &str = "hermes";
    pub const HERMES_GSP: &str = "hermes-gsp";

    pub const ALL_NVIDIA_SET: &[&str] = &[
        NVIDIA,
        NVIDIA_MODESET,
        NVIDIA_UVM,
        NVIDIA_DRM,
        NVIDIA_PEERMEM,
    ];
}

/// Character device nodes created by the proprietary/open NVIDIA stack.
pub mod devices {
    pub const NVIDIA_CTL: &str = "/dev/nvidiactl";
    pub const NVIDIA_0: &str = "/dev/nvidia0";
    pub const NVIDIA_UVM: &str = "/dev/nvidia-uvm";
    pub const NVIDIA_UVM_TOOLS: &str = "/dev/nvidia-uvm-tools";
    pub const NVIDIA_MODESET: &str = "/dev/nvidia-modeset";
    pub const NVIDIA_CAPS: &str = "/dev/nvidia-caps";
}

/// Userspace control binaries Hermes replaces or coexists with.
pub mod userspace {
    pub const NVIDIA_SETTINGS: &str = "nvidia-settings";
    pub const NVIDIA_SMI: &str = "nvidia-smi";
    pub const NVIDIA_MODPROBE: &str = "nvidia-modprobe";
    pub const NVIDIA_PERSISTENCED: &str = "nvidia-persistenced";
    pub const NVIDIA_CUDA_MPS_CONTROL: &str = "nvidia-cuda-mps-control";
    pub const NVIDIA_DEBUGDUMP: &str = "nvidia-debugdump";
    pub const LIB_NVIDIA_ML: &str = "libnvidia-ml.so.1";
    pub const LIB_CUDA: &str = "libcuda.so.1";
    pub const LIB_CUDART: &str = "libcudart.so.12";
    pub const LIB_GLX_NVIDIA: &str = "libGLX_nvidia.so.0";
    pub const LIB_EGL_NVIDIA: &str = "libEGL_nvidia.so.0";
    pub const LIB_NVIDIA_CFG: &str = "libnvidia-cfg.so.1";
}

/// Drop-in module table entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleSurface {
    pub name: &'static str,
    pub replaces: &'static str,
    pub description: &'static str,
}

pub const MODULE_SURFACES: &[ModuleSurface] = &[
    ModuleSurface {
        name: modules::NVIDIA,
        replaces: modules::NVIDIA,
        description: "Hermes GSP-RM host / Resource Manager path (classic name)",
    },
    ModuleSurface {
        name: modules::NVIDIA_MODESET,
        replaces: modules::NVIDIA_MODESET,
        description: "Display modeset broker (fail-closed until GSP online)",
    },
    ModuleSurface {
        name: modules::NVIDIA_UVM,
        replaces: modules::NVIDIA_UVM,
        description: "Unified virtual memory personality",
    },
    ModuleSurface {
        name: modules::NVIDIA_DRM,
        replaces: modules::NVIDIA_DRM,
        description: "DRM/KMS bridge for modesetting clients",
    },
    ModuleSurface {
        name: modules::NVIDIA_PEERMEM,
        replaces: modules::NVIDIA_PEERMEM,
        description: "Peer memory registration bridge",
    },
];

/// One advertised proprietary-named drop-in surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DropInSurface {
    pub kind: &'static str,
    pub name: &'static str,
    pub hermes_crate: &'static str,
}

/// Complete catalog of Hermes' advertised open-stack NVIDIA drop-in names.
pub const DROP_IN_CATALOG: &[DropInSurface] = &[
    DropInSurface {
        kind: "kmod",
        name: modules::NVIDIA,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "kmod",
        name: modules::NVIDIA_MODESET,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "kmod",
        name: modules::NVIDIA_UVM,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "kmod",
        name: modules::NVIDIA_DRM,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "kmod",
        name: modules::NVIDIA_PEERMEM,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: devices::NVIDIA_CTL,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: devices::NVIDIA_0,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: devices::NVIDIA_UVM,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: devices::NVIDIA_UVM_TOOLS,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: devices::NVIDIA_MODESET,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: "/dev/nvidia-drm",
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: devices::NVIDIA_CAPS,
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "device",
        name: "/dev/nvidia-peermem",
        hermes_crate: "linux/kmod",
    },
    DropInSurface {
        kind: "bin",
        name: userspace::NVIDIA_SMI,
        hermes_crate: "hermes-ctl",
    },
    DropInSurface {
        kind: "bin",
        name: userspace::NVIDIA_SETTINGS,
        hermes_crate: "hermes-settings",
    },
    DropInSurface {
        kind: "bin",
        name: userspace::NVIDIA_MODPROBE,
        hermes_crate: "hermes-ctl",
    },
    DropInSurface {
        kind: "bin",
        name: userspace::NVIDIA_PERSISTENCED,
        hermes_crate: "hermes-ctl",
    },
    DropInSurface {
        kind: "bin",
        name: userspace::NVIDIA_CUDA_MPS_CONTROL,
        hermes_crate: "hermes-ctl",
    },
    DropInSurface {
        kind: "bin",
        name: userspace::NVIDIA_DEBUGDUMP,
        hermes_crate: "hermes-ctl",
    },
    DropInSurface {
        kind: "lib",
        name: userspace::LIB_NVIDIA_ML,
        hermes_crate: "hermes-nvml",
    },
    DropInSurface {
        kind: "lib",
        name: userspace::LIB_CUDA,
        hermes_crate: "hermes-cuda",
    },
    DropInSurface {
        kind: "lib",
        name: userspace::LIB_CUDART,
        hermes_crate: "hermes-cuda",
    },
    DropInSurface {
        kind: "lib",
        name: userspace::LIB_GLX_NVIDIA,
        hermes_crate: "hermes-mesa",
    },
    DropInSurface {
        kind: "lib",
        name: userspace::LIB_EGL_NVIDIA,
        hermes_crate: "hermes-mesa",
    },
    DropInSurface {
        kind: "lib",
        name: userspace::LIB_NVIDIA_CFG,
        hermes_crate: "hermes-nvml",
    },
    DropInSurface {
        kind: "surface",
        name: "DRM/KMS atomic",
        hermes_crate: "hermes-drm",
    },
    DropInSurface {
        kind: "surface",
        name: "Mesa Vulkan/GL",
        hermes_crate: "hermes-mesa",
    },
    DropInSurface {
        kind: "surface",
        name: "CCCL/Thrust host",
        hermes_crate: "hermes-cccl",
    },
    DropInSurface {
        kind: "surface",
        name: "GSP firmware stage",
        hermes_crate: "hermes-gsp",
    },
];

/// Number of catalog entries (kmod + device + bin + lib + surface).
pub fn drop_in_catalog_len() -> usize {
    DROP_IN_CATALOG.len()
}

/// Target count of classic open-stack named surfaces Hermes advertises.
pub const DROP_IN_PARITY_TARGET: usize = 29;

/// Percent of parity target covered by the live catalog (capped at 100).
pub fn drop_in_parity_percent() -> u32 {
    let n = DROP_IN_CATALOG.len();
    if n >= DROP_IN_PARITY_TARGET {
        100
    } else {
        ((n * 100) / DROP_IN_PARITY_TARGET) as u32
    }
}

/// True when catalog hits the advertised parity target count.
pub fn drop_in_parity_complete() -> bool {
    DROP_IN_CATALOG.len() >= DROP_IN_PARITY_TARGET
}

/// True when every classic open-gpu-kernel-modules module name is catalogued.
pub fn drop_in_has_all_kmod_names() -> bool {
    for m in modules::ALL_NVIDIA_SET {
        if !DROP_IN_CATALOG
            .iter()
            .any(|s| s.kind == "kmod" && s.name == *m)
        {
            return false;
        }
    }
    true
}

/// Runtime view of one GPU under the Linux personality.
#[derive(Clone, Debug)]
pub struct LinuxGpuSlot {
    pub admitted: AdmittedDevice,
    pub manifold: HermesManifold,
    pub pci_bdf: alloc::string::String,
}

impl LinuxGpuSlot {
    pub fn from_identity(
        identity: HermesPciIdentity,
        generation: u32,
    ) -> Result<Self, hermes_core::AdmissionError> {
        let admitted = admit_display_device(&identity)?;
        Ok(Self {
            admitted,
            manifold: HermesManifold::dark(generation),
            pci_bdf: alloc::format!(
                "{:04x}:{:02x}:{:02x}.{}",
                identity.segment,
                identity.bus,
                identity.slot,
                identity.function
            ),
        })
    }

    /// Apply a bring-up report produced by the shared sequencer.
    pub fn apply_bringup(&mut self, report: &BringupReport) {
        self.manifold = report.manifold;
        if let Some(admitted) = report.admitted {
            self.admitted = admitted;
        }
    }

    pub fn is_online(&self) -> bool {
        self.manifold.is_online()
    }

    pub fn phase(&self) -> HermesPhase {
        self.manifold.phase
    }

    pub fn module_provides_nvidia_name(&self) -> &'static str {
        modules::NVIDIA
    }
}

/// True when Hermes is configured to bind the classic `nvidia` module name.
pub fn drop_in_module_name(prefer_classic: bool) -> &'static str {
    if prefer_classic {
        modules::NVIDIA
    } else {
        modules::HERMES_GSP
    }
}

/// Linux surface entry: run shared bring-up on a platform and return the report.
///
/// This is the same function the out-of-tree module init path is defined to call
/// (via the C ABI in `linux/kmod` / `hermes_kmod_api`).
pub fn linux_bringup<P: hermes_core::HermesPlatform>(
    platform: &P,
    request: &BringupRequest<'_>,
) -> BringupReport {
    run_bringup(platform, request)
}

/// Convenience: full hardware evidence for simulators that model successful SEC2/GSP.
pub fn sim_full_hardware() -> HardwareEvidence {
    HardwareEvidence::full()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::{NVIDIA_VENDOR_ID, pci_identity};
    use hermes_gsp::{
        FirmwareFamily, NvidiaGspFirmwareAuthority, NvidiaGspFirmwareManifest, firmware_version,
        sha256_bytes,
    };

    #[test]
    fn module_set_covers_open_gpu_kernel_modules() {
        assert_eq!(modules::ALL_NVIDIA_SET.len(), 5);
        assert!(modules::ALL_NVIDIA_SET.contains(&"nvidia"));
        assert!(modules::ALL_NVIDIA_SET.contains(&"nvidia-drm"));
        assert_eq!(MODULE_SURFACES.len(), 5);
        for surface in MODULE_SURFACES {
            assert!(modules::ALL_NVIDIA_SET.contains(&surface.replaces));
        }
        // Classic NVIDIA names are first-class module surfaces.
        assert_eq!(MODULE_SURFACES[0].name, "nvidia");
    }

    #[test]
    fn drop_in_catalog_covers_advertised_stack() {
        assert!(drop_in_catalog_len() >= DROP_IN_PARITY_TARGET);
        assert!(drop_in_parity_complete());
        assert_eq!(drop_in_parity_percent(), 100);
        assert!(drop_in_has_all_kmod_names());
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::NVIDIA_SMI));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::NVIDIA_SETTINGS));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::NVIDIA_MODPROBE));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::NVIDIA_PERSISTENCED));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::LIB_CUDA));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::LIB_CUDART));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == userspace::LIB_EGL_NVIDIA));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == devices::NVIDIA_CTL));
        assert!(DROP_IN_CATALOG
            .iter()
            .any(|s| s.name == devices::NVIDIA_UVM_TOOLS));
        assert!(DROP_IN_CATALOG.iter().any(|s| s.name == "/dev/nvidia-drm"));
        let kinds: alloc::vec::Vec<_> = DROP_IN_CATALOG.iter().map(|s| s.kind).collect();
        for need in ["kmod", "device", "bin", "lib", "surface"] {
            assert!(kinds.contains(&need), "missing kind {need}");
        }
    }

    #[test]
    fn linux_slot_starts_offline_after_admission() {
        let id = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let slot = LinuxGpuSlot::from_identity(id, 1).unwrap();
        assert!(!slot.is_online());
        assert_eq!(slot.phase(), HermesPhase::Offline);
        assert_eq!(slot.module_provides_nvidia_name(), "nvidia");
        assert_eq!(drop_in_module_name(true), "nvidia");
    }

    #[test]
    fn device_node_paths_match_nvidia_stack() {
        assert_eq!(devices::NVIDIA_CTL, "/dev/nvidiactl");
        assert_eq!(devices::NVIDIA_0, "/dev/nvidia0");
        assert_eq!(userspace::NVIDIA_SETTINGS, "nvidia-settings");
        assert_eq!(userspace::NVIDIA_SMI, "nvidia-smi");
    }

    #[test]
    fn bringup_isolation_fail_never_online() {
        let plat = SimPlatform::new();
        plat.set_fail_isolation(true);
        let payload = b"hermes-linux-bringup-iso-fail";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        let report = linux_bringup(&plat, &req);
        assert!(!report.is_online());
        assert!(!report.manifold.is_online());
        assert!(report.fault.is_some());
        assert_eq!(plat.isolate_calls(), 1);
    }

    #[test]
    fn bringup_full_evidence_reaches_online_via_shared_path() {
        let plat = SimPlatform::new();
        let payload = b"hermes-linux-bringup-success";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        let report = linux_bringup(&plat, &req);
        assert!(report.is_online(), "fault={:?}", report.fault);
        assert_eq!(report.phase(), HermesPhase::Online);
        assert!(plat.isolate_calls() >= 1);
        assert!(plat.map_bar_calls() >= 1);
        assert!(plat.dma_alloc_calls() >= 1);

        let mut slot = LinuxGpuSlot::from_identity(identity, 1).unwrap();
        slot.apply_bringup(&report);
        assert!(slot.is_online());
    }

    #[test]
    fn bringup_missing_wpr_stays_offline_after_isolation() {
        let plat = SimPlatform::new();
        let payload = b"hermes-linux-bringup-no-wpr";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence {
            wpr_locked: false,
            boot_mailbox_ok: true,
            ready_queue_observed: true,
        };
        let report = linux_bringup(&plat, &req);
        assert!(!report.is_online());
        assert!(matches!(
            report.fault,
            Some(hermes_gsp::BringupFault::Manifold(
                hermes_core::ManifoldFault::MissingWpr
            ))
        ));
    }

    #[test]
    fn bringup_pre_turing_rejected_before_isolation() {
        let plat = SimPlatform::new();
        let payload = b"hermes-linux-volta";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1db6, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        let report = linux_bringup(&plat, &req);
        assert!(!report.is_online());
        assert_eq!(plat.isolate_calls(), 0);
    }

    #[test]
    fn bringup_wrong_firmware_hash_never_online() {
        let plat = SimPlatform::new();
        let payload = b"actual-bytes";
        let wrong = b"other-bytes!!";
        let digest = sha256_bytes(wrong);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        let report = linux_bringup(&plat, &req);
        assert!(!report.is_online());
        assert_eq!(plat.isolate_calls(), 0);
    }

    #[test]
    fn full_image_stage_publishes_all_bytes() {
        let plat = SimPlatform::new();
        // Multi-chunk payload (9 KiB → 3×4 KiB staging windows).
        let mut payload = alloc::vec![0u8; 9000];
        for (i, b) in payload.iter_mut().enumerate() {
            *b = (i % 199) as u8;
        }
        let digest = sha256_bytes(&payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, &payload, auth);
        req.hardware = HardwareEvidence::full();
        let report = linux_bringup(&plat, &req);
        assert!(report.is_online(), "fault={:?}", report.fault);
        let stage = report.stage.expect("stage");
        assert_eq!(stage.bytes_staged, 9000);
        assert_eq!(stage.chunks, 3);
        assert_eq!(stage.staged_sha256, digest);
        assert_eq!(plat.bytes_published(), 9000);
    }

    #[test]
    fn drive_mailbox_without_ack_never_online_even_if_hardware_claimed() {
        let plat = SimPlatform::new();
        // No auto_mailbox_ack — live observe must fail closed.
        let payload = b"mailbox-live-fail-closed";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        req.drive_mailbox = true;
        let report = linux_bringup(&plat, &req);
        assert!(!report.is_online(), "must not invent Online without mailbox ACK");
        assert!(report.mailbox.is_some());
        assert!(!report.final_evidence.boot_mailbox_ok);
    }

    #[test]
    fn drive_mailbox_with_ack_and_full_evidence_online() {
        let plat = SimPlatform::new();
        plat.set_auto_mailbox_ack(true);
        let payload = b"mailbox-live-success-path";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        req.drive_mailbox = true;
        let report = linux_bringup(&plat, &req);
        assert!(report.is_online(), "fault={:?}", report.fault);
        let mb = report.mailbox.expect("mailbox");
        assert!(mb.mailbox_ok && mb.ready_ok);
        assert!(report.final_evidence.boot_mailbox_ok);
    }

    #[test]
    fn host_facts_nouveau_preflight_never_online() {
        use hermes_gsp::{facts_from_sysfs, run_bringup};
        let plat = SimPlatform::new();
        let payload = b"host-preflight-nouveau";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        // Live host shape: Nouveau + no IOMMU.
        req.host_facts = Some(facts_from_sysfs(None, Some("nouveau"), true, false));
        let report = run_bringup(&plat, &req);
        assert!(!report.is_online());
        assert_eq!(plat.isolate_calls(), 0, "must fail before isolation");
        assert!(matches!(
            report.fault,
            Some(hermes_gsp::BringupFault::Hermes(
                hermes_core::HermesFault::DeviceIsolation
            ))
        ));
    }

    #[test]
    fn retain_on_online_keeps_domain_until_release() {
        use hermes_gsp::run_bringup_ex;
        let plat = SimPlatform::new();
        let payload = b"retain-session-resources";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let mut req = BringupRequest::with_defaults(identity, payload, auth);
        req.hardware = HardwareEvidence::full();
        req.retain_on_online = true;
        let outcome = run_bringup_ex(&plat, &req);
        assert!(outcome.report.is_online(), "fault={:?}", outcome.report.fault);
        assert!(outcome.report.resources_retained);
        assert!(outcome.retained.is_some());
        // Domain still live: second isolate still works (new domain).
        let report = outcome.release(&plat);
        assert!(report.is_online());
        assert!(!report.resources_retained);
    }
}
