//! Shared GSP bring-up sequencer.
//!
//! Walks Turing+ admission → measured GSP-RM (and optional T1000 bootstrap) →
//! platform isolation/BAR/DMA full-image stage → optional live mailbox/WPR
//! observation → fail-closed manifold gates. Online is returned only when every
//! evidence token is present. Production kmod and hermes-ctl call this path.

use hermes_abi::hermes::HermesPciIdentity;
use hermes_core::{
    AdmittedDevice, HermesFault, HermesManifold, HermesPhase, HermesPlatform, ManifoldFault,
    admit_display_device,
};

use crate::bootstrap::{TuringGspBootstrapMaterial, VerifiedTuringGspBootstrap};
use crate::firmware::{
    NvidiaGspFirmwareAuthority, VerifiedFirmware, firmware_family_for_device,
};
use crate::mailbox::{boot_handshake, MailboxEvidence};
use crate::session::default_negotiated_features;
use crate::stage::{stage_gsp_rm_image, stage_matches_admit, StageReport, STAGE_CHUNK_BYTES};
use crate::wpr::{
    TuringFramebufferEvidence, TuringGspDmaInputs, TuringMmuLock, TuringRiscvBootOffsets,
    TuringWprPlan, T1000_GSP_BOOT_BINARY_BYTES,
};

/// Hardware evidence that is not implied by PCI isolation alone.
///
/// The Linux platform (or sim) fills these after real WPR/mailbox/ready work.
/// Leaving any false keeps Online unreachable. When `drive_mailbox` /
/// `drive_wpr` are set, live observations **AND** with these flags — they can
/// only tighten fail-closed, never invent success beyond what hardware shows.
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
    /// Required for Turing TU10x devices when the operator stages the five-file bundle.
    pub bootstrap: Option<TuringGspBootstrapMaterial<'a>>,
    pub require_bootstrap: bool,
    pub features: u64,
    pub hardware: HardwareEvidence,
    /// After DMA stage, run Falcon HELLO handshake and AND into evidence.
    pub drive_mailbox: bool,
    /// After stage, build WPR plan + SEC2 mailbox complete path (sim/host).
    pub drive_wpr: bool,
    /// Optional framebuffer evidence for WPR plan (required if drive_wpr).
    pub wpr_framebuffer: Option<TuringFramebufferEvidence>,
    /// Optional RISC-V boot offsets for WPR plan.
    pub wpr_boot_offsets: Option<TuringRiscvBootOffsets>,
    /// Optional mapped boot-binary DMA address (page-aligned).
    pub gsp_boot_binary_address: Option<u64>,
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
}

impl BringupReport {
    pub fn is_online(&self) -> bool {
        self.manifold.is_online() && self.fault.is_none()
    }

    pub fn phase(&self) -> HermesPhase {
        self.manifold.phase
    }
}

/// Run the production bring-up path against any `HermesPlatform`.
///
/// Order is fixed and fail-closed:
/// 1. admit display device (Turing+)
/// 2. manifold probe
/// 3. measure GSP-RM image (length+hash+ELF)
/// 4. optional T1000 bootstrap bundle verify
/// 5. isolate device (IOMMU domain)
/// 6. map BAR0, **stage full image** via chunked DMA, verify staged digest
/// 7. optional Falcon mailbox handshake (AND into evidence)
/// 8. optional WPR plan + SEC2 complete (AND into evidence)
/// 9. arm queues + negotiate features
/// 10. ignite only with combined evidence
pub fn run_bringup<P: HermesPlatform>(
    platform: &P,
    request: &BringupRequest<'_>,
) -> BringupReport {
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
    };

    let admitted = match admit_display_device(&request.identity) {
        Ok(a) => a,
        Err(e) => {
            report.fault = Some(e.into());
            return report;
        }
    };
    report.admitted = Some(admitted);

    if let Err(e) = report.manifold.observe_probe(true) {
        report.fault = Some(e.into());
        return report;
    }

    let firmware = match request
        .firmware_authority
        .admit(request.identity.device_id, request.gsp_rm_image)
    {
        Ok(fw) => fw,
        Err(e) => {
            report.fault = Some(e.into());
            return report;
        }
    };
    let admitted_digest = firmware.sha256;
    report.firmware = Some(firmware);

    if let Err(e) = report.manifold.observe_firmware(true) {
        report.fault = Some(e.into());
        return report;
    }

    if request.require_bootstrap {
        let material = match request.bootstrap {
            Some(m) => m,
            None => {
                report.fault = Some(BringupFault::BootstrapRequired);
                return report;
            }
        };
        match material.verify_t1000_610_43_03() {
            Ok(v) => report.bootstrap = Some(v),
            Err(e) => {
                report.fault = Some(e.into());
                return report;
            }
        }
    } else if let Some(material) = request.bootstrap {
        match material.verify_t1000_610_43_03() {
            Ok(v) => report.bootstrap = Some(v),
            Err(e) => {
                report.fault = Some(e.into());
                return report;
            }
        }
    }

    let domain = match platform.isolate_device(request.identity) {
        Ok(d) => d,
        Err(e) => {
            report.fault = Some(e.into());
            return report;
        }
    };
    let domain_id = domain_token::<P>(&domain);
    report.domain_id = domain_id;

    // Control BAR large enough for Falcon mailbox block (~0x11_xxxx).
    let bar = match platform.map_bar(domain, 0, 0x20_0000) {
        Ok(w) => w,
        Err(e) => {
            platform.release_domain(domain);
            report.fault = Some(e.into());
            return report;
        }
    };
    if let Err(e) = platform.read32(bar, 0) {
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(e.into());
        return report;
    }

    // Full-image DMA stage (chunked).
    let (stage_rep, dma) =
        match stage_gsp_rm_image(platform, domain, request.gsp_rm_image, STAGE_CHUNK_BYTES) {
            Ok(v) => v,
            Err(crate::stage::StageError::EmptyImage) => {
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.fault = Some(BringupFault::Hermes(HermesFault::FirmwareMissing));
                return report;
            }
            Err(crate::stage::StageError::Platform(e)) => {
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.fault = Some(e.into());
                return report;
            }
            Err(crate::stage::StageError::DigestMismatch) => {
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.fault = Some(BringupFault::StageDigestMismatch);
                return report;
            }
        };
    report.stage = Some(stage_rep);

    if !stage_matches_admit(&stage_rep.staged_sha256, &admitted_digest) {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(BringupFault::StageDigestMismatch);
        return report;
    }

    // Combine declared hardware evidence with optional live observations.
    let mut evidence = request.hardware;

    if request.drive_mailbox {
        match boot_handshake(platform, bar, 64) {
            Ok(ev) => {
                report.mailbox = Some(ev);
                evidence = evidence.and(HardwareEvidence {
                    wpr_locked: true, // mailbox path does not revoke WPR claim
                    boot_mailbox_ok: ev.mailbox_ok,
                    ready_queue_observed: ev.ready_ok,
                });
                // If handshake saw nothing, force mailbox/ready false.
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
                return report;
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
                return report;
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
                return report;
            }
        };
        // WPR DMA addresses must be page-aligned and nonzero. Prefer operator
        // overrides (real VT-d IOVAs); otherwise use proven-valid sim defaults
        // that satisfy TuringWprPlan::build while staging still used last_device_address
        // only as a fallback when it is page-aligned.
        let boot_bin = request
            .gsp_boot_binary_address
            .unwrap_or(0x1_0200_0000);
        let meta_addr = (boot_bin.wrapping_add(T1000_GSP_BOOT_BINARY_BYTES) + 4095) & !4095u64;
        let gsp_rm_iova = if stage_rep.last_device_address != 0
            && stage_rep.last_device_address % 4096 == 0
        {
            // Prefer staged region when it is a legal IOVA; else fall back.
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
                    // Post SEC2-style metadata address through Falcon mailboxes.
                    if platform.write32(bar, 0x0011_0040, lo).is_err()
                        || platform.write32(bar, 0x0011_0044, hi).is_err()
                    {
                        evidence.wpr_locked = false;
                        report.fault = Some(BringupFault::WprFailed);
                        platform.release_dma(dma);
                        platform.unmap_bar(bar);
                        platform.release_domain(domain);
                        report.final_evidence = evidence;
                        return report;
                    }
                    let _ = platform.io_fence();
                    // Booter completion: mailbox0 must be 0 and WPR2 active.
                    // Platforms that model SEC2 success clear MB0; otherwise complete fails.
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
                    return report;
                }
            },
            Err(e) => {
                report.fault = Some(e.into());
                platform.release_dma(dma);
                platform.unmap_bar(bar);
                platform.release_domain(domain);
                report.final_evidence = evidence;
                return report;
            }
        }
    }

    report.final_evidence = evidence;

    if let Err(e) = report.manifold.arm_queues(true, domain_id) {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(e.into());
        return report;
    }

    if let Err(e) = report.manifold.negotiate(request.features) {
        platform.release_dma(dma);
        platform.unmap_bar(bar);
        platform.release_domain(domain);
        report.fault = Some(e.into());
        return report;
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
            return report;
        }
    }

    platform.release_dma(dma);
    platform.unmap_bar(bar);
    platform.release_domain(domain);

    let _ = firmware_family_for_device(request.identity.device_id);
    report
}

/// Sim-friendly WPR framebuffer sample that `TuringWprPlan::build` accepts.
pub fn sample_turing_wpr_framebuffer() -> TuringFramebufferEvidence {
    // Matches hermes-gsp WPR unit-test geometry (valid for TuringWprPlan::build).
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
    // Offsets must be < T1000_GSP_BOOT_BINARY_BYTES (4096) and 4-byte aligned.
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
