//! Intel architecture classification.

pub const INTEL_VENDOR_ID: u16 = 0x8086;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntelArchitecture {
    Gen9,
    Gen11,
    Gen12,
    Xe,
    Arc,
}

pub const fn intel_architecture(device_id: u16) -> Option<IntelArchitecture> {
    // Rough device ID ranges for Intel architectures
    match device_id {
        0x3e00..=0x3eff => Some(IntelArchitecture::Gen9),
        0x8a00..=0x8aff => Some(IntelArchitecture::Gen11),
        0x4600..=0x46ff => Some(IntelArchitecture::Gen12),
        0x4f00..=0x4fff => Some(IntelArchitecture::Xe),
        0x5600..=0x56ff => Some(IntelArchitecture::Arc),
        _ => None,
    }
}
