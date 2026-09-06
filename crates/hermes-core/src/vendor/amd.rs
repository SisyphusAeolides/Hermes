//! AMD architecture classification.

pub const AMD_VENDOR_ID: u16 = 0x1002;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AmdArchitecture {
    RDNA,
    RDNA2,
    RDNA3,
    CDNA,
    CDNA2,
    CDNA3,
}

pub const fn amd_architecture(device_id: u16) -> Option<AmdArchitecture> {
    // Rough device ID ranges for AMD architectures
    match device_id {
        0x7310..=0x731f | 0x7340..=0x734f => Some(AmdArchitecture::RDNA),
        0x73a0..=0x73df => Some(AmdArchitecture::RDNA2),
        0x7440..=0x749f => Some(AmdArchitecture::RDNA3),
        0x7380..=0x739f => Some(AmdArchitecture::CDNA),
        0x7400..=0x743f => Some(AmdArchitecture::CDNA2),
        0x74a0..=0x74df => Some(AmdArchitecture::CDNA3),
        _ => None,
    }
}
