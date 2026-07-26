//! Measured NVIDIA GSP-RM firmware admission and bootstrap gates.
//!
//! Firmware bytes are never committed to this repository. Callers stage
//! redistributable images from a matching NVIDIA driver release; Hermes
//! admits them only after exact length and SHA-256 match a pinned manifest.

#![no_std]

extern crate alloc;

pub mod bootstrap;
pub mod bringup;
pub mod firmware;
pub mod session;

pub use bootstrap::{
    TuringGspBootstrapMaterial, TuringGspBootstrapRole, VerifiedTuringGspBootstrap,
    T1000_TU117_BOOTSTRAP_610_43_03,
};
pub use bringup::{
    BringupFault, BringupReport, BringupRequest, HardwareEvidence, run_bringup,
};
pub use firmware::{
    firmware_family_for_device, firmware_version, FirmwareFamily, NvidiaGspFirmwareAuthority,
    NvidiaGspFirmwareManifest, VerifiedFirmware, NVIDIA_GSP_RM_610_43_03,
};
pub use firmware::sha256_bytes;
pub use session::{
    default_negotiated_features, drive_full_success, plan_activation, ActivationPlan,
    ActivationStep,
};
