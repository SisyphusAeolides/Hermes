//! GSP register map reverse-engineered from MIT-licensed OpenRM published headers.
//! Source note: swref/published/turing/tu102/dev_gsp*.h
//! Clean-room Rust constants (not a verbatim dump of proprietary code).

#![allow(non_upper_case_globals)]
#![allow(dead_code)]

pub const NV_PGSP_FALCON_MAILBOX0: u32 = 0x00110040;
/// Bitfield 31:0
pub const NV_PGSP_FALCON_MAILBOX0_DATA_HI: u32 = 31;
pub const NV_PGSP_FALCON_MAILBOX0_DATA_LO: u32 = 0;
pub const fn nv_pgsp_falcon_mailbox0_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_MAILBOX1: u32 = 0x00110044;
/// Bitfield 31:0
pub const NV_PGSP_FALCON_MAILBOX1_DATA_HI: u32 = 31;
pub const NV_PGSP_FALCON_MAILBOX1_DATA_LO: u32 = 0;
pub const fn nv_pgsp_falcon_mailbox1_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_ENGINE: u32 = 0x001103c0;
/// Bitfield 0:0
pub const NV_PGSP_FALCON_ENGINE_RESET_HI: u32 = 0;
pub const NV_PGSP_FALCON_ENGINE_RESET_LO: u32 = 0;
pub const fn nv_pgsp_falcon_engine_reset_mask() -> u32 {
    if 0 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (0 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_ENGINE_RESET_TRUE: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ENGINE_RESET_FALSE: u32 = 0x00000000;
pub const NV_PGSP_MAILBOX__SIZE_1: u32 = 0x00000004;
/// Bitfield 31:0
pub const NV_PGSP_MAILBOX_DATA_HI: u32 = 31;
pub const NV_PGSP_MAILBOX_DATA_LO: u32 = 0;
pub const fn nv_pgsp_mailbox_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_QUEUE_HEAD__SIZE_1: u32 = 0x00000008;
/// Bitfield 31:0
pub const NV_PGSP_QUEUE_HEAD_ADDRESS_HI: u32 = 31;
pub const NV_PGSP_QUEUE_HEAD_ADDRESS_LO: u32 = 0;
pub const fn nv_pgsp_queue_head_address_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_EMEMC__SIZE_1: u32 = 0x00000004;
/// Bitfield 7:2
pub const NV_PGSP_EMEMC_OFFS_HI: u32 = 7;
pub const NV_PGSP_EMEMC_OFFS_LO: u32 = 2;
pub const fn nv_pgsp_ememc_offs_mask() -> u32 {
    if 7 >= 31 && 2 == 0 { 0xffff_ffff } else { ((1u32 << (7 - 2 + 1)) - 1) << 2 }
}
pub const NV_PGSP_EMEMC_OFFS_INIT: u32 = 0x00000000;
/// Bitfield 15:8
pub const NV_PGSP_EMEMC_BLK_HI: u32 = 15;
pub const NV_PGSP_EMEMC_BLK_LO: u32 = 8;
pub const fn nv_pgsp_ememc_blk_mask() -> u32 {
    if 15 >= 31 && 8 == 0 { 0xffff_ffff } else { ((1u32 << (15 - 8 + 1)) - 1) << 8 }
}
pub const NV_PGSP_EMEMC_BLK_INIT: u32 = 0x00000000;
/// Bitfield 24:24
pub const NV_PGSP_EMEMC_AINCW_HI: u32 = 24;
pub const NV_PGSP_EMEMC_AINCW_LO: u32 = 24;
pub const fn nv_pgsp_ememc_aincw_mask() -> u32 {
    if 24 >= 31 && 24 == 0 { 0xffff_ffff } else { ((1u32 << (24 - 24 + 1)) - 1) << 24 }
}
pub const NV_PGSP_EMEMC_AINCW_INIT: u32 = 0x00000000;
pub const NV_PGSP_EMEMC_AINCW_TRUE: u32 = 0x00000001;
pub const NV_PGSP_EMEMC_AINCW_FALSE: u32 = 0x00000000;
/// Bitfield 25:25
pub const NV_PGSP_EMEMC_AINCR_HI: u32 = 25;
pub const NV_PGSP_EMEMC_AINCR_LO: u32 = 25;
pub const fn nv_pgsp_ememc_aincr_mask() -> u32 {
    if 25 >= 31 && 25 == 0 { 0xffff_ffff } else { ((1u32 << (25 - 25 + 1)) - 1) << 25 }
}
pub const NV_PGSP_EMEMC_AINCR_INIT: u32 = 0x00000000;
pub const NV_PGSP_EMEMC_AINCR_TRUE: u32 = 0x00000001;
pub const NV_PGSP_EMEMC_AINCR_FALSE: u32 = 0x00000000;
pub const NV_PGSP_EMEMD__SIZE_1: u32 = 0x00000004;
/// Bitfield 31:0
pub const NV_PGSP_EMEMD_DATA_HI: u32 = 31;
pub const NV_PGSP_EMEMD_DATA_LO: u32 = 0;
pub const fn nv_pgsp_ememd_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FBIF_BASE: u32 = 0x00110600;
