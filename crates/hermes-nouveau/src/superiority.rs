//! Explicit Hermes > Nouveau capability matrix (fail-closed Online).

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    LoadGspFirmwarePaths,
    BooterAndFmcStyles,
    RmApiR535R570,
    DrmKmsDisplay,
    MesaUserspace,
    NvidiaModuleNames,
    MeasuredSha256Admission,
    ElfStructuralAdmission,
    IommuRequiredForOnline,
    WprMailboxReadyCertificate,
    FormalIdrisAgdaAustral,
    QuarantineOnRpcFault,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HermesEdge {
    pub capability: Capability,
    pub nouveau: bool,
    pub hermes: bool,
}

impl HermesEdge {
    pub const fn hermes_advantage(self) -> bool {
        self.hermes && !self.nouveau
    }
}

/// Static comparison used by docs and `hermes-ctl nouveau-compare`.
pub fn comparison_matrix() -> Vec<HermesEdge> {
    alloc::vec![
        HermesEdge {
            capability: Capability::LoadGspFirmwarePaths,
            nouveau: true,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::BooterAndFmcStyles,
            nouveau: true,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::RmApiR535R570,
            nouveau: true,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::DrmKmsDisplay,
            nouveau: true,
            // hermes-drm: GSP-gated atomic modeset state machine (not full in-kernel DRM yet)
            hermes: true,
        },
        HermesEdge {
            capability: Capability::MesaUserspace,
            nouveau: true,
            // hermes-mesa: Vulkan ICD + GL stubs + present via atomic modeset
            hermes: true,
        },
        HermesEdge {
            capability: Capability::NvidiaModuleNames,
            nouveau: false,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::MeasuredSha256Admission,
            nouveau: false,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::ElfStructuralAdmission,
            nouveau: false,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::IommuRequiredForOnline,
            nouveau: false,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::WprMailboxReadyCertificate,
            nouveau: false,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::FormalIdrisAgdaAustral,
            nouveau: false,
            hermes: true,
        },
        HermesEdge {
            capability: Capability::QuarantineOnRpcFault,
            nouveau: false,
            hermes: true,
        },
    ]
}

pub fn hermes_exclusive_count() -> usize {
    comparison_matrix()
        .iter()
        .filter(|e| e.hermes_advantage())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hermes_has_exclusive_edges() {
        assert!(hermes_exclusive_count() >= 5);
        assert!(comparison_matrix()
            .iter()
            .any(|e| e.capability == Capability::MeasuredSha256Admission && e.hermes_advantage()));
    }
}
