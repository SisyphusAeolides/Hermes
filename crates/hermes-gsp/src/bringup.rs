//! Shared GSP bring-up sequencer.
//!
//! Walks Turing+ admission → optional host preflight → measured GSP-RM →
//! platform isolation/BAR/DMA full-image stage → optional live mailbox/WPR →
//! evidence-gated manifold progression. Online only when every evidence token
//! is present.
//! Production kmod and hermes-ctl call this path.

use hermes_abi::hermes::HermesPciIdentity;
use hermes_core::{
    admit_display_device, AdmittedDevice, DmaRegion, HermesFault, HermesManifold, HermesPhase,
    HermesPlatform, ManifoldFault, MmioWindow,
};

use crate::bootstrap::{TuringGspBootstrapMaterial, VerifiedTuringGspBootstrap};
use crate::firmware::{firmware_family_for_device, NvidiaGspFirmwareAuthority, VerifiedFirmware};
use crate::host_gate::{host_preflight_fault, HostDeviceFacts};
use crate::mailbox::{boot_handshake, MailboxEvidence};
use crate::session::default_negotiated_features;
use crate::stage::{stage_gsp_rm_image, stage_matches_admit, StageReport, STAGE_CHUNK_BYTES};
use crate::wpr::{
    TuringFramebufferEvidence, TuringGspDmaInputs, TuringMmuLock, TuringRiscvBootOffsets,
    TuringWprPlan, T1000_GSP_BOOT_BINARY_BYTES,
};

/// Hardware evidence that is not implied by PCI isolation alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HardwareEvidence {
    pub wpr_locked: bool,
    pub boot_mailbox_ok: bool,
    pub ready_queue_observed: bool,
}

impl HardwareEvidence {
    pub const fn none() -> Self {
        Self {
            wpr_locked: false,
            boot_mailbox_ok: false,
            ready_queue_observed: false,
        }
    }

    pub const fn full() -> Self {
        Self {
            wpr_locked: true,
            boot_mailbox_ok: true,
            ready_queue_observed: true,
        }
    }

    pub const fn and(self, other: Self) -> Self {
        Self {
            wpr_locked: self.wpr_locked && other.wpr_locked,
            boot_mailbox_ok: self.boot_mailbox_ok && other.boot_mailbox_ok,
            ready_queue_observed: self.ready_queue_observed && other.ready_queue_observed,
        }
    }
}

/// Inputs to one bring-up attempt.
pub struct BringupRequest<'a> {
    pub identity: HermesPciIdentity,
    pub generation: u32,
    pub gsp_rm_image: &'a [u8],
    pub firmware_authority: NvidiaGspFirmwareAuthority<'a>,
    pub bootstrap: Option<TuringGspBootstrapMaterial<'a>>,
    pub require_bootstrap: bool,
    pub features: u64,
    pub hardware: HardwareEvidence,
    pub drive_mailbox: bool,
    pub drive_wpr: bool,
    pub wpr_framebuffer: Option<TuringFramebufferEvidence>,
    pub wpr_boot_offsets: Option<TuringRiscvBootOffsets>,
    pub gsp_boot_binary_address: Option<u64>,
    /// Host IOMMU/driver/BAR facts; when set, preflighted before progression.
    pub host_facts: Option<HostDeviceFacts>,
    /// Keep domain/BAR/DMA live for the caller when Online.
    pub retain_on_online: bool,
}

impl<'a> BringupRequest<'a> {
    pub fn with_defaults(
        identity: HermesPciIdentity,
        gsp_rm_image: &'a [u8],
        authority: NvidiaGspFirmwareAuthority<'a>,
    ) -> Self {
        Self {
            identity,
            generation: 1,
            gsp_rm_image,
            firmware_authority: authority,
            bootstrap: None,
            require_bootstrap: false,
            features: default_negotiated_features(),
            hardware: HardwareEvidence::none(),
            drive_mailbox: false,
            drive_wpr: false,
            wpr_framebuffer: None,
            wpr_boot_offsets: None,
            gsp_boot_binary_address: None,
            host_facts: None,
            retain_on_online: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BringupFault {
    Admission(hermes_core::AdmissionError),
    Hermes(HermesFault),
    Manifold(ManifoldFault),
    BootstrapRequired,
    StageDigestMismatch,
    MailboxFailed,
    WprFailed,
    WprInputsMissing,
    HostPreflight,
}

impl From<hermes_core::AdmissionError> for BringupFault {
    fn from(value: hermes_core::AdmissionError) -> Self {
        Self::Admission(value)
    }
}

impl From<HermesFault> for BringupFault {
    fn from(value: HermesFault) -> Self {
        Self::Hermes(value)
    }
}

impl From<ManifoldFault> for BringupFault {
    fn from(value: ManifoldFault) -> Self {
        Self::Manifold(value)
    }
}

/// Result of the shared sequencer. Online only if `manifold.is_online()`.
#[derive(Clone, Debug)]
pub struct BringupReport {
    pub admitted: Option<AdmittedDevice>,
    pub firmware: Option<VerifiedFirmware>,
    pub bootstrap: Option<VerifiedTuringGspBootstrap>,
    pub manifold: HermesManifold,
    pub fault: Option<BringupFault>,
    pub domain_id: u32,
    pub stage: Option<StageReport>,
    pub mailbox: Option<MailboxEvidence>,
    pub wpr_locked_observed: bool,
    pub final_evidence: HardwareEvidence,
    pub resources_retained: bool,
}

impl BringupReport {
    pub fn is_online(&self) -> bool {
        self.manifold.is_online() && self.fault.is_none()
    }

    pub fn phase(&self) -> HermesPhase {
        self.manifold.phase
    }
}

/// Platform resources retained after a successful Online bring-up.
pub struct RetainedResources<P: HermesPlatform> {
    pub domain: P::Domain,
    pub bar: MmioWindow<P::Mmio>,
    pub dma: DmaRegion<P::Dma>,
}

/// Outcome of bring-up that may hand back live DMA/BAR for the session path.
pub struct BringupOutcome<P: HermesPlatform> {
    pub report: BringupReport,
    pub retained: Option<RetainedResources<P>>,
}

impl<P: HermesPlatform> BringupOutcome<P> {
    pub fn release(self, platform: &P) -> BringupReport {
        if let Some(r) = self.retained {
            platform.release_dma(r.dma);
            platform.unmap_bar(r.bar);
            platform.release_domain(r.domain);
        }
        let mut report = self.report;
        report.resources_retained = false;
        report
    }
}

fn fail<P: HermesPlatform>(report: BringupReport) -> BringupOutcome<P> {
    BringupOutcome {
        report,
        retained: None,
    }
}

/// Compat wrapper: always releases domain/BAR/DMA before return.
pub fn run_bringup<P: HermesPlatform>(platform: &P, request: &BringupRequest<'_>) -> BringupReport {
    let outcome = run_bringup_ex(platform, request);
    let mut report = outcome.release(platform);
    report.resources_retained = false;
    report
}

/// Extended bring-up: may retain domain/BAR/DMA when Online and `retain_on_online`.
pub fn run_bringup_ex<P: HermesPlatform>(
    platform: &P,
    request: &BringupRequest<'_>,
) -> BringupOutcome<P> {
    let mut report = BringupReport {
        admitted: None,
        firmware: None,
        bootstrap: None,
        manifold: HermesManifold::dark(request.generation),
        fault: None,
        domain_id: 0,
        stage: None,
        mailbox: None,
        wpr_locked_observed: false,
        final_evidence: HardwareEvidence::none(),
        resources_retained: false,
    };

    if let Some(facts) = request.host_facts {
        if let Some(fault) = host_preflight_fault(&facts) {
            report.fault = Some(BringupFault::Hermes(fault));
            return fail(report);
        }
    }

    let admitted = match admit_display_device(&request.identity) {
        Ok(a) => a,
        Err(e) => {
            report.fault = Some(e.into());
            return fail(report);
        }
    };
    report.admitted = Some(admitted);

    if let Err(e) = report.manifold.observe_probe(true) {
        report.fault = Some(e.into());
        return fail(report);
    }

    let firmware = match request
        .firmware_authority
        .admit(request.identity.device_id, request.gsp_rm_image)
    {
        Ok(fw) => fw,
        Err(e) => {
            report.fault = Some(e.into());
            return fail(report);
        }
    };
    let admitted_digest = firmware.sha256;
    report.firmware = Some(firmware);

    if let Err(e) = report.manifold.observe_firmware(true) {
        report.fault = Some(e.into());
        return fail(report);
    }

    if request.require_bootstrap {
        let material = match request.bootstrap {
            Some(m) => m,
            None => {
                report.fault = Some(BringupFault::BootstrapRequired);
                return fail(report);
            }
        };
        match material.verify_t1000_610_43_03() {
            Ok(v) => report.bootstrap = Some(v),
            Err(e) => {
                report.fault = Some(e.into());
                return fail(report);
            }
        }
    } else if let Some(material) = request.bootstrap {
        match material.verify_t1000_610_43_03() {
            Ok(v) => report.bootstrap = Some(v),
            Err(e) => {
                report.fault = Some(e.into());
                return fail(report);
            }
        }
    }

    let domain = match platform.isolate_device(request.identity) {
        Ok(d) => d,
        Err(e) => {
            report.fault = Some(e.into());
            return fail(report);
        }
    };
    let domain_id = domain_token::<P>(&domain);
    report.domain_id = domain_id;

    let bar = match platform.map_bar(domain, 0, 0x20_0000) {
        Ok(w) => w,
        Err(e) => {
            platform.release_domain(domain);
            report.fault = Some(e.into());
            return fail(report);
        }
    };
    if let Err(e) = platform.read32(bar, 0) {
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(e.into());
        return fail(report);
    }

    let (stage_rep, dma) =
        match stage_gsp_rm_image(platform, domain, request.gsp_rm_image, STAGE_CHUNK_BYTES) {
            Ok(v) => v,
            Err(crate::stage::StageError::EmptyImage) => {
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.fault = Some(BringupFault::Hermes(HermesFault::FirmwareMissing));
                return fail(report);
            }
            Err(crate::stage::StageError::Platform(e)) => {
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.fault = Some(e.into());
                return fail(report);
            }
            Err(crate::stage::StageError::DigestMismatch) => {
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.fault = Some(BringupFault::StageDigestMismatch);
                return fail(report);
            }
        };
    report.stage = Some(stage_rep);

    if !stage_matches_admit(&stage_rep.staged_sha256, &admitted_digest) {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(BringupFault::StageDigestMismatch);
        return fail(report);
    }

    let mut evidence = request.hardware;

    if request.drive_mailbox {
        match boot_handshake(platform, bar, 64) {
            Ok(ev) => {
                report.mailbox = Some(ev);
                evidence = evidence.and(HardwareEvidence {
                    wpr_locked: true,
                    boot_mailbox_ok: ev.mailbox_ok,
                    ready_queue_observed: ev.ready_ok,
                });
                if !ev.mailbox_ok || !ev.ready_ok {
                    evidence.boot_mailbox_ok = false;
                    evidence.ready_queue_observed = false;
                }
            }
            Err(_) => {
                evidence.boot_mailbox_ok = false;
                evidence.ready_queue_observed = false;
                report.fault = Some(BringupFault::MailboxFailed);
                platform.release_dma(dma);
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.final_evidence = evidence;
                return fail(report);
            }
        }
    }

    if request.drive_wpr {
        let fb = match request.wpr_framebuffer {
            Some(f) => f,
            None => {
                report.fault = Some(BringupFault::WprInputsMissing);
                platform.release_dma(dma);
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.final_evidence = evidence;
                return fail(report);
            }
        };
        let boot_off = match request.wpr_boot_offsets {
            Some(b) => b,
            None => {
                report.fault = Some(BringupFault::WprInputsMissing);
                platform.release_dma(dma);
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.final_evidence = evidence;
                return fail(report);
            }
        };
        let boot_bin = request.gsp_boot_binary_address.unwrap_or(0x1_0200_0000);
        let meta_addr = (boot_bin.wrapping_add(T1000_GSP_BOOT_BINARY_BYTES) + 4095) & !4095u64;
        let gsp_rm_iova =
            if stage_rep.last_device_address != 0 && stage_rep.last_device_address % 4096 == 0 {
                stage_rep.last_device_address
            } else {
                0x1_0000_0000
            };
        let dma_in = TuringGspDmaInputs {
            gsp_rm_address: gsp_rm_iova,
            gsp_rm_bytes: stage_rep.bytes_staged,
            gsp_boot_binary_address: boot_bin,
            metadata_address: meta_addr,
        };
        match TuringWprPlan::build(fb, dma_in, boot_off) {
            Ok(plan) => match plan.booter_load(meta_addr) {
                Ok(load) => {
                    let (lo, hi) = load.mailbox_words();
                    if platform.write32(bar, 0x0011_0040, lo).is_err()
                        || platform.write32(bar, 0x0011_0044, hi).is_err()
                    {
                        evidence.wpr_locked = false;
                        report.fault = Some(BringupFault::WprFailed);
                        platform.release_dma(dma);
                        platform.unmap_bar(bar);
                        platform.release_domain(domain);
                        report.final_evidence = evidence;
                        return fail(report);
                    }
                    let _ = platform.io_fence();
                    let mb0 = platform.read32(bar, 0x0011_0040).unwrap_or(0xffff_ffff);
                    let wpr_active = request.hardware.wpr_locked;
                    match load.complete(mb0, wpr_active) {
                        Ok(()) => {
                            report.wpr_locked_observed = true;
                            evidence = evidence.and(HardwareEvidence {
                                wpr_locked: true,
                                boot_mailbox_ok: true,
                                ready_queue_observed: true,
                            });
                        }
                        Err(_) => {
                            evidence.wpr_locked = false;
                            report.wpr_locked_observed = false;
                        }
                    }
                    let _ = plan.metadata;
                }
                Err(e) => {
                    report.fault = Some(e.into());
                    platform.release_dma(dma);
                    platform.unmap_bar(bar);
                    platform.release_domain(domain);
                    report.final_evidence = evidence;
                    return fail(report);
                }
            },
            Err(e) => {
                report.fault = Some(e.into());
                platform.release_dma(dma);
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.final_evidence = evidence;
                return fail(report);
            }
        }
    }

    report.final_evidence = evidence;

    if let Err(e) = report.manifold.arm_queues(true, domain_id) {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(e.into());
        return fail(report);
    }

    if let Err(e) = report.manifold.negotiate(request.features) {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(e.into());
        return fail(report);
    }

    match report.manifold.ignite(
        evidence.wpr_locked,
        evidence.boot_mailbox_ok,
        evidence.ready_queue_observed,
    ) {
        Ok(()) => {}
        Err(e) => {
            report.fault = Some(e.into());
            platform.release_dma(dma);
            platform.unmap_bar(bar);
            platform.release_domain(domain);
            return fail(report);
        }
    }

    let _ = firmware_family_for_device(request.identity.device_id);

    if request.retain_on_online {
        report.resources_retained = true;
        BringupOutcome {
            report,
            retained: Some(RetainedResources { domain, bar, dma }),
        }
    } else {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        fail(report)
    }
}

/// Sim-friendly WPR framebuffer sample that `TuringWprPlan::build` accepts.
pub fn sample_turing_wpr_framebuffer() -> TuringFramebufferEvidence {
    TuringFramebufferEvidence {
        usable_bytes: 4 * 1024 * 1024 * 1024,
        vga_workspace_offset: 4 * 1024 * 1024 * 1024 - 2 * 1024 * 1024,
        vga_workspace_bytes: 2 * 1024 * 1024,
        mmu_lock: TuringMmuLock::NotPresent,
        wpr_end_margin: 0,
        wpr_heap_bytes: 64 * 1024 * 1024,
        non_wpr_heap_bytes: 8 * 1024 * 1024,
    }
}

pub fn sample_turing_boot_offsets() -> TuringRiscvBootOffsets {
    TuringRiscvBootOffsets {
        monitor_code_offset: 0,
        monitor_data_offset: 0,
        manifest_offset: 0,
    }
}

fn domain_token<P: HermesPlatform>(domain: &P::Domain) -> u32 {
    let bytes = core::mem::size_of_val(domain);
    if bytes == 4 {
        let mut buf = [0u8; 4];
        unsafe {
            core::ptr::copy_nonoverlapping(
                (domain as *const P::Domain) as *const u8,
                buf.as_mut_ptr(),
                4,
            );
        }
        let v = u32::from_ne_bytes(buf);
        if v == 0 {
            1
        } else {
            v
        }
    } else {
        1
    }
}
