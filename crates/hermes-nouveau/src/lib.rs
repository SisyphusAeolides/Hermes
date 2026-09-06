//! Independent open-source implementation of Nouveau's NVKM GSP path in Hermes.
//!
//! Source of architectural truth:
//! - Linux `drivers/gpu/drm/nouveau/nvkm/subdev/gsp` (MIT-licensed GSP core)
//! - Regenerated tables: `scripts/reverse-engineer-nouveau.py`
//!
//! Hermes keeps Nouveau's firmware binding knowledge but **raises the Online bar**:
//! missing measured firmware / IOMMU / WPR / mailbox / ready never becomes Online.

#![no_std]

extern crate alloc;

pub mod chip;
pub mod firmware_manifest;
pub mod gsp;
pub mod nvkm;
pub mod rpc;
pub mod superiority;

pub use chip::{ChipFamily, NouveauChip};
pub use firmware_manifest::{
    booter_path, required_roles, NouveauFirmwareNeed, NouveauFirmwareStyle, NouveauFwifEntry,
    NOUVEAU_BOOTER_FIRMWARE, NOUVEAU_FMC_FIRMWARE, NOUVEAU_GSP_FWIF, NOUVEAU_SIG_SECTIONS,
};
pub use gsp::{plan_gsp_load, GspLoadPlan, GspPhase, GspSession};
pub use superiority::{comparison_matrix, hermes_exclusive_count, Capability, HermesEdge};
