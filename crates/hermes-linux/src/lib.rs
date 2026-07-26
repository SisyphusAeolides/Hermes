//! Linux drop-in personality for the NVIDIA open kernel module set.
//!
//! Hermes presents the same module and device node names operators expect
//! when replacing `nvidia` / `nvidia-settings`. Activation remains fail-closed:
//! exporting these names never implies the GPU is online.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use hermes_core::{
    AdmittedDevice, HermesManifold, HermesPhase, admit_display_device,
};
use hermes_abi::hermes::HermesPciIdentity;

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
        name: modules::HERMES_GSP,
        replaces: modules::NVIDIA,
        description: "Hermes GSP-RM host / Resource Manager path",
    },
    ModuleSurface {
        name: "hermes-modeset",
        replaces: modules::NVIDIA_MODESET,
        description: "Display modeset broker (fail-closed until GSP online)",
    },
    ModuleSurface {
        name: "hermes-uvm",
        replaces: modules::NVIDIA_UVM,
        description: "Unified virtual memory personality",
    },
    ModuleSurface {
        name: "hermes-drm",
        replaces: modules::NVIDIA_DRM,
        description: "DRM/KMS bridge for modesetting clients",
    },
    ModuleSurface {
        name: "hermes-peermem",
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
    pub fn from_identity(identity: HermesPciIdentity, generation: u32) -> Result<Self, hermes_core::AdmissionError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::{NVIDIA_VENDOR_ID, pci_identity};

    #[test]
    fn module_set_covers_open_gpu_kernel_modules() {
        assert_eq!(modules::ALL_NVIDIA_SET.len(), 5);
        assert!(modules::ALL_NVIDIA_SET.contains(&"nvidia"));
        assert!(modules::ALL_NVIDIA_SET.contains(&"nvidia-drm"));
        assert_eq!(MODULE_SURFACES.len(), 5);
        for surface in MODULE_SURFACES {
            assert!(modules::ALL_NVIDIA_SET.contains(&surface.replaces));
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
}
