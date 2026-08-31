//! Vendor classifications.

pub mod amd;
pub mod intel;
pub mod nvidia;

pub use amd::{amd_architecture, AmdArchitecture, AMD_VENDOR_ID};
pub use intel::{intel_architecture, IntelArchitecture, INTEL_VENDOR_ID};
pub use nvidia::{
    is_nvidia_turing_or_newer, nvidia_architecture, nvidia_architecture_hint, NvidiaArchitecture,
    NVIDIA_ARCHITECTURE_ADA, NVIDIA_ARCHITECTURE_AMPERE, NVIDIA_ARCHITECTURE_BLACKWELL,
    NVIDIA_ARCHITECTURE_HOPPER, NVIDIA_ARCHITECTURE_TURING, NVIDIA_ARCHITECTURE_TURING_OR_NEWER,
    NVIDIA_VENDOR_ID,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VendorArchitecture {
    Nvidia(NvidiaArchitecture),
    Amd(AmdArchitecture),
    Intel(IntelArchitecture),
}

impl VendorArchitecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nvidia(arch) => arch.as_str(),
            Self::Amd(arch) => match arch {
                AmdArchitecture::RDNA => "RDNA",
                AmdArchitecture::RDNA2 => "RDNA2",
                AmdArchitecture::RDNA3 => "RDNA3",
                AmdArchitecture::CDNA => "CDNA",
                AmdArchitecture::CDNA2 => "CDNA2",
                AmdArchitecture::CDNA3 => "CDNA3",
            },
            Self::Intel(arch) => match arch {
                IntelArchitecture::Gen9 => "Gen9",
                IntelArchitecture::Gen11 => "Gen11",
                IntelArchitecture::Gen12 => "Gen12",
                IntelArchitecture::Xe => "Xe",
                IntelArchitecture::Arc => "Arc",
            },
        }
    }
}
