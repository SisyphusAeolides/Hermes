//! Source-pinned GSP-RM firmware manifests and admission.

use hermes_core::family::{
    NVIDIA_ARCHITECTURE_ADA, NVIDIA_ARCHITECTURE_AMPERE, NVIDIA_ARCHITECTURE_BLACKWELL,
    NVIDIA_ARCHITECTURE_HOPPER, NVIDIA_ARCHITECTURE_TURING, nvidia_architecture_hint,
};
use hermes_core::HermesFault;
use sha2::{Digest, Sha256};

/// GSP-RM firmware line used by TU10x devices and GA100.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirmwareFamily {
    Tu10x,
    Ga10x,
}

pub const MAXIMUM_TU10X_GSP_BYTES: u32 = 32 * 1024 * 1024;
pub const MAXIMUM_GA10X_GSP_BYTES: u32 = 96 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NvidiaGspFirmwareManifest {
    pub family: FirmwareFamily,
    pub version: u64,
    pub byte_length: u32,
    pub sha256: [u8; 32],
}

impl NvidiaGspFirmwareManifest {
    pub const fn new(
        family: FirmwareFamily,
        version: u64,
        byte_length: u32,
        sha256: [u8; 32],
    ) -> Self {
        Self {
            family,
            version,
            byte_length,
            sha256,
        }
    }

    pub fn valid(self) -> bool {
        self.version != 0
            && self.byte_length != 0
            && self.sha256 != [0; 32]
            && self.byte_length <= maximum_image_bytes(self.family)
    }
}

pub const fn firmware_version(major: u16, minor: u16, patch: u16) -> u64 {
    (major as u64) << 32 | (minor as u64) << 16 | patch as u64
}

pub const fn maximum_image_bytes(family: FirmwareFamily) -> u32 {
    match family {
        FirmwareFamily::Tu10x => MAXIMUM_TU10X_GSP_BYTES,
        FirmwareFamily::Ga10x => MAXIMUM_GA10X_GSP_BYTES,
    }
}

/// Locally measured 610.43.03 GSP-RM artifacts (manifest data only).
pub const NVIDIA_GSP_RM_610_43_03: [NvidiaGspFirmwareManifest; 2] = [
    NvidiaGspFirmwareManifest::new(
        FirmwareFamily::Tu10x,
        firmware_version(610, 43, 3),
        29_352_832,
        [
            0x73, 0x06, 0x56, 0x19, 0xdb, 0x9e, 0xc9, 0x21, 0xd1, 0x9f, 0xc4, 0xe5, 0x19, 0xdd,
            0x04, 0xd9, 0x1a, 0x91, 0x99, 0xb5, 0x25, 0xea, 0xca, 0x9b, 0x25, 0x7b, 0x89, 0xfb,
            0x8c, 0x5e, 0x52, 0xc0,
        ],
    ),
    NvidiaGspFirmwareManifest::new(
        FirmwareFamily::Ga10x,
        firmware_version(610, 43, 3),
        84_277_400,
        [
            0x57, 0x23, 0x73, 0x62, 0x0a, 0x37, 0x41, 0x8f, 0x24, 0xdc, 0x16, 0xb5, 0x03, 0x1c,
            0x39, 0x33, 0x87, 0x78, 0xc3, 0x25, 0x7e, 0x48, 0xe8, 0x40, 0x8d, 0xe9, 0xa5, 0x72,
            0x91, 0xb2, 0x4f, 0x3a,
        ],
    ),
];

/// Classifies device IDs into GSP firmware lines. Unknown IDs return None.
pub const fn firmware_family_for_device(device_id: u16) -> Option<FirmwareFamily> {
    let architecture = nvidia_architecture_hint(device_id);
    if architecture & NVIDIA_ARCHITECTURE_TURING != 0 || (device_id >= 0x2000 && device_id <= 0x20ff)
    {
        return Some(FirmwareFamily::Tu10x);
    }
    if architecture
        & (NVIDIA_ARCHITECTURE_AMPERE
            | NVIDIA_ARCHITECTURE_HOPPER
            | NVIDIA_ARCHITECTURE_ADA
            | NVIDIA_ARCHITECTURE_BLACKWELL)
        != 0
    {
        return Some(FirmwareFamily::Ga10x);
    }
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedFirmware {
    pub family: FirmwareFamily,
    pub version: u64,
    pub byte_length: u32,
    pub sha256: [u8; 32],
}

/// Authenticates images against an allow-list. Empty allow-list rejects all.
pub struct NvidiaGspFirmwareAuthority<'a> {
    allow_list: &'a [NvidiaGspFirmwareManifest],
}

impl<'a> NvidiaGspFirmwareAuthority<'a> {
    pub const fn new(allow_list: &'a [NvidiaGspFirmwareManifest]) -> Self {
        Self { allow_list }
    }

    pub const fn default_610_43_03() -> Self {
        Self {
            allow_list: &NVIDIA_GSP_RM_610_43_03,
        }
    }

    /// Admit staged firmware bytes for a device only on exact length+hash match.
    pub fn admit(
        &self,
        device_id: u16,
        image: &[u8],
    ) -> Result<VerifiedFirmware, HermesFault> {
        let family = firmware_family_for_device(device_id).ok_or(HermesFault::UnsupportedArchitecture)?;
        let length = u32::try_from(image.len()).map_err(|_| HermesFault::FirmwareSize)?;
        if length == 0 {
            return Err(HermesFault::FirmwareMissing);
        }
        if length > maximum_image_bytes(family) {
            return Err(HermesFault::FirmwareSize);
        }

        let digest = sha256_bytes(image);
        let mut matched = None;
        for manifest in self.allow_list {
            if manifest.family == family
                && manifest.byte_length == length
                && manifest.sha256 == digest
                && manifest.valid()
            {
                matched = Some(*manifest);
                break;
            }
        }
        let manifest = matched.ok_or(HermesFault::FirmwareRejected)?;
        Ok(VerifiedFirmware {
            family: manifest.family,
            version: manifest.version,
            byte_length: manifest.byte_length,
            sha256: manifest.sha256,
        })
    }

    /// Reject without hashing when length alone cannot match any allow-list entry.
    pub fn reject_length_mismatch(&self, device_id: u16, length: u32) -> Result<(), HermesFault> {
        let family = firmware_family_for_device(device_id).ok_or(HermesFault::UnsupportedArchitecture)?;
        let any = self.allow_list.iter().any(|m| {
            m.family == family && m.byte_length == length && m.valid()
        });
        if any {
            Ok(())
        } else {
            Err(HermesFault::FirmwareRejected)
        }
    }
}

pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_selection_turing_vs_ga10x() {
        assert_eq!(firmware_family_for_device(0x1fb9), Some(FirmwareFamily::Tu10x));
        assert_eq!(firmware_family_for_device(0x1e04), Some(FirmwareFamily::Tu10x));
        assert_eq!(firmware_family_for_device(0x2204), Some(FirmwareFamily::Ga10x));
        assert_eq!(firmware_family_for_device(0x2684), Some(FirmwareFamily::Ga10x));
        assert_eq!(firmware_family_for_device(0x1db6), None);
    }

    #[test]
    fn length_mismatch_rejects_without_claiming_online() {
        let auth = NvidiaGspFirmwareAuthority::default_610_43_03();
        assert_eq!(
            auth.reject_length_mismatch(0x1fb9, 100),
            Err(HermesFault::FirmwareRejected)
        );
        assert!(auth
            .reject_length_mismatch(0x1fb9, 29_352_832)
            .is_ok());
    }

    #[test]
    fn wrong_hash_rejects_real_admit_path() {
        let auth = NvidiaGspFirmwareAuthority::default_610_43_03();
        // Exact length for tu10x but wrong content → hash mismatch → reject.
        let fake = alloc::vec![0u8; 29_352_832];
        assert_eq!(
            auth.admit(0x1fb9, &fake),
            Err(HermesFault::FirmwareRejected)
        );
    }

    #[test]
    fn correct_hash_admits_through_real_function() {
        let auth = NvidiaGspFirmwareAuthority::default_610_43_03();
        let manifest = NVIDIA_GSP_RM_610_43_03[0];
        // Build an image whose SHA-256 matches by using a synthetic allow-list
        // with a tiny known payload — proves admit() hashes and compares.
        let payload = b"hermes-gsp-test-image-v1";
        let digest = sha256_bytes(payload);
        let test_manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(1, 0, 0),
            payload.len() as u32,
            digest,
        );
        let test_auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&test_manifest));
        let verified = test_auth.admit(0x1fb9, payload).expect("must admit");
        assert_eq!(verified.sha256, digest);
        assert_eq!(verified.family, FirmwareFamily::Tu10x);
        // Default pin still rejects this tiny payload.
        assert!(auth.admit(0x1fb9, payload).is_err());
        assert!(manifest.valid());
    }

    #[test]
    fn empty_image_is_missing() {
        let auth = NvidiaGspFirmwareAuthority::default_610_43_03();
        assert_eq!(auth.admit(0x1fb9, &[]), Err(HermesFault::FirmwareMissing));
    }
}
