//! Intel architecture classification.

pub const INTEL_VENDOR_ID: u16 = 0x8086;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntelArchitecture {
    Gen7,
    Gen8,
    Gen9,
    Gen11,
    Gen12,
    Xe,
    Arc,
}

pub const fn intel_architecture(device_id: u16) -> Option<IntelArchitecture> {
    // Device ID ranges for Intel graphics architectures (Gen7.5 Haswell/Crystal Well through Arc)
    match device_id {
        0x0400..=0x04ff | 0x0a00..=0x0aff | 0x0c00..=0x0dff => Some(IntelArchitecture::Gen7),
        0x1600..=0x16ff => Some(IntelArchitecture::Gen8),
        0x3e00..=0x3eff => Some(IntelArchitecture::Gen9),
        0x8a00..=0x8aff => Some(IntelArchitecture::Gen11),
        0x4600..=0x46ff => Some(IntelArchitecture::Gen12),
        0x4f00..=0x4fff => Some(IntelArchitecture::Xe),
        0x5600..=0x56ff => Some(IntelArchitecture::Arc),
        _ => None,
    }
}
