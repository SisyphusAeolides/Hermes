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

pub mod sim_platform;

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
    pub const LIB_NVIDIA_ML: &str = "libnvidia-ml.so.1";
    pub const LIB_CUDA: &str = "libcuda.so.1";
    pub const LIB_GLX_NVIDIA: &str = "libGLX_nvidia.so.0";
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
}
