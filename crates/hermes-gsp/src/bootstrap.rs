//! Turing SEC2 / GSP bootstrap auxiliary image manifests.

use hermes_core::HermesFault;

use crate::firmware::{firmware_version, sha256_bytes};

/// One auxiliary image used by the documented SEC2/GSP bootstrap sequence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TuringGspBootstrapRole {
    GenericSec2Bootloader = 1,
    GspBootloader = 2,
    BooterLoad = 3,
    BooterUnload = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NvidiaGspBootstrapArtifactManifest {
    pub role: TuringGspBootstrapRole,
    pub byte_length: u32,
    pub sha256: [u8; 32],
}

impl NvidiaGspBootstrapArtifactManifest {
    pub const fn new(role: TuringGspBootstrapRole, byte_length: u32, sha256: [u8; 32]) -> Self {
        Self {
            role,
            byte_length,
            sha256,
        }
    }
}

/// Exact TU117 (Quadro T1000) auxiliary images for NVIDIA 610.43.03.
pub const T1000_TU117_BOOTSTRAP_610_43_03: [NvidiaGspBootstrapArtifactManifest; 4] = [
    NvidiaGspBootstrapArtifactManifest::new(
        TuringGspBootstrapRole::GenericSec2Bootloader,
        816,
        [
            0xb3, 0x77, 0x76, 0xa5, 0x11, 0xb4, 0xa0, 0x09, 0x01, 0xe4, 0xe3, 0xac, 0x56, 0x8d,
            0xb9, 0x17, 0x08, 0x6d, 0x3b, 0xf4, 0x39, 0xf8, 0x5b, 0xc9, 0xb3, 0xe4, 0xad, 0xc7,
            0x33, 0x8a, 0x0a, 0xff,
        ],
    ),
    NvidiaGspBootstrapArtifactManifest::new(
        TuringGspBootstrapRole::GspBootloader,
        4_196,
        [
            0x12, 0xe9, 0x87, 0xb6, 0x36, 0xc2, 0xf0, 0x0f, 0xa4, 0x0f, 0x42, 0xfd, 0x95, 0x09,
            0x75, 0x15, 0xc0, 0x81, 0x7b, 0x15, 0x81, 0x19, 0xc5, 0x84, 0x04, 0x9a, 0x37, 0xfa,
            0xf3, 0x8f, 0x8f, 0x96,
        ],
    ),
    NvidiaGspBootstrapArtifactManifest::new(
        TuringGspBootstrapRole::BooterLoad,
        59_016,
        [
            0x9b, 0xd0, 0x18, 0x04, 0xb4, 0xb9, 0x1d, 0x92, 0x90, 0x4e, 0x77, 0x35, 0xb0, 0x25,
            0xe0, 0x7a, 0x3c, 0x93, 0x5b, 0xc6, 0xfb, 0x92, 0xe3, 0x83, 0x3e, 0x85, 0xc2, 0x97,
            0x54, 0x60, 0x25, 0xb9,
        ],
    ),
    NvidiaGspBootstrapArtifactManifest::new(
        TuringGspBootstrapRole::BooterUnload,
        39_048,
        [
            0xbf, 0x4a, 0x2b, 0x77, 0x87, 0x22, 0xdd, 0xe5, 0x78, 0x50, 0x83, 0x9b, 0xb9, 0xf7,
            0x65, 0x1a, 0xe2, 0x95, 0x92, 0xce, 0x85, 0xd6, 0xdf, 0x41, 0x60, 0xd2, 0xd2, 0x28,
            0xff, 0x43, 0x01, 0x56,
        ],
    ),
];

#[derive(Clone, Copy)]
pub struct TuringGspBootstrapMaterial<'a> {
    pub generic_sec2_bootloader: &'a [u8],
    pub gsp_bootloader: &'a [u8],
    pub booter_load: &'a [u8],
    pub booter_unload: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedTuringGspBootstrap {
    pub version: u64,
    pub roles_ok: u8,
}

impl TuringGspBootstrapMaterial<'_> {
    /// Verifies every auxiliary image before any firmware DMA or SEC2 MMIO.
    pub fn verify_t1000_610_43_03(&self) -> Result<VerifiedTuringGspBootstrap, HermesFault> {
        let bytes = [
            self.generic_sec2_bootloader,
            self.gsp_bootloader,
            self.booter_load,
            self.booter_unload,
        ];
        let mut roles_ok = 0u8;
        for (artifact, expected) in bytes.iter().copied().zip(T1000_TU117_BOOTSTRAP_610_43_03) {
            let length = u32::try_from(artifact.len()).map_err(|_| HermesFault::FirmwareSize)?;
            let hash = sha256_bytes(artifact);
            if length != expected.byte_length || hash != expected.sha256 {
                return Err(HermesFault::FirmwareRejected);
            }
            roles_ok = roles_ok.saturating_add(1);
        }
        if roles_ok != 4 {
            return Err(HermesFault::FirmwareRejected);
        }
        Ok(VerifiedTuringGspBootstrap {
            version: firmware_version(610, 43, 3),
            roles_ok,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_bootstrap_bytes_reject() {
        let material = TuringGspBootstrapMaterial {
            generic_sec2_bootloader: &[0u8; 816],
            gsp_bootloader: &[0u8; 4196],
            booter_load: &[0u8; 59016],
            booter_unload: &[0u8; 39048],
        };
        assert_eq!(
            material.verify_t1000_610_43_03(),
            Err(HermesFault::FirmwareRejected)
        );
    }

    #[test]
    fn wrong_length_rejects() {
        let material = TuringGspBootstrapMaterial {
            generic_sec2_bootloader: &[0u8; 10],
            gsp_bootloader: &[0u8; 4196],
            booter_load: &[0u8; 59016],
            booter_unload: &[0u8; 39048],
        };
        assert_eq!(
            material.verify_t1000_610_43_03(),
            Err(HermesFault::FirmwareRejected)
        );
    }
}
