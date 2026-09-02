//! GSP register map reverse-engineered from MIT-licensed OpenRM published headers.
//! Source note: blackwell/gb100/dev_gsp.h
//! Clean-room Rust constants (not a verbatim dump of proprietary code).

#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::eq_op, clippy::identity_op)]

pub const NV_PGSP_FALCON_MAILBOX0: u32 = 0x00110040;
/// Bitfield 31:0
pub const NV_PGSP_FALCON_MAILBOX0_DATA_HI: u32 = 31;
pub const NV_PGSP_FALCON_MAILBOX0_DATA_LO: u32 = 0;
pub const fn nv_pgsp_falcon_mailbox0_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_MAILBOX0_DATA_INIT: u32 = 0x00000000;
pub const NV_PGSP_FALCON_MAILBOX1: u32 = 0x00110044;
/// Bitfield 31:0
pub const NV_PGSP_FALCON_MAILBOX1_DATA_HI: u32 = 31;
pub const NV_PGSP_FALCON_MAILBOX1_DATA_LO: u32 = 0;
pub const fn nv_pgsp_falcon_mailbox1_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_MAILBOX1_DATA_INIT: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ENGINE: u32 = 0x001103c0;
/// Bitfield 0:0
pub const NV_PGSP_FALCON_ENGINE_RESET_HI: u32 = 0;
pub const NV_PGSP_FALCON_ENGINE_RESET_LO: u32 = 0;
pub const fn nv_pgsp_falcon_engine_reset_mask() -> u32 {
    if 0 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (0 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_ENGINE_RESET_DEASSERT: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ENGINE_RESET_ASSERT: u32 = 0x00000001;
/// Bitfield 10:8
pub const NV_PGSP_FALCON_ENGINE_RESET_STATUS_HI: u32 = 10;
pub const NV_PGSP_FALCON_ENGINE_RESET_STATUS_LO: u32 = 8;
pub const fn nv_pgsp_falcon_engine_reset_status_mask() -> u32 {
    if 10 >= 31 && 8 == 0 { 0xffff_ffff } else { ((1u32 << (10 - 8 + 1)) - 1) << 8 }
}
pub const NV_PGSP_FALCON_ENGINE_RESET_STATUS_ASSERTED: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ENGINE_RESET_STATUS_DEASSERTED: u32 = 0x00000002;
pub const NV_PGSP_EMEMC__SIZE_1: u32 = 0x00000008;
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
pub const NV_PGSP_EMEMD__SIZE_1: u32 = 0x00000008;
/// Bitfield 31:0
pub const NV_PGSP_EMEMD_DATA_HI: u32 = 31;
pub const NV_PGSP_EMEMD_DATA_LO: u32 = 0;
pub const fn nv_pgsp_ememd_data_mask() -> u32 {
    if 31 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK: u32 = 0x001103c4;
/// Bitfield 0:0
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_READ_PROTECTION_LEVEL0_HI: u32 = 0;
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_READ_PROTECTION_LEVEL0_LO: u32 = 0;
pub const fn nv_pgsp_falcon_reset_priv_level_mask_read_protection_level0_mask() -> u32 {
    if 0 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (0 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_READ_PROTECTION_LEVEL0_ENABLE: u32 = 0x00000001;
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_READ_PROTECTION_LEVEL0_DISABLE: u32 = 0x00000000;
/// Bitfield 4:4
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_WRITE_PROTECTION_LEVEL0_HI: u32 = 4;
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_WRITE_PROTECTION_LEVEL0_LO: u32 = 4;
pub const fn nv_pgsp_falcon_reset_priv_level_mask_write_protection_level0_mask() -> u32 {
    if 4 >= 31 && 4 == 0 { 0xffff_ffff } else { ((1u32 << (4 - 4 + 1)) - 1) << 4 }
}
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_WRITE_PROTECTION_LEVEL0_ENABLE: u32 = 0x00000001;
pub const NV_PGSP_FALCON_RESET_PRIV_LEVEL_MASK_WRITE_PROTECTION_LEVEL0_DISABLE: u32 = 0x00000000;
pub const NV_PGSP_FALCON_IRQSTAT: u32 = 0x00110008;
/// Bitfield 24:24
pub const NV_PGSP_FALCON_IRQSTAT_FATAL_ERROR_HI: u32 = 24;
pub const NV_PGSP_FALCON_IRQSTAT_FATAL_ERROR_LO: u32 = 24;
pub const fn nv_pgsp_falcon_irqstat_fatal_error_mask() -> u32 {
    if 24 >= 31 && 24 == 0 { 0xffff_ffff } else { ((1u32 << (24 - 24 + 1)) - 1) << 24 }
}
pub const NV_PGSP_FALCON_IRQSTAT_FATAL_ERROR_TRUE: u32 = 0x00000001;
pub const NV_PGSP_FALCON_IRQSTAT_FATAL_ERROR_FALSE: u32 = 0x00000000;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT: u32 = 0x00111700;
/// Bitfield 0:0
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_GLOBAL_MEM_HI: u32 = 0;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_GLOBAL_MEM_LO: u32 = 0;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_global_mem_mask() -> u32 {
    if 0 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (0 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_GLOBAL_MEM_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_GLOBAL_MEM_NO_FAULT: u32 = 0x00000000;
/// Bitfield 1:1
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ROM_HI: u32 = 1;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ROM_LO: u32 = 1;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_rom_mask() -> u32 {
    if 1 >= 31 && 1 == 0 { 0xffff_ffff } else { ((1u32 << (1 - 1 + 1)) - 1) << 1 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ROM_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ROM_NO_FAULT: u32 = 0x00000000;
/// Bitfield 2:2
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ITCM_HI: u32 = 2;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ITCM_LO: u32 = 2;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_itcm_mask() -> u32 {
    if 2 >= 31 && 2 == 0 { 0xffff_ffff } else { ((1u32 << (2 - 2 + 1)) - 1) << 2 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ITCM_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ITCM_NO_FAULT: u32 = 0x00000000;
/// Bitfield 3:3
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DTCM_HI: u32 = 3;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DTCM_LO: u32 = 3;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_dtcm_mask() -> u32 {
    if 3 >= 31 && 3 == 0 { 0xffff_ffff } else { ((1u32 << (3 - 3 + 1)) - 1) << 3 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DTCM_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DTCM_NO_FAULT: u32 = 0x00000000;
/// Bitfield 4:4
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ICACHE_HI: u32 = 4;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ICACHE_LO: u32 = 4;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_icache_mask() -> u32 {
    if 4 >= 31 && 4 == 0 { 0xffff_ffff } else { ((1u32 << (4 - 4 + 1)) - 1) << 4 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ICACHE_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_ICACHE_NO_FAULT: u32 = 0x00000000;
/// Bitfield 5:5
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DCACHE_HI: u32 = 5;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DCACHE_LO: u32 = 5;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_dcache_mask() -> u32 {
    if 5 >= 31 && 5 == 0 { 0xffff_ffff } else { ((1u32 << (5 - 5 + 1)) - 1) << 5 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DCACHE_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_DCACHE_NO_FAULT: u32 = 0x00000000;
/// Bitfield 6:6
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_RVCORE_HI: u32 = 6;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_RVCORE_LO: u32 = 6;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_rvcore_mask() -> u32 {
    if 6 >= 31 && 6 == 0 { 0xffff_ffff } else { ((1u32 << (6 - 6 + 1)) - 1) << 6 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_RVCORE_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_RVCORE_NO_FAULT: u32 = 0x00000000;
/// Bitfield 7:7
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_REG_HI: u32 = 7;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_REG_LO: u32 = 7;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_reg_mask() -> u32 {
    if 7 >= 31 && 7 == 0 { 0xffff_ffff } else { ((1u32 << (7 - 7 + 1)) - 1) << 7 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_REG_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_REG_NO_FAULT: u32 = 0x00000000;
/// Bitfield 8:8
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_LOGIC_HI: u32 = 8;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_LOGIC_LO: u32 = 8;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_se_logic_mask() -> u32 {
    if 8 >= 31 && 8 == 0 { 0xffff_ffff } else { ((1u32 << (8 - 8 + 1)) - 1) << 8 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_LOGIC_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_LOGIC_NO_FAULT: u32 = 0x00000000;
/// Bitfield 9:9
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_KSLT_HI: u32 = 9;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_KSLT_LO: u32 = 9;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_se_kslt_mask() -> u32 {
    if 9 >= 31 && 9 == 0 { 0xffff_ffff } else { ((1u32 << (9 - 9 + 1)) - 1) << 9 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_KSLT_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_SE_KSLT_NO_FAULT: u32 = 0x00000000;
/// Bitfield 10:10
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_TKE_HI: u32 = 10;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_TKE_LO: u32 = 10;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_tke_mask() -> u32 {
    if 10 >= 31 && 10 == 0 { 0xffff_ffff } else { ((1u32 << (10 - 10 + 1)) - 1) << 10 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_TKE_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_TKE_NO_FAULT: u32 = 0x00000000;
/// Bitfield 11:11
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_FBIF_HI: u32 = 11;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_FBIF_LO: u32 = 11;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_fbif_mask() -> u32 {
    if 11 >= 31 && 11 == 0 { 0xffff_ffff } else { ((1u32 << (11 - 11 + 1)) - 1) << 11 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_FBIF_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_FBIF_NO_FAULT: u32 = 0x00000000;
/// Bitfield 12:12
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_MPURAM_HI: u32 = 12;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_MPURAM_LO: u32 = 12;
pub const fn nv_pgsp_riscv_fault_containment_srcstat_mpuram_mask() -> u32 {
    if 12 >= 31 && 12 == 0 { 0xffff_ffff } else { ((1u32 << (12 - 12 + 1)) - 1) << 12 }
}
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_MPURAM_FAULTED: u32 = 0x00000001;
pub const NV_PGSP_RISCV_FAULT_CONTAINMENT_SRCSTAT_MPURAM_NO_FAULT: u32 = 0x00000000;
pub const NV_PGSP_ECC_INTR_STATUS: u32 = 0x00110888;
/// Bitfield 0:0
pub const NV_PGSP_ECC_INTR_STATUS_CORRECTED_HI: u32 = 0;
pub const NV_PGSP_ECC_INTR_STATUS_CORRECTED_LO: u32 = 0;
pub const fn nv_pgsp_ecc_intr_status_corrected_mask() -> u32 {
    if 0 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (0 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_ECC_INTR_STATUS_CORRECTED_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_ECC_INTR_STATUS_CORRECTED_PENDING: u32 = 0x00000001;
/// Bitfield 1:1
pub const NV_PGSP_ECC_INTR_STATUS_UNCORRECTED_HI: u32 = 1;
pub const NV_PGSP_ECC_INTR_STATUS_UNCORRECTED_LO: u32 = 1;
pub const fn nv_pgsp_ecc_intr_status_uncorrected_mask() -> u32 {
    if 1 >= 31 && 1 == 0 { 0xffff_ffff } else { ((1u32 << (1 - 1 + 1)) - 1) << 1 }
}
pub const NV_PGSP_ECC_INTR_STATUS_UNCORRECTED_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_ECC_INTR_STATUS_UNCORRECTED_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS: u32 = 0x00110878;
/// Bitfield 8:8
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_IMEM_HI: u32 = 8;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_IMEM_LO: u32 = 8;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_imem_mask() -> u32 {
    if 8 >= 31 && 8 == 0 { 0xffff_ffff } else { ((1u32 << (8 - 8 + 1)) - 1) << 8 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_IMEM_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_IMEM_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_IMEM_CLEAR: u32 = 0x00000001;
/// Bitfield 9:9
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DMEM_HI: u32 = 9;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DMEM_LO: u32 = 9;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_dmem_mask() -> u32 {
    if 9 >= 31 && 9 == 0 { 0xffff_ffff } else { ((1u32 << (9 - 9 + 1)) - 1) << 9 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DMEM_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DMEM_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DMEM_CLEAR: u32 = 0x00000001;
/// Bitfield 10:10
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_MPU_RAM_HI: u32 = 10;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_MPU_RAM_LO: u32 = 10;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_mpu_ram_mask() -> u32 {
    if 10 >= 31 && 10 == 0 { 0xffff_ffff } else { ((1u32 << (10 - 10 + 1)) - 1) << 10 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_MPU_RAM_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_MPU_RAM_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_MPU_RAM_CLEAR: u32 = 0x00000001;
/// Bitfield 11:11
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCLS_HI: u32 = 11;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCLS_LO: u32 = 11;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_dcls_mask() -> u32 {
    if 11 >= 31 && 11 == 0 { 0xffff_ffff } else { ((1u32 << (11 - 11 + 1)) - 1) << 11 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCLS_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCLS_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCLS_CLEAR: u32 = 0x00000001;
/// Bitfield 12:12
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_REG_HI: u32 = 12;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_REG_LO: u32 = 12;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_reg_mask() -> u32 {
    if 12 >= 31 && 12 == 0 { 0xffff_ffff } else { ((1u32 << (12 - 12 + 1)) - 1) << 12 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_REG_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_REG_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_REG_CLEAR: u32 = 0x00000001;
/// Bitfield 13:13
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_EMEM_HI: u32 = 13;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_EMEM_LO: u32 = 13;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_emem_mask() -> u32 {
    if 13 >= 31 && 13 == 0 { 0xffff_ffff } else { ((1u32 << (13 - 13 + 1)) - 1) << 13 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_EMEM_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_EMEM_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_EMEM_CLEAR: u32 = 0x00000001;
/// Bitfield 14:14
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_ICACHE_HI: u32 = 14;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_ICACHE_LO: u32 = 14;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_icache_mask() -> u32 {
    if 14 >= 31 && 14 == 0 { 0xffff_ffff } else { ((1u32 << (14 - 14 + 1)) - 1) << 14 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_ICACHE_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_ICACHE_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_ICACHE_CLEAR: u32 = 0x00000001;
/// Bitfield 15:15
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCACHE_HI: u32 = 15;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCACHE_LO: u32 = 15;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_dcache_mask() -> u32 {
    if 15 >= 31 && 15 == 0 { 0xffff_ffff } else { ((1u32 << (15 - 15 + 1)) - 1) << 15 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCACHE_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCACHE_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_DCACHE_CLEAR: u32 = 0x00000001;
/// Bitfield 16:16
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_HI: u32 = 16;
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_LO: u32 = 16;
pub const fn nv_pgsp_falcon_ecc_status_corrected_err_total_counter_overflow_mask() -> u32 {
    if 16 >= 31 && 16 == 0 { 0xffff_ffff } else { ((1u32 << (16 - 16 + 1)) - 1) << 16 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_CLEAR: u32 = 0x00000001;
/// Bitfield 17:17
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_HI: u32 = 17;
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_LO: u32 = 17;
pub const fn nv_pgsp_falcon_ecc_status_corrected_err_unique_counter_overflow_mask() -> u32 {
    if 17 >= 31 && 17 == 0 { 0xffff_ffff } else { ((1u32 << (17 - 17 + 1)) - 1) << 17 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_CORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_CLEAR: u32 = 0x00000001;
/// Bitfield 18:18
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_HI: u32 = 18;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_LO: u32 = 18;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_total_counter_overflow_mask() -> u32 {
    if 18 >= 31 && 18 == 0 { 0xffff_ffff } else { ((1u32 << (18 - 18 + 1)) - 1) << 18 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_TOTAL_COUNTER_OVERFLOW_CLEAR: u32 = 0x00000001;
/// Bitfield 19:19
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_HI: u32 = 19;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_LO: u32 = 19;
pub const fn nv_pgsp_falcon_ecc_status_uncorrected_err_unique_counter_overflow_mask() -> u32 {
    if 19 >= 31 && 19 == 0 { 0xffff_ffff } else { ((1u32 << (19 - 19 + 1)) - 1) << 19 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_NOT_PENDING: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_PENDING: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_UNCORRECTED_ERR_UNIQUE_COUNTER_OVERFLOW_CLEAR: u32 = 0x00000001;
/// Bitfield 31:31
pub const NV_PGSP_FALCON_ECC_STATUS_RESET_HI: u32 = 31;
pub const NV_PGSP_FALCON_ECC_STATUS_RESET_LO: u32 = 31;
pub const fn nv_pgsp_falcon_ecc_status_reset_mask() -> u32 {
    if 31 >= 31 && 31 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 31 + 1)) - 1) << 31 }
}
pub const NV_PGSP_FALCON_ECC_STATUS_RESET_TASK: u32 = 0x00000001;
pub const NV_PGSP_FALCON_ECC_STATUS_RESET_INIT: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT: u32 = 0x00110880;
/// Bitfield 15:0
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT_TOTAL_HI: u32 = 15;
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT_TOTAL_LO: u32 = 0;
pub const fn nv_pgsp_falcon_ecc_corrected_err_count_total_mask() -> u32 {
    if 15 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (15 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT_TOTAL_INIT: u32 = 0x00000000;
/// Bitfield 31:16
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT_UNIQUE_HI: u32 = 31;
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT_UNIQUE_LO: u32 = 16;
pub const fn nv_pgsp_falcon_ecc_corrected_err_count_unique_mask() -> u32 {
    if 31 >= 31 && 16 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 16 + 1)) - 1) << 16 }
}
pub const NV_PGSP_FALCON_ECC_CORRECTED_ERR_COUNT_UNIQUE_INIT: u32 = 0x00000000;
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT: u32 = 0x00110884;
/// Bitfield 15:0
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT_TOTAL_HI: u32 = 15;
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT_TOTAL_LO: u32 = 0;
pub const fn nv_pgsp_falcon_ecc_uncorrected_err_count_total_mask() -> u32 {
    if 15 >= 31 && 0 == 0 { 0xffff_ffff } else { ((1u32 << (15 - 0 + 1)) - 1) << 0 }
}
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT_TOTAL_INIT: u32 = 0x00000000;
/// Bitfield 31:16
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT_UNIQUE_HI: u32 = 31;
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT_UNIQUE_LO: u32 = 16;
pub const fn nv_pgsp_falcon_ecc_uncorrected_err_count_unique_mask() -> u32 {
    if 31 >= 31 && 16 == 0 { 0xffff_ffff } else { ((1u32 << (31 - 16 + 1)) - 1) << 16 }
}
pub const NV_PGSP_FALCON_ECC_UNCORRECTED_ERR_COUNT_UNIQUE_INIT: u32 = 0x00000000;
