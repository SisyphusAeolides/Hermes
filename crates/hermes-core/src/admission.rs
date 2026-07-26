//! Device admission for NVIDIA Turing+ display controllers.

use hermes_abi::hermes::HermesPciIdentity;

use crate::family::{
    NVIDIA_VENDOR_ID, NvidiaArchitecture, is_nvidia_turing_or_newer, nvidia_architecture,
};
use crate::platform::HermesFault;

pub const PCI_CLASS_DISPLAY: u8 = 0x03;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    NotNvidia,
    NotDisplayController,
    UnsupportedArchitecture,
    InvalidIdentity,
}

impl From<AdmissionError> for HermesFault {
    fn from(value: AdmissionError) -> Self {
        match value {
            AdmissionError::NotNvidia => HermesFault::NotNvidia,
            AdmissionError::NotDisplayController => HermesFault::NotDisplayController,
            AdmissionError::UnsupportedArchitecture => HermesFault::UnsupportedArchitecture,
            AdmissionError::InvalidIdentity => HermesFault::CompatibilityRejected,
        }
    }
}

/// A device that has passed structural PCI + Turing+ gates (not yet online).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmittedDevice {
    pub identity: HermesPciIdentity,
    pub architecture: NvidiaArchitecture,
}

/// Admit any NVIDIA Turing+ device (display class preferred for graphics path).
pub fn admit_nvidia_device(identity: &HermesPciIdentity) -> Result<AdmittedDevice, AdmissionError> {
    validate_identity(identity)?;
    if identity.vendor_id != NVIDIA_VENDOR_ID {
        return Err(AdmissionError::NotNvidia);
    }
    let architecture =
        nvidia_architecture(identity.device_id).ok_or(AdmissionError::UnsupportedArchitecture)?;
    if !is_nvidia_turing_or_newer(identity.device_id) {
        return Err(AdmissionError::UnsupportedArchitecture);
    }
    Ok(AdmittedDevice {
        identity: *identity,
        architecture,
    })
}

/// Admit an NVIDIA Turing+ display controller (class 0x03).
pub fn admit_display_device(identity: &HermesPciIdentity) -> Result<AdmittedDevice, AdmissionError> {
    let admitted = admit_nvidia_device(identity)?;
    if identity.class_code != PCI_CLASS_DISPLAY {
        return Err(AdmissionError::NotDisplayController);
    }
    Ok(admitted)
}

fn validate_identity(identity: &HermesPciIdentity) -> Result<(), AdmissionError> {
    if identity.vendor_id == 0 || identity.vendor_id == 0xffff {
        return Err(AdmissionError::InvalidIdentity);
    }
    if identity.device_id == 0xffff {
        return Err(AdmissionError::InvalidIdentity);
    }
    Ok(())
}

/// Build a HermesPciIdentity for tests and host census tools.
pub const fn pci_identity(
    vendor_id: u16,
    device_id: u16,
    class_code: u8,
    subclass: u8,
) -> HermesPciIdentity {
    HermesPciIdentity {
        segment: 0,
        bus: 0,
        slot: 0,
        function: 0,
        revision: 0,
        vendor_id,
        device_id,
        subsystem_vendor_id: 0,
        subsystem_device_id: 0,
        class_code,
        subclass,
        programming_interface: 0,
        reserved: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::family::NVIDIA_VENDOR_ID;

    #[test]
    fn admits_t1000_display_rejects_volta() {
        let t1000 = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, PCI_CLASS_DISPLAY, 0x00);
        let admitted = admit_display_device(&t1000).expect("T1000 must admit");
        assert_eq!(admitted.architecture, NvidiaArchitecture::Turing);

        let volta = pci_identity(NVIDIA_VENDOR_ID, 0x1db6, PCI_CLASS_DISPLAY, 0x00);
        assert_eq!(
            admit_display_device(&volta),
            Err(AdmissionError::UnsupportedArchitecture)
        );
    }

    #[test]
    fn rejects_non_nvidia_and_non_display() {
        let intel = pci_identity(0x8086, 0x3e9b, PCI_CLASS_DISPLAY, 0x00);
        assert_eq!(
            admit_display_device(&intel),
            Err(AdmissionError::NotNvidia)
        );

        let audio = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x04, 0x03);
        assert_eq!(
            admit_display_device(&audio),
            Err(AdmissionError::NotDisplayController)
        );
        // Non-display NVIDIA Turing device can still admit via generic path.
        assert!(admit_nvidia_device(&audio).is_ok());
    }

    #[test]
    fn admits_ampere_ada_blackwell() {
        for id in [0x2204u16, 0x2684, 0x2b85] {
            let dev = pci_identity(NVIDIA_VENDOR_ID, id, PCI_CLASS_DISPLAY, 0x00);
            assert!(admit_display_device(&dev).is_ok(), "device {id:#x}");
        }
    }
}
