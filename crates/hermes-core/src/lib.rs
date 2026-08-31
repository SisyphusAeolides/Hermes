//! Kernel-portable Hermes core: device family admission, phase manifold, HAL.
//!
//! Mirrors the Idris phase lattice and Agda feature rules in executable form.
//! No path reaches Online without every required evidence token.

#![no_std]

pub mod admission;
pub mod vendor;
pub mod chaos;
pub mod manifold;
pub mod platform;
pub mod ring;

pub use admission::{
    AdmissionError, AdmittedDevice, admit_display_device, admit_gpu_device, pci_identity,
};
pub use vendor::*;
pub use manifold::{
    HermesEvidence, HermesManifold, HermesPhase, ManifoldFault, OnlineCertificate, feature,
};
pub use platform::{
    DmaPurpose, DmaRegion, HermesCodec, HermesFault, HermesPlatform, MmioWindow,
};
