//! Measured NVIDIA GSP-RM firmware admission and bootstrap gates.
//!
//! Firmware bytes are never committed to this repository. Callers stage
//! redistributable images from a matching NVIDIA driver / linux-firmware
//! release; Hermes admits them only after exact length, SHA-256, and GSP ELF
//! structure match.

#![no_std]

extern crate alloc;

pub mod bootstrap;
pub mod bringup;
pub mod elf_gsp;
pub mod firmware;
pub mod host_gate;
pub mod layout;
pub mod mailbox;
pub mod regs;
pub mod session;
pub mod stage;
pub mod wpr;

pub use bootstrap::{
    TuringGspBootstrapMaterial, TuringGspBootstrapRole, VerifiedTuringGspBootstrap,
    T1000_TU117_BOOTSTRAP_610_43_03,
};
pub use bringup::{
    run_bringup, run_bringup_ex, sample_turing_boot_offsets, sample_turing_wpr_framebuffer,
    BringupFault, BringupOutcome, BringupReport, BringupRequest, HardwareEvidence,
    RetainedResources,
};
pub use elf_gsp::{fwversion_bytes, parse_gsp_rm_elf, GspElfEvidence};
pub use firmware::sha256_bytes;
pub use firmware::{
    firmware_family_for_device, firmware_version, FirmwareFamily, NvidiaGspFirmwareAuthority,
    NvidiaGspFirmwareManifest, VerifiedFirmware, NVIDIA_GSP_RM_610_43_02, NVIDIA_GSP_RM_610_43_03,
    NVIDIA_GSP_RM_610_57_04, NVIDIA_GSP_RM_DEFAULT_ALLOW_LIST,
};
pub use host_gate::{
    facts_from_sysfs, host_isolation_ready, host_may_claim_online, host_online_blockers,
    host_preflight_fault, host_preflight_fault_require_map, is_foreign_gpu_driver,
    is_hermes_or_nvidia_driver, HostDeviceFacts, HostGateBlocker,
};
pub use layout::{
    chip_for_architecture, chip_gsp_relative, openrm_gsp_basename, openrm_gsp_relative,
    BootstrapArtifactKind, NvidiaChipDir, TURING_BOOTSTRAP_KINDS,
};
pub use mailbox::{
    boot_handshake, falcon_reset_pulse, rpc_post_u32, snapshot_mailbox, MailboxError,
    MailboxEvidence, MailboxSequence, MailboxSnapshot, RPC_CMD_HELLO, RPC_RSP_ACK,
};
pub use session::{
    default_negotiated_features, drive_full_success, plan_activation, ActivationPlan,
    ActivationStep,
};
pub use stage::{
    stage_gsp_rm_image, stage_matches_admit, StageError, StageReport, STAGE_CHUNK_BYTES,
};
pub use wpr::{
    TuringFramebufferEvidence, TuringGspDmaInputs, TuringMmuLock, TuringRiscvBootOffsets,
    TuringSec2BooterLoad, TuringWprPlan, T1000_FRTS_BYTES, T1000_GSP_BOOT_BINARY_BYTES,
    T1000_WPR_ALIGNMENT, T1000_WPR_HEAP_ALIGNMENT, T1000_WPR_METADATA_BYTES,
};
