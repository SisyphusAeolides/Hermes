//! Kernel-portable Hermes core: device family admission, phase manifold, HAL.
//!
//! Mirrors the Idris phase lattice and Agda feature rules in executable form.
//! No path reaches Online without every required evidence token.

#![no_std]

pub mod admission;
pub mod family;
pub mod manifold;
pub mod platform;

pub use admission::{
    AdmissionError, AdmittedDevice, admit_display_device, admit_nvidia_device, pci_identity,
};
pub use family::{
    NVIDIA_ARCHITECTURE_ADA, NVIDIA_ARCHITECTURE_AMPERE, NVIDIA_ARCHITECTURE_BLACKWELL,
    NVIDIA_ARCHITECTURE_HOPPER, NVIDIA_ARCHITECTURE_TURING, NVIDIA_ARCHITECTURE_TURING_OR_NEWER,
    NVIDIA_VENDOR_ID, NvidiaArchitecture, is_nvidia_turing_or_newer, nvidia_architecture,
    nvidia_architecture_hint,
};
pub use manifold::{
    HermesEvidence, HermesManifold, HermesPhase, ManifoldFault, OnlineCertificate, feature,
};
pub use platform::{
    DmaPurpose, DmaRegion, HermesCodec, HermesFault, HermesPlatform, MmioWindow,
};
