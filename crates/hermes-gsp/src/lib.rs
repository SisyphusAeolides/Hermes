//! Measured NVIDIA GSP-RM firmware admission and bootstrap gates.
//!
//! Firmware bytes are never committed to this repository. Callers stage
//! redistributable images from a matching NVIDIA driver release; Hermes
//! admits them only after exact length and SHA-256 match a pinned manifest.

#![no_std]

extern crate alloc;

pub mod bootstrap;
pub mod firmware;
pub mod session;

pub use bootstrap::{
    TuringGspBootstrapMaterial, TuringGspBootstrapRole, VerifiedTuringGspBootstrap,
    T1000_TU117_BOOTSTRAP_610_43_03,
};
pub use firmware::{
    FirmwareFamily, NvidiaGspFirmwareAuthority, NvidiaGspFirmwareManifest, VerifiedFirmware,
    firmware_family_for_device, firmware_version, NVIDIA_GSP_RM_610_43_03,
};
pub use session::{
    ActivationPlan, ActivationStep, default_negotiated_features, drive_full_success,
    plan_activation,
};
pub use firmware::sha256_bytes;
