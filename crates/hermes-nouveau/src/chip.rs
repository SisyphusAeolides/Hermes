//! Nouveau chip identity mapped onto Hermes architecture families.

use hermes_core::{nvidia_architecture, NvidiaArchitecture};
use hermes_gsp::{firmware_family_for_device, FirmwareFamily};

/// Chip directory names used by Nouveau `NVKM_GSP_FIRMWARE_*` macros.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NouveauChip {
    Tu102,
    Tu104,
    Tu106,
    Tu116,
    Tu117,
    Ga100,
    Ga102,
    Ga103,
    Ga104,
    Ga106,
    Ga107,
    Ad102,
    Ad103,
    Ad104,
    Ad106,
    Ad107,
    Gh100,
    Gb100,
    Gb102,
    Gb202,
    Gb203,
    Gb205,
    Gb206,
    Gb207,
}

impl NouveauChip {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tu102 => "tu102",
            Self::Tu104 => "tu104",
            Self::Tu106 => "tu106",
            Self::Tu116 => "tu116",
            Self::Tu117 => "tu117",
            Self::Ga100 => "ga100",
            Self::Ga102 => "ga102",
            Self::Ga103 => "ga103",
            Self::Ga104 => "ga104",
            Self::Ga106 => "ga106",
            Self::Ga107 => "ga107",
            Self::Ad102 => "ad102",
            Self::Ad103 => "ad103",
            Self::Ad104 => "ad104",
            Self::Ad106 => "ad106",
            Self::Ad107 => "ad107",
            Self::Gh100 => "gh100",
            Self::Gb100 => "gb100",
            Self::Gb102 => "gb102",
            Self::Gb202 => "gb202",
            Self::Gb203 => "gb203",
            Self::Gb205 => "gb205",
            Self::Gb206 => "gb206",
            Self::Gb207 => "gb207",
        }
    }

    pub fn from_str_name(name: &str) -> Option<Self> {
        Some(match name {
            "tu102" => Self::Tu102,
            "tu104" => Self::Tu104,
            "tu106" => Self::Tu106,
            "tu116" => Self::Tu116,
            "tu117" => Self::Tu117,
            "ga100" => Self::Ga100,
            "ga102" => Self::Ga102,
            "ga103" => Self::Ga103,
            "ga104" => Self::Ga104,
            "ga106" => Self::Ga106,
            "ga107" => Self::Ga107,
            "ad102" => Self::Ad102,
            "ad103" => Self::Ad103,
            "ad104" => Self::Ad104,
            "ad106" => Self::Ad106,
            "ad107" => Self::Ad107,
            "gh100" => Self::Gh100,
            "gb100" => Self::Gb100,
            "gb102" => Self::Gb102,
            "gb202" => Self::Gb202,
            "gb203" => Self::Gb203,
            "gb205" => Self::Gb205,
            "gb206" => Self::Gb206,
            "gb207" => Self::Gb207,
            _ => return None,
        })
    }

    /// Canonical chip that actually stores GSP files after WHENCE/Nouveau aliases.
    pub const fn firmware_canonical(self) -> Self {
        match self {
            Self::Tu104 | Self::Tu106 => Self::Tu102,
            Self::Tu117 => Self::Tu116,
            Self::Ga103 | Self::Ga104 | Self::Ga106 | Self::Ga107 => Self::Ga102,
            Self::Ad103 | Self::Ad104 | Self::Ad106 | Self::Ad107 => Self::Ad102,
            Self::Gb102 => Self::Gb100,
            Self::Gb203 | Self::Gb205 | Self::Gb206 | Self::Gb207 => Self::Gb202,
            other => other,
        }
    }

    pub const fn family(self) -> ChipFamily {
        match self.firmware_canonical() {
            Self::Tu102 | Self::Tu116 => ChipFamily::Turing,
            Self::Ga100 => ChipFamily::AmpereGa100,
            Self::Ga102 => ChipFamily::AmpereGa10x,
            Self::Ad102 => ChipFamily::Ada,
            Self::Gh100 => ChipFamily::Hopper,
            Self::Gb100 | Self::Gb202 => ChipFamily::Blackwell,
            _ => ChipFamily::Turing,
        }
    }

    pub const fn uses_fmc(self) -> bool {
        matches!(
            self.firmware_canonical(),
            Self::Gh100 | Self::Gb100 | Self::Gb202
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChipFamily {
    Turing,
    AmpereGa100,
    AmpereGa10x,
    Ada,
    Hopper,
    Blackwell,
}

impl ChipFamily {
    pub const fn hermes_architecture(self) -> NvidiaArchitecture {
        match self {
            Self::Turing => NvidiaArchitecture::Turing,
            Self::AmpereGa100 | Self::AmpereGa10x => NvidiaArchitecture::Ampere,
            Self::Ada => NvidiaArchitecture::Ada,
            Self::Hopper => NvidiaArchitecture::Hopper,
            Self::Blackwell => NvidiaArchitecture::Blackwell,
        }
    }

    pub const fn gsp_rm_family(self) -> FirmwareFamily {
        match self {
            Self::Turing | Self::AmpereGa100 => FirmwareFamily::Tu10x,
            _ => FirmwareFamily::Ga10x,
        }
    }
}

/// Best-effort chip guess from PCI device id using Hermes architecture bands.
pub fn chip_hint_from_device_id(device_id: u16) -> Option<NouveauChip> {
    let arch = nvidia_architecture(device_id)?;
    let _fam = firmware_family_for_device(device_id)?;
    Some(match arch {
        NvidiaArchitecture::Turing => NouveauChip::Tu102,
        NvidiaArchitecture::Ampere => {
            // GA100 band is 0x20xx in Hermes family map.
            if (0x2000..=0x20ff).contains(&device_id) {
                NouveauChip::Ga100
            } else {
                NouveauChip::Ga102
            }
        }
        NvidiaArchitecture::Hopper => NouveauChip::Gh100,
        NvidiaArchitecture::Ada => NouveauChip::Ad102,
        NvidiaArchitecture::Blackwell => NouveauChip::Gb202,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_fmc() {
        assert_eq!(NouveauChip::Tu117.firmware_canonical(), NouveauChip::Tu116);
        assert_eq!(NouveauChip::Ad107.firmware_canonical(), NouveauChip::Ad102);
        assert!(NouveauChip::Gb202.uses_fmc());
        assert!(!NouveauChip::Tu102.uses_fmc());
        assert_eq!(chip_hint_from_device_id(0x1fb9), Some(NouveauChip::Tu102));
    }
}
