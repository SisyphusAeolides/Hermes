//! Host-side isolation and BAR readiness gates (pure logic, fail-closed).
//!
//! These predicates decide whether a Linux/sysfs view of a device is allowed to
//! attempt Hermes GSP Online. They never invent success: missing IOMMU or a
//! foreign bound driver blocks Online authority. Unit-tested without root.

use hermes_core::HermesFault;

/// Snapshot of host sysfs facts for one NVIDIA display GPU.
///
/// Built at the host edge (sysfs); pure gate logic lives here so bring-up and
/// tests share one fail-closed policy without inventing Online.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostDeviceFacts {
    /// Present when `/sys/.../iommu_group` resolves to a group id.
    pub iommu_group: Option<u32>,
    /// True when a foreign driver (Nouveau, vfio, …) owns the device.
    pub foreign_driver_bound: bool,
    /// True when Hermes/nvidia personality is bound.
    pub hermes_driver_bound: bool,
    /// True when BAR0 resource node exists and reports a nonzero window.
    pub bar0_described: bool,
    /// True only after a real open/mmap of `resource0` succeeded.
    pub bar0_mapped: bool,
}

impl HostDeviceFacts {
    pub const fn empty() -> Self {
        Self {
            iommu_group: None,
            foreign_driver_bound: false,
            hermes_driver_bound: false,
            bar0_described: false,
            bar0_mapped: false,
        }
    }

    /// SimPlatform / unit tests: isolated domain available, BAR described+mapped.
    pub const fn sim_ready() -> Self {
        Self {
            iommu_group: Some(1),
            foreign_driver_bound: false,
            hermes_driver_bound: true,
            bar0_described: true,
            bar0_mapped: true,
        }
    }
}

/// Why host Online is forbidden (operator-facing).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostGateBlocker {
    NoIommuGroup,
    ForeignDriver,
    BarNotDescribed,
    BarNotMapped,
}

impl HostGateBlocker {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoIommuGroup => "no IOMMU group — Online impossible",
            Self::ForeignDriver => "foreign driver bound — unbind before Hermes GSP BAR map",
            Self::BarNotDescribed => "BAR0 not described in PCI resource",
            Self::BarNotMapped => "BAR0 not mapped (privilege or driver ownership)",
        }
    }
}

/// Drivers that own the GPU and must be unbound before Hermes maps BAR0.
pub fn is_foreign_gpu_driver(name: &str) -> bool {
    matches!(
        name,
        "nouveau" | "vfio-pci" | "nvidiafb" | "nvidia_drm" | "nvidia-modeset"
    )
}

/// Classic proprietary / Hermes drop-in names that *are* our surface.
pub fn is_hermes_or_nvidia_driver(name: &str) -> bool {
    matches!(name, "nvidia" | "hermes" | "hermes-gsp")
}

/// Build facts from optional driver name + iommu + bar flags (host edge helper).
pub fn facts_from_sysfs(
    iommu_group: Option<u32>,
    driver: Option<&str>,
    bar0_described: bool,
    bar0_mapped: bool,
) -> HostDeviceFacts {
    let foreign = driver.map(is_foreign_gpu_driver).unwrap_or(false);
    let hermes = driver.map(is_hermes_or_nvidia_driver).unwrap_or(false);
    HostDeviceFacts {
        iommu_group,
        foreign_driver_bound: foreign,
        hermes_driver_bound: hermes,
        bar0_described,
        bar0_mapped,
    }
}

/// Collect blockers that forbid claiming Online on live silicon.
pub fn host_online_blockers(facts: &HostDeviceFacts) -> alloc::vec::Vec<HostGateBlocker> {
    let mut out = alloc::vec::Vec::new();
    if facts.iommu_group.is_none() {
        out.push(HostGateBlocker::NoIommuGroup);
    }
    if facts.foreign_driver_bound {
        out.push(HostGateBlocker::ForeignDriver);
    }
    if !facts.bar0_described {
        out.push(HostGateBlocker::BarNotDescribed);
    }
    if !facts.bar0_mapped {
        out.push(HostGateBlocker::BarNotMapped);
    }
    out
}

/// True only when no blockers remain.
pub fn host_may_claim_online(facts: &HostDeviceFacts) -> bool {
    host_online_blockers(facts).is_empty()
}

/// Preflight for isolation: IOMMU present, no foreign driver.
pub fn host_preflight_fault(facts: &HostDeviceFacts) -> Option<HermesFault> {
    if facts.iommu_group.is_none() {
        return Some(HermesFault::DeviceIsolation);
    }
    if facts.foreign_driver_bound {
        return Some(HermesFault::DeviceIsolation);
    }
    if !facts.bar0_described {
        return Some(HermesFault::BarUnavailable);
    }
    None
}

/// Stricter preflight when a live BAR map is required before Online.
pub fn host_preflight_fault_require_map(facts: &HostDeviceFacts) -> Option<HermesFault> {
    if let Some(f) = host_preflight_fault(facts) {
        return Some(f);
    }
    if !facts.bar0_mapped {
        return Some(HermesFault::BarUnavailable);
    }
    None
}

/// Whether isolation alone is legal (IOMMU present, no foreign driver).
pub fn host_isolation_ready(facts: &HostDeviceFacts) -> bool {
    facts.iommu_group.is_some() && !facts.foreign_driver_bound
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nouveau_without_iommu_blocks_online() {
        let facts = facts_from_sysfs(None, Some("nouveau"), true, false);
        let b = host_online_blockers(&facts);
        assert!(b.contains(&HostGateBlocker::NoIommuGroup));
        assert!(b.contains(&HostGateBlocker::ForeignDriver));
        assert!(b.contains(&HostGateBlocker::BarNotMapped));
        assert!(!host_may_claim_online(&facts));
        assert!(!host_isolation_ready(&facts));
        assert_eq!(
            host_preflight_fault(&facts),
            Some(HermesFault::DeviceIsolation)
        );
    }

    #[test]
    fn full_host_facts_allow_online_gate() {
        let facts = facts_from_sysfs(Some(12), Some("nvidia"), true, true);
        assert!(host_online_blockers(&facts).is_empty());
        assert!(host_may_claim_online(&facts));
        assert!(host_isolation_ready(&facts));
        assert_eq!(host_preflight_fault_require_map(&facts), None);
    }

    #[test]
    fn iommu_without_map_blocks_online_but_isolation_ready() {
        let facts = facts_from_sysfs(Some(3), None, true, false);
        assert!(host_isolation_ready(&facts));
        assert!(!host_may_claim_online(&facts));
        assert_eq!(
            host_preflight_fault_require_map(&facts),
            Some(HermesFault::BarUnavailable)
        );
        assert_eq!(host_preflight_fault(&facts), None);
    }

    #[test]
    fn foreign_driver_classification() {
        assert!(is_foreign_gpu_driver("nouveau"));
        assert!(is_foreign_gpu_driver("vfio-pci"));
        assert!(!is_foreign_gpu_driver("nvidia"));
        assert!(is_hermes_or_nvidia_driver("nvidia"));
    }

    #[test]
    fn sim_ready_facts_clear_all_blockers() {
        assert!(host_may_claim_online(&HostDeviceFacts::sim_ready()));
    }
}
