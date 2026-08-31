//! Fail-closed activation plan: ordered steps that feed the Hermes manifold.
//!
//! This module does not talk to hardware. It sequences the only legal order of
//! evidence publication. Online requires every step to succeed.

use hermes_core::{
    HermesManifold, HermesPhase, ManifoldFault, NvidiaArchitecture, feature,
};

use crate::firmware::{FirmwareFamily, VerifiedFirmware};

/// Ordered activation steps. Skipping is a type/logic error at the plan layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationStep {
    ProbePci,
    MeasureFirmware,
    IsolateIommu,
    ArmQueues,
    NegotiateFeatures,
    LockWpr,
    BootMailbox,
    ReadyQueue,
    PublishOnline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationPlan {
    pub architecture: NvidiaArchitecture,
    pub firmware_family: FirmwareFamily,
    pub firmware_version: u64,
    pub steps: [ActivationStep; 9],
}

/// Build the only legal activation plan for an admitted + firmware-verified GPU.
pub fn plan_activation(
    architecture: NvidiaArchitecture,
    firmware: &VerifiedFirmware,
) -> ActivationPlan {
    ActivationPlan {
        architecture,
        firmware_family: firmware.family,
        firmware_version: firmware.version,
        steps: [
            ActivationStep::ProbePci,
            ActivationStep::MeasureFirmware,
            ActivationStep::IsolateIommu,
            ActivationStep::ArmQueues,
            ActivationStep::NegotiateFeatures,
            ActivationStep::LockWpr,
            ActivationStep::BootMailbox,
            ActivationStep::ReadyQueue,
            ActivationStep::PublishOnline,
        ],
    }
}

/// Drive a manifold through a complete successful evidence chain.
/// Used by tests and by host simulators; production backends supply real evidence.
pub fn drive_full_success(
    generation: u32,
    dma_domain: u32,
    features: u64,
) -> Result<HermesManifold, ManifoldFault> {
    let mut m = HermesManifold::dark(generation);
    m.observe_probe(true)?;
    m.observe_firmware(true)?;
    m.arm_queues(true, dma_domain)?;
    m.negotiate(features)?;
    m.ignite(true, true, true)?;
    if !m.is_online() || m.phase != HermesPhase::Online {
        return Err(ManifoldFault::CertificateMissing);
    }
    Ok(m)
}

/// Default feature set for a display+compute GSP session (Agda-well-formed).
pub const fn default_negotiated_features() -> u64 {
    feature::BOOT_RPC
        | feature::COMMAND_RING
        | feature::EVENT_RING
        | feature::RECOVERY
        | feature::DISPLAY
        | feature::COPY_ENGINE
        | feature::TELEMETRY
        | feature::POWER
        | feature::MEMORY_MANAGEMENT
        | feature::COMPUTE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::{
        FirmwareFamily, NvidiaGspFirmwareAuthority, NvidiaGspFirmwareManifest, firmware_version,
        sha256_bytes,
    };
    use hermes_core::{NvidiaArchitecture, admit_display_device, pci_identity, NVIDIA_VENDOR_ID};

    #[test]
    fn plan_matches_architecture_and_never_skips_steps() {
        let payload = b"plan-test-fw";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let fw = auth.admit(0x1fb9, payload).unwrap();
        let plan = plan_activation(NvidiaArchitecture::Turing, &fw);
        assert_eq!(plan.steps[0], ActivationStep::ProbePci);
        assert_eq!(plan.steps[8], ActivationStep::PublishOnline);
        assert_eq!(plan.steps.len(), 9);
        assert_eq!(plan.firmware_family, FirmwareFamily::Tu10x);
    }

    #[test]
    fn end_to_end_admission_plus_gates_use_shipped_functions() {
        let id = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
        let admitted = admit_display_device(&id).expect("admit");
        let payload = b"e2e-hermes-gsp";
        let digest = sha256_bytes(payload);
        let manifest = NvidiaGspFirmwareManifest::new(
            FirmwareFamily::Tu10x,
            firmware_version(610, 43, 3),
            payload.len() as u32,
            digest,
        );
        let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
        let fw = auth.admit(admitted.identity.device_id, payload).unwrap();
        let architecture = match admitted.architecture {
            hermes_core::vendor::VendorArchitecture::Nvidia(architecture) => architecture,
            _ => panic!("NVIDIA identity admitted a non-NVIDIA architecture"),
        };
        let _plan = plan_activation(architecture, &fw);
        let online = drive_full_success(1, 42, default_negotiated_features()).unwrap();
        assert!(online.is_online());
        assert_eq!(online.evidence.dma_domain, 42);
    }

    #[test]
    fn incomplete_drive_cannot_reach_online() {
        let mut m = HermesManifold::dark(1);
        assert!(m.observe_probe(true).is_ok());
        // skip firmware
        assert!(m.arm_queues(true, 1).is_err());
        assert!(!m.is_online());
    }
}
