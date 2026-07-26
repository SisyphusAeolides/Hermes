//! Structural admission of NVIDIA GSP-RM ELF images (RISC-V relocatable).
//!
//! Reverse-engineered from linux-firmware / OpenRM `gsp_*.bin`:
//! ELF64 + EM_RISCV + `.fwimage` + `.fwversion` [+ optional `.fwsignature_*`].

use hermes_core::HermesFault;

const ELFMAG: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_REL: u16 = 1;
const EM_RISCV: u16 = 243;
const SHT_PROGBITS: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GspElfEvidence {
    pub fwimage_offset: u64,
    pub fwimage_size: u64,
    pub fwversion_offset: u64,
    pub fwversion_size: u16,
    pub signature_sections: u8,
    pub machine: u16,
}

/// Parse a GSP-RM blob and require the structural contract of a redistributable
/// NVIDIA GSP ELF. Does not prove cryptographic authenticity.
pub fn parse_gsp_rm_elf(image: &[u8]) -> Result<GspElfEvidence, HermesFault> {
    if image.len() < 64 {
        return Err(HermesFault::FirmwareRejected);
    }
    if image[0..4] != ELFMAG {
        return Err(HermesFault::FirmwareRejected);
    }
    if image[4] != ELFCLASS64 || image[5] != ELFDATA2LSB {
        return Err(HermesFault::FirmwareRejected);
    }

    let e_type = u16::from_le_bytes([image[16], image[17]]);
    let e_machine = u16::from_le_bytes([image[18], image[19]]);
    if e_type != ET_REL || e_machine != EM_RISCV {
        return Err(HermesFault::FirmwareRejected);
    }

    let e_shoff = u64::from_le_bytes(image[40..48].try_into().unwrap());
    let e_shentsize = u16::from_le_bytes([image[58], image[59]]) as u64;
    let e_shnum = u16::from_le_bytes([image[60], image[61]]) as u64;
    let e_shstrndx = u16::from_le_bytes([image[62], image[63]]) as u64;

    if e_shentsize < 64 || e_shnum == 0 || e_shstrndx >= e_shnum {
        return Err(HermesFault::FirmwareRejected);
    }
    let table_end = e_shoff
        .checked_add(e_shnum.checked_mul(e_shentsize).ok_or(HermesFault::FirmwareRejected)?)
        .ok_or(HermesFault::FirmwareRejected)?;
    if table_end > image.len() as u64 {
        return Err(HermesFault::FirmwareRejected);
    }

    let str_hdr = section_header(image, e_shoff, e_shentsize, e_shstrndx)?;
    let str_off = str_hdr.sh_offset as usize;
    let str_end = str_off
        .checked_add(str_hdr.sh_size as usize)
        .ok_or(HermesFault::FirmwareRejected)?;
    if str_end > image.len() {
        return Err(HermesFault::FirmwareRejected);
    }
    let strtab = &image[str_off..str_end];

    let mut fwimage_offset = 0u64;
    let mut fwimage_size = 0u64;
    let mut fwversion_offset = 0u64;
    let mut fwversion_size = 0u16;
    let mut signature_sections = 0u8;

    for i in 0..e_shnum {
        let hdr = section_header(image, e_shoff, e_shentsize, i)?;
        let name = section_name(strtab, hdr.sh_name)?;
        if name == b".fwimage" {
            if hdr.sh_type != SHT_PROGBITS || hdr.sh_size == 0 {
                return Err(HermesFault::FirmwareRejected);
            }
            fwimage_offset = hdr.sh_offset;
            fwimage_size = hdr.sh_size;
        } else if name == b".fwversion" {
            if hdr.sh_size == 0 || hdr.sh_size > 32 {
                return Err(HermesFault::FirmwareRejected);
            }
            fwversion_offset = hdr.sh_offset;
            fwversion_size = hdr.sh_size as u16;
        } else if name.starts_with(b".fwsignature") {
            signature_sections = signature_sections.saturating_add(1);
        }
    }

    if fwimage_size == 0 || fwversion_size == 0 {
        return Err(HermesFault::FirmwareRejected);
    }
    let ver_end = (fwversion_offset as usize)
        .checked_add(fwversion_size as usize)
        .ok_or(HermesFault::FirmwareRejected)?;
    if ver_end > image.len() {
        return Err(HermesFault::FirmwareRejected);
    }

    Ok(GspElfEvidence {
        fwimage_offset,
        fwimage_size,
        fwversion_offset,
        fwversion_size,
        signature_sections,
        machine: e_machine,
    })
}

/// Extract the ASCII version string from `.fwversion` (NUL-trimmed).
pub fn fwversion_bytes<'a>(image: &'a [u8], evidence: &GspElfEvidence) -> Result<&'a [u8], HermesFault> {
    let start = evidence.fwversion_offset as usize;
    let end = start
        .checked_add(evidence.fwversion_size as usize)
        .ok_or(HermesFault::FirmwareRejected)?;
    if end > image.len() {
        return Err(HermesFault::FirmwareRejected);
    }
    let raw = &image[start..end];
    Ok(raw.split(|&b| b == 0).next().unwrap_or(raw))
}

struct SectionHeader {
    sh_name: u32,
    sh_type: u32,
    sh_offset: u64,
    sh_size: u64,
}

fn section_header(
    image: &[u8],
    shoff: u64,
    shentsize: u64,
    index: u64,
) -> Result<SectionHeader, HermesFault> {
    let off = (shoff as usize)
        .checked_add(
            (index as usize)
                .checked_mul(shentsize as usize)
                .ok_or(HermesFault::FirmwareRejected)?,
        )
        .ok_or(HermesFault::FirmwareRejected)?;
    if off + 64 > image.len() {
        return Err(HermesFault::FirmwareRejected);
    }
    let sh_name = u32::from_le_bytes(image[off..off + 4].try_into().unwrap());
    let sh_type = u32::from_le_bytes(image[off + 4..off + 8].try_into().unwrap());
    let sh_offset = u64::from_le_bytes(image[off + 24..off + 32].try_into().unwrap());
    let sh_size = u64::from_le_bytes(image[off + 32..off + 40].try_into().unwrap());
    Ok(SectionHeader {
        sh_name,
        sh_type,
        sh_offset,
        sh_size,
    })
}

fn section_name(strtab: &[u8], name_off: u32) -> Result<&[u8], HermesFault> {
    let start = name_off as usize;
    if start >= strtab.len() {
        return Err(HermesFault::FirmwareRejected);
    }
    let end = strtab[start..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| start + p)
        .unwrap_or(strtab.len());
    Ok(&strtab[start..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_bad() -> [u8; 64] {
        let mut b = [0u8; 64];
        b[0..4].copy_from_slice(&ELFMAG);
        b[4] = ELFCLASS64;
        b[5] = ELFDATA2LSB;
        b
    }

    #[test]
    fn rejects_non_elf() {
        assert!(parse_gsp_rm_elf(b"not-an-elf").is_err());
    }

    #[test]
    fn rejects_empty_elf_header_without_sections() {
        let b = minimal_bad();
        assert!(parse_gsp_rm_elf(&b).is_err());
    }
}
