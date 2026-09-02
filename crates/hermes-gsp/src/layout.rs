//! linux-firmware / OpenRM path layout for GSP artifacts.
//!
//! Two coexisting trees (see docs/GSP_FIRMWARE.md):
//! - Versioned OpenRM: `nvidia/<ver>/gsp_{tu10x,ga10x}.bin`
//! - Chip tree: `nvidia/<chip>/gsp/gsp-<driver>.bin[.xz]` with WHENCE links

extern crate alloc;

use alloc::string::String;

use crate::firmware::FirmwareFamily;
use hermes_core::NvidiaArchitecture;

/// Chip directory names used under `/lib/firmware/nvidia/`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NvidiaChipDir {
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
    Gh100,
    Gb100,
    Gb202,
}

impl NvidiaChipDir {
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
            Self::Gh100 => "gh100",
            Self::Gb100 => "gb100",
            Self::Gb202 => "gb202",
        }
    }

    /// WHENCE-resolved chip that actually holds the GSP files (after symlink).
    pub const fn gsp_canonical(self) -> Self {
        match self {
            Self::Tu104 | Self::Tu106 => Self::Tu102,
            Self::Tu117 => Self::Tu116,
            Self::Ga103 | Self::Ga104 | Self::Ga106 | Self::Ga107 => Self::Ga102,
            other => other,
        }
    }

    pub const fn firmware_family(self) -> FirmwareFamily {
        match self.gsp_canonical() {
            Self::Tu102 | Self::Tu116 | Self::Ga100 => FirmwareFamily::Tu10x,
            _ => FirmwareFamily::Ga10x,
        }
    }
}

/// Map architecture class to a representative chip directory for firmware lookup.
pub const fn chip_for_architecture(arch: NvidiaArchitecture) -> NvidiaChipDir {
    match arch {
        NvidiaArchitecture::Turing => NvidiaChipDir::Tu102,
        NvidiaArchitecture::Ampere => NvidiaChipDir::Ga102,
        NvidiaArchitecture::Hopper => NvidiaChipDir::Gh100,
        NvidiaArchitecture::Ada => NvidiaChipDir::Ad102,
        NvidiaArchitecture::Blackwell => NvidiaChipDir::Gb202,
    }
}

pub const GSP_TU10X_BASENAME: &str = "gsp_tu10x.bin";
pub const GSP_GA10X_BASENAME: &str = "gsp_ga10x.bin";
pub const UCODES_TU10X_BASENAME: &str = "ucodes_tu10x.bin";
pub const UCODES_GA10X_BASENAME: &str = "ucodes_ga10x.bin";

pub const fn openrm_gsp_basename(family: FirmwareFamily) -> &'static str {
    match family {
        FirmwareFamily::Tu10x => GSP_TU10X_BASENAME,
        FirmwareFamily::Ga10x => GSP_GA10X_BASENAME,
    }
}

/// Relative path: `nvidia/<version>/gsp_tu10x.bin`
pub fn openrm_gsp_relative(version: &str, family: FirmwareFamily) -> String {
    let mut s = String::new();
    s.push_str("nvidia/");
    s.push_str(version);
    s.push('/');
    s.push_str(openrm_gsp_basename(family));
    s
}

/// Chip-tree relative path: `nvidia/tu116/gsp/gsp-570.144.bin` (canonical chip).
pub fn chip_gsp_relative(chip: NvidiaChipDir, driver: &str) -> String {
    let canon = chip.gsp_canonical();
    let mut s = String::new();
    s.push_str("nvidia/");
    s.push_str(canon.as_str());
    s.push_str("/gsp/gsp-");
    s.push_str(driver);
    s.push_str(".bin");
    s
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootstrapArtifactKind {
    BooterLoad,
    BooterUnload,
    Bootloader,
    GenBootloader,
    Fmc,
    Scrubber,
}

impl BootstrapArtifactKind {
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::BooterLoad => "booter_load-",
            Self::BooterUnload => "booter_unload-",
            Self::Bootloader => "bootloader-",
            Self::GenBootloader => "gen_bootloader-",
            Self::Fmc => "fmc-",
            Self::Scrubber => "scrubber-",
        }
    }
}

pub const TURING_BOOTSTRAP_KINDS: &[BootstrapArtifactKind] = &[
    BootstrapArtifactKind::GenBootloader,
    BootstrapArtifactKind::Bootloader,
    BootstrapArtifactKind::BooterLoad,
    BootstrapArtifactKind::BooterUnload,
];

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::NvidiaArchitecture;

    #[test]
    fn whencelike_aliases_and_families() {
        assert_eq!(NvidiaChipDir::Tu117.gsp_canonical(), NvidiaChipDir::Tu116);
        assert_eq!(NvidiaChipDir::Tu104.gsp_canonical(), NvidiaChipDir::Tu102);
        assert_eq!(NvidiaChipDir::Ga107.gsp_canonical(), NvidiaChipDir::Ga102);
        assert_eq!(
            NvidiaChipDir::Ga100.firmware_family(),
            FirmwareFamily::Tu10x
        );
        assert_eq!(
            NvidiaChipDir::Ad102.firmware_family(),
            FirmwareFamily::Ga10x
        );
        assert_eq!(
            chip_for_architecture(NvidiaArchitecture::Blackwell),
            NvidiaChipDir::Gb202
        );
    }

    #[test]
    fn openrm_and_chip_paths() {
        assert_eq!(
            openrm_gsp_relative("610.43.02", FirmwareFamily::Tu10x),
            "nvidia/610.43.02/gsp_tu10x.bin"
        );
        assert_eq!(
            chip_gsp_relative(NvidiaChipDir::Tu117, "570.144"),
            "nvidia/tu116/gsp/gsp-570.144.bin"
        );
    }
}
