//! Turing-and-newer NVIDIA architecture classification.
//!
//! Device-ID bands are coarse admission gates aligned with the open GPU kernel
//! modules GSP-required scope (Turing+). Unknown IDs never inherit support
//! merely because the vendor is NVIDIA. Pre-Turing (Maxwell/Pascal/Volta)
//! always rejects.

/// PCI vendor ID for NVIDIA Corporation.
pub const NVIDIA_VENDOR_ID: u16 = 0x10de;

/// Present on every recognized Turing-or-newer device.
pub const NVIDIA_ARCHITECTURE_TURING_OR_NEWER: u32 = 1 << 0;
pub const NVIDIA_ARCHITECTURE_TURING: u32 = 1 << 8;
pub const NVIDIA_ARCHITECTURE_AMPERE: u32 = 1 << 9;
pub const NVIDIA_ARCHITECTURE_HOPPER: u32 = 1 << 10;
pub const NVIDIA_ARCHITECTURE_ADA: u32 = 1 << 11;
pub const NVIDIA_ARCHITECTURE_BLACKWELL: u32 = 1 << 12;

/// Named architecture family for GSP firmware line selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum NvidiaArchitecture {
    Turing = 1,
    Ampere = 2,
    Hopper = 3,
    Ada = 4,
    Blackwell = 5,
}

impl NvidiaArchitecture {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Turing => "Turing",
            Self::Ampere => "Ampere",
            Self::Hopper => "Hopper",
            Self::Ada => "Ada",
            Self::Blackwell => "Blackwell",
        }
    }

    pub const fn hint_bits(self) -> u32 {
        match self {
            Self::Turing => NVIDIA_ARCHITECTURE_TURING_OR_NEWER | NVIDIA_ARCHITECTURE_TURING,
            Self::Ampere => NVIDIA_ARCHITECTURE_TURING_OR_NEWER | NVIDIA_ARCHITECTURE_AMPERE,
            Self::Hopper => NVIDIA_ARCHITECTURE_TURING_OR_NEWER | NVIDIA_ARCHITECTURE_HOPPER,
            Self::Ada => NVIDIA_ARCHITECTURE_TURING_OR_NEWER | NVIDIA_ARCHITECTURE_ADA,
            Self::Blackwell => NVIDIA_ARCHITECTURE_TURING_OR_NEWER | NVIDIA_ARCHITECTURE_BLACKWELL,
        }
    }
}

/// Classifies only PCI device-ID bands whose NVIDIA architecture is known.
///
/// Bands follow open-gpu-kernel-modules / Nouveau GSP-required generations:
/// - Turing:  0x1E00–0x1FFF, 0x2180–0x21FF
/// - Ampere:  0x2000–0x20FF, 0x2200–0x22FF, 0x2400–0x25FF
/// - Hopper:  0x2300–0x23FF
/// - Ada:     0x2600–0x28FF
/// - Blackwell: 0x2900–0x2FFF
pub const fn nvidia_architecture(device_id: u16) -> Option<NvidiaArchitecture> {
    match device_id {
        0x1e00..=0x1fff | 0x2180..=0x21ff => Some(NvidiaArchitecture::Turing),
        0x2000..=0x20ff | 0x2200..=0x22ff | 0x2400..=0x25ff => Some(NvidiaArchitecture::Ampere),
        0x2300..=0x23ff => Some(NvidiaArchitecture::Hopper),
        0x2600..=0x28ff => Some(NvidiaArchitecture::Ada),
        0x2900..=0x2fff => Some(NvidiaArchitecture::Blackwell),
        _ => None,
    }
}

pub const fn nvidia_architecture_hint(device_id: u16) -> u32 {
    match nvidia_architecture(device_id) {
        Some(arch) => arch.hint_bits(),
        None => 0,
    }
}

pub const fn is_nvidia_turing_or_newer(device_id: u16) -> bool {
    nvidia_architecture(device_id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_turing_and_later_rejects_pre_turing() {
        // Volta GV100 — pre-Turing, no GSP-RM open path
        assert!(!is_nvidia_turing_or_newer(0x1db6));
        assert!(nvidia_architecture(0x1db6).is_none());

        // Pascal / Maxwell-class samples
        assert!(!is_nvidia_turing_or_newer(0x1b80)); // GP104
        assert!(!is_nvidia_turing_or_newer(0x13c0)); // GM204

        // Turing
        assert_eq!(
            nvidia_architecture(0x1e04),
            Some(NvidiaArchitecture::Turing)
        ); // TU102
        assert_eq!(
            nvidia_architecture(0x1fb9),
            Some(NvidiaArchitecture::Turing)
        ); // TU117GLM T1000
        assert_eq!(
            nvidia_architecture(0x2182),
            Some(NvidiaArchitecture::Turing)
        ); // TU116

        // Ampere
        assert_eq!(
            nvidia_architecture(0x2204),
            Some(NvidiaArchitecture::Ampere)
        ); // GA102
        assert_eq!(
            nvidia_architecture(0x2484),
            Some(NvidiaArchitecture::Ampere)
        );

        // Hopper
        assert_eq!(
            nvidia_architecture(0x2330),
            Some(NvidiaArchitecture::Hopper)
        );

        // Ada
        assert_eq!(nvidia_architecture(0x2684), Some(NvidiaArchitecture::Ada)); // AD102

        // Blackwell
        assert_eq!(
            nvidia_architecture(0x2b85),
            Some(NvidiaArchitecture::Blackwell)
        );

        // Unknown future / non-GPU
        assert!(!is_nvidia_turing_or_newer(0x4000));
        assert!(!is_nvidia_turing_or_newer(0x0000));
    }

    #[test]
    fn turing_or_newer_bit_set_on_all_supported_families() {
        for id in [0x1e04u16, 0x2204, 0x2330, 0x2684, 0x2b85] {
            assert_ne!(
                nvidia_architecture_hint(id) & NVIDIA_ARCHITECTURE_TURING_OR_NEWER,
                0
            );
        }
        assert_eq!(nvidia_architecture_hint(0x1db6), 0);
    }
}
