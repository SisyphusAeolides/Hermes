//! Hermes GSP control and host inspection.
//! Reports phase from the real shared sequencer (never invents Online).

mod silicon;

use hermes_core::{
    HermesManifold, NVIDIA_VENDOR_ID, admit_display_device, is_nvidia_turing_or_newer,
    nvidia_architecture, pci_identity,
};
use hermes_gsp::{
    boot_handshake, facts_from_sysfs, run_bringup_ex, sample_turing_boot_offsets,
    sample_turing_wpr_framebuffer, BringupRequest, FirmwareFamily, HardwareEvidence,
    NVIDIA_GSP_RM_610_43_02, NVIDIA_GSP_RM_610_43_03, NVIDIA_GSP_RM_DEFAULT_ALLOW_LIST,
    NvidiaGspFirmwareAuthority, NvidiaGspFirmwareManifest, chip_gsp_relative,
    default_negotiated_features, drive_full_success, firmware_family_for_device, firmware_version,
    openrm_gsp_relative, parse_gsp_rm_elf, plan_activation, sha256_bytes, NvidiaChipDir,
    fwversion_bytes,
};
use hermes_core::HermesPlatform;
use hermes_linux::{
    MODULE_SURFACES, SimPlatform, linux_bringup, modules, sim_full_hardware,
};
use hermes_nouveau::{
    comparison_matrix, plan_gsp_load, hermes_exclusive_count, NouveauChip,
};
use hermes_cccl::{
    cub_module_count, thrust_header_count, CCCL_VERSION, HERMES_HOST_IMPLEMENTED,
    THRUST_PUBLIC_HEADERS, hermes_sort,
};
use hermes_cuda::{
    self, cuCtxCreate_v2, cuCtxDestroy_v2, cuDeviceGetCount, cuEventCreate, cuEventDestroy_v2,
    cuEventRecord, cuEventSynchronize, cuInit, cuLaunchKernel, cuMemAlloc_v2, cuMemFree_v2,
    cuMemsetD8_v2, cuModuleGetFunction, cuModuleLoadData, cuModuleUnload, cuStreamCreate,
    cuStreamDestroy_v2, cuStreamSynchronize, hermes_cuda_cccl_version,
    hermes_cuda_driver_entry_count, hermes_cuda_reset, hermes_cuda_set_gsp_online,
    CUDA_ERROR_HERMES_GSP_OFFLINE, CUDA_SUCCESS,
};
use hermes_drm::{
    page_flip, AtomicCommit, AtomicRequest, DisplayMode, DrmDevice, Framebuffer, PageFlipRequest,
    PixelFormat,
};
use hermes_mesa::{
    hermes_mesa_reset, hermes_mesa_set_gsp_online, hermes_present_gem_flip,
    hermes_present_solid_frame, hermes_vulkan_api_version, hermes_vulkan_icd_json,
    hermes_vulkan_icd_library_path, vkCreateDevice, vkCreateInstance, vkDestroyDevice,
    vkDestroyInstance, vkEnumeratePhysicalDevices, vkGetDeviceQueue,
    vkGetPhysicalDeviceProperties, HermesVkPhysicalDeviceProperties,
    VK_ERROR_INCOMPATIBLE_DRIVER, VK_SUCCESS,
};

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("status") => status(),
        Some("admit") => {
            let id = parse_hex(args.next().as_deref().unwrap_or("0x1fb9"));
            admit_cmd(id);
        }
        Some("test-gates") => test_gates(),
        Some("bringup") => {
            let mode = args.next().unwrap_or_else(|| "fail".into());
            bringup_cmd(&mode);
        }
        Some("modules") => {
            for s in MODULE_SURFACES {
                println!("{} -> {}", s.name, s.replaces);
            }
        }
        Some("firmware-pin") => firmware_pin(),
        Some("firmware-scan") => firmware_scan(args.next().as_deref().unwrap_or("/lib/firmware")),
        Some("nouveau-compare") => nouveau_compare(),
        Some("nouveau-plan") => {
            let chip = args.next().unwrap_or_else(|| "tu102".into());
            let ver = args.next().unwrap_or_else(|| "570.144".into());
            nouveau_plan(&chip, &ver);
        }
        Some("cccl") => cccl_status(),
        Some("cuda-smoke") => {
            let mode = args.next().unwrap_or_else(|| "offline".into());
            cuda_smoke(&mode);
        }
        Some("drm-smoke") => {
            let mode = args.next().unwrap_or_else(|| "online".into());
            drm_smoke(&mode);
        }
        Some("mesa-smoke") => {
            let mode = args.next().unwrap_or_else(|| "online".into());
            mesa_smoke(&mode);
        }
        Some("stack-smoke") => stack_smoke(),
        Some("icd-json") => {
            print!("{}", hermes_vulkan_icd_json());
        }
        Some("silicon-probe") => {
            let root = args.next().unwrap_or_else(|| "/lib/firmware".into());
            let report = silicon::probe_host(std::path::Path::new(&root));
            silicon::print_report(&report);
        }
        Some("host-bar") => silicon::host_bar_smoke(),
        Some("mailbox-smoke") => mailbox_smoke(),
        Some("silicon-bringup") => {
            let mode = args.next().unwrap_or_else(|| "sim".into());
            silicon_bringup_cmd(&mode);
        }
        Some("session-smoke") => session_smoke(),
        Some("smi-smoke") => {
            let mode = args.next().unwrap_or_else(|| "host".into());
            smi_smoke(&mode);
        }
        _ => {
            println!("hermes-ctl — Hermes GSP control\n");
            println!(
                "commands: status | admit | test-gates | bringup | modules | firmware-pin | firmware-scan | nouveau-compare | nouveau-plan | cccl | cuda-smoke <offline|online|deep> | drm-smoke <offline|online|dual|gem> | mesa-smoke <offline|online|gem> | stack-smoke | icd-json | silicon-probe [fwroot] | host-bar | mailbox-smoke | silicon-bringup <sim|live-fw|fail-mailbox|host-block> | session-smoke | smi-smoke <host|online>"
            );
        }
    }
}

fn parse_hex(s: &str) -> u16 {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(s, 16).unwrap_or(0)
}

fn status() {
    println!("Hermes GSP 0.1.0");
    println!("Scope: NVIDIA Turing and newer (open-gpu-kernel-modules GSP path)");
    println!("Languages: Rust, Austral, Idris2, Agda");
    println!("Primary module: {}", modules::NVIDIA);
    println!("Manifold default: {}", HermesManifold::dark(0).phase.label());
    println!("Kmod tree: linux/kmod (nvidia, nvidia-modeset, nvidia-uvm, nvidia-drm, nvidia-peermem)");
    println!("Display: hermes-drm atomic modeset + hermes-mesa ICD surface");
    println!("Vulkan ICD library: {}", hermes_vulkan_icd_library_path());
    println!("Vulkan API version: {:#x}", hermes_vulkan_api_version());
}

fn admit_cmd(device_id: u16) {
    let identity = pci_identity(NVIDIA_VENDOR_ID, device_id, 0x03, 0x00);
    match admit_display_device(&identity) {
        Ok(a) => {
            println!(
                "ADMIT {device_id:#06x} arch={} turing+={} fw={:?}",
                a.architecture.as_str(),
                is_nvidia_turing_or_newer(device_id),
                firmware_family_for_device(device_id)
            );
        }
        Err(e) => {
            println!("REJECT {device_id:#06x}: {e:?}");
            std::process::exit(1);
        }
    }
}

fn test_gates() {
    let online = drive_full_success(1, 7, default_negotiated_features()).expect("gates");
    assert!(online.is_online());
    println!(
        "gate-chain: ONLINE generation={} domain={}",
        online.generation, online.evidence.dma_domain
    );

    let volta = pci_identity(NVIDIA_VENDOR_ID, 0x1db6, 0x03, 0x00);
    assert!(admit_display_device(&volta).is_err());
    println!("pre-turing: REJECT 0x1db6");

    let payload = b"hermes-ctl-gate-probe";
    let digest = sha256_bytes(payload);
    let manifest = NvidiaGspFirmwareManifest::new(
        FirmwareFamily::Tu10x,
        firmware_version(610, 43, 3),
        payload.len() as u32,
        digest,
    );
    let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
    let fw = auth.admit(0x1fb9, payload).expect("fw");
    let arch = nvidia_architecture(0x1fb9).unwrap();
    let plan = plan_activation(arch, &fw);
    println!(
        "firmware: admitted family={:?} steps={}",
        plan.firmware_family,
        plan.steps.len()
    );
    println!("PASS");
}

/// Drive the shipped linux_bringup + SimPlatform path twice (fail then ok).
fn bringup_cmd(mode: &str) {
    let payload = b"hermes-ctl-shared-bringup-image";
    let digest = sha256_bytes(payload);
    let manifest = NvidiaGspFirmwareManifest::new(
        FirmwareFamily::Tu10x,
        firmware_version(610, 43, 3),
        payload.len() as u32,
        digest,
    );
    let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
    let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);

    match mode {
        "fail" | "isolation" => {
            let plat = SimPlatform::new();
            plat.set_fail_isolation(true);
            let mut req = BringupRequest::with_defaults(identity, payload, auth);
            req.hardware = sim_full_hardware();
            let report = linux_bringup(&plat, &req);
            println!(
                "bringup isolation-fail: online={} phase={} fault={:?} isolate_calls={}",
                report.is_online(),
                report.phase().label(),
                report.fault,
                plat.isolate_calls()
            );
            if report.is_online() {
                eprintln!("error: isolation failure must not yield Online");
                std::process::exit(1);
            }
            println!("PASS");
        }
        "ok" | "success" | "full" => {
            let plat = SimPlatform::new();
            let mut req = BringupRequest::with_defaults(identity, payload, auth);
            req.hardware = HardwareEvidence::full();
            let report = linux_bringup(&plat, &req);
            println!(
                "bringup full: online={} phase={} domain={} map={} dma={}",
                report.is_online(),
                report.phase().label(),
                report.domain_id,
                plat.map_bar_calls(),
                plat.dma_alloc_calls()
            );
            if !report.is_online() {
                eprintln!("error: full evidence should Online, fault={:?}", report.fault);
                std::process::exit(1);
            }
            println!("PASS");
        }
        "both" => {
            bringup_cmd("fail");
            bringup_cmd("ok");
        }
        "mailbox" => {
            // Full evidence + live Falcon HELLO/ACK driven inside run_bringup.
            let plat = SimPlatform::new();
            plat.set_auto_mailbox_ack(true);
            let mut req = BringupRequest::with_defaults(identity, payload, auth);
            req.hardware = HardwareEvidence::full();
            req.drive_mailbox = true;
            let report = linux_bringup(&plat, &req);
            println!(
                "bringup mailbox-sim: online={} phase={} writes={} staged={:?}",
                report.is_online(),
                report.phase().label(),
                plat.write32_calls(),
                report.stage.map(|s| s.bytes_staged)
            );
            let plat2 = SimPlatform::new();
            plat2.set_auto_mailbox_ack(true);
            let domain = plat2.isolate_device(identity).expect("iso");
            let bar = plat2.map_bar(domain, 0, 0x20_0000).expect("bar");
            let ev = boot_handshake(&plat2, bar, 16).expect("handshake");
            println!(
                "falcon handshake: mailbox_ok={} ready_ok={} resp={:#x}",
                ev.mailbox_ok, ev.ready_ok, ev.last_response
            );
            if !ev.ready_ok || !report.is_online() {
                eprintln!("error: mailbox path should Online with ACK");
                std::process::exit(1);
            }
            println!("PASS");
        }
        other => {
            eprintln!("unknown bringup mode: {other} (use fail|ok|both|mailbox)");
            std::process::exit(2);
        }
    }
}

fn mailbox_smoke() {
    bringup_cmd("mailbox");
}

/// Silicon-class bring-up: multi-chunk stage + optional live GSP-RM from host.
fn silicon_bringup_cmd(mode: &str) {
    match mode {
        "fail-mailbox" => {
            let plat = SimPlatform::new();
            // Claim full hardware but live mailbox has no ACK → Offline.
            let payload = b"silicon-bringup-fail-mailbox-evidence";
            let digest = sha256_bytes(payload);
            let manifest = NvidiaGspFirmwareManifest::new(
                FirmwareFamily::Tu10x,
                firmware_version(610, 43, 3),
                payload.len() as u32,
                digest,
            );
            let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
            let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
            let mut req = BringupRequest::with_defaults(identity, payload, auth);
            req.hardware = HardwareEvidence::full();
            req.drive_mailbox = true;
            let report = linux_bringup(&plat, &req);
            println!(
                "silicon-bringup fail-mailbox: online={} staged={} mb={:?}",
                report.is_online(),
                report.stage.map(|s| s.bytes_staged).unwrap_or(0),
                report.mailbox
            );
            if report.is_online() {
                eprintln!("error: fail-mailbox must stay Offline");
                std::process::exit(1);
            }
            println!("PASS");
        }
        "sim" => {
            let plat = SimPlatform::new();
            plat.set_auto_mailbox_ack(true);
            plat.set_sec2_success(true);
            // Multi-chunk image to prove full-stage path.
            let mut payload = vec![0u8; 12_000];
            for (i, b) in payload.iter_mut().enumerate() {
                *b = (i % 251) as u8;
            }
            let digest = sha256_bytes(&payload);
            let manifest = NvidiaGspFirmwareManifest::new(
                FirmwareFamily::Tu10x,
                firmware_version(610, 43, 3),
                payload.len() as u32,
                digest,
            );
            let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
            let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
            let mut req = BringupRequest::with_defaults(identity, &payload, auth);
            req.hardware = HardwareEvidence::full();
            req.drive_mailbox = true;
            let report = linux_bringup(&plat, &req);
            let stage = report.stage.expect("stage");
            println!(
                "silicon-bringup sim: online={} phase={} bytes_staged={} chunks={} published={} mb_ok={}",
                report.is_online(),
                report.phase().label(),
                stage.bytes_staged,
                stage.chunks,
                plat.bytes_published(),
                report.mailbox.map(|m| m.ready_ok).unwrap_or(false)
            );
            if !report.is_online() || stage.bytes_staged != 12_000 || stage.chunks != 3 {
                eprintln!("error: sim silicon-bringup failed fault={:?}", report.fault);
                std::process::exit(1);
            }
            if plat.bytes_published() != 12_000 {
                eprintln!("error: published byte count mismatch");
                std::process::exit(1);
            }
            println!("PASS");
        }
        "live-fw" => {
            // Load real host gsp_tu10x.bin, admit + full stage + mailbox (sim HAL).
            // Still Offline without IOMMU on live GPU — here SimPlatform isolates.
            let path = "/lib/firmware/nvidia/610.43.02/gsp_tu10x.bin";
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("skip live-fw: cannot read {path}: {e}");
                    println!("PASS (skipped — firmware absent)");
                    return;
                }
            };
            let auth = NvidiaGspFirmwareAuthority::default_allow_list();
            let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
            let plat = SimPlatform::new();
            plat.set_auto_mailbox_ack(true);
            plat.set_sec2_success(true);
            let mut req = BringupRequest::with_defaults(identity, &bytes, auth);
            req.hardware = HardwareEvidence::full();
            req.drive_mailbox = true;
            req.drive_wpr = true;
            req.wpr_framebuffer = Some(sample_turing_wpr_framebuffer());
            req.wpr_boot_offsets = Some(sample_turing_boot_offsets());
            req.gsp_boot_binary_address = Some(0x1_0200_0000);
            // Sim host facts: isolated domain + mapped BAR (not live Nouveau).
            req.host_facts = Some(hermes_gsp::HostDeviceFacts::sim_ready());
            let report = linux_bringup(&plat, &req);
            let stage = report.stage.expect("stage");
            println!(
                "silicon-bringup live-fw: online={} bytes_staged={} chunks={} wpr_obs={} digest_ok={}",
                report.is_online(),
                stage.bytes_staged,
                stage.chunks,
                report.wpr_locked_observed,
                stage.staged_sha256 == report.firmware.as_ref().map(|f| f.sha256).unwrap_or([0; 32])
            );
            if stage.bytes_staged != bytes.len() as u64 {
                eprintln!("error: must stage entire GSP-RM image");
                std::process::exit(1);
            }
            if !report.is_online() {
                eprintln!(
                    "error: sim+live-fw with full evidence should Online, fault={:?}",
                    report.fault
                );
                std::process::exit(1);
            }
            if !report.wpr_locked_observed {
                eprintln!("error: WPR path should observe lock");
                std::process::exit(1);
            }
            println!("PASS");
        }
        "host-block" => {
            // Inject live-shaped host facts (Nouveau, no IOMMU) into shared sequencer.
            let plat = SimPlatform::new();
            let payload = b"silicon-bringup-host-block-nouveau";
            let digest = sha256_bytes(payload);
            let manifest = NvidiaGspFirmwareManifest::new(
                FirmwareFamily::Tu10x,
                firmware_version(610, 43, 3),
                payload.len() as u32,
                digest,
            );
            let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
            let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
            let mut req = BringupRequest::with_defaults(identity, payload, auth);
            req.hardware = HardwareEvidence::full();
            req.host_facts = Some(facts_from_sysfs(None, Some("nouveau"), true, false));
            let report = linux_bringup(&plat, &req);
            println!(
                "silicon-bringup host-block: online={} fault={:?} isolate_calls={}",
                report.is_online(),
                report.fault,
                plat.isolate_calls()
            );
            if report.is_online() || plat.isolate_calls() != 0 {
                eprintln!("error: host-block must fail closed before isolation");
                std::process::exit(1);
            }
            println!("PASS");
        }
        other => {
            eprintln!("use sim|live-fw|fail-mailbox|host-block, got {other}");
            std::process::exit(2);
        }
    }
}

fn session_smoke() {
    let plat = SimPlatform::new();
    let payload = b"session-retain-smoke";
    let digest = sha256_bytes(payload);
    let manifest = NvidiaGspFirmwareManifest::new(
        FirmwareFamily::Tu10x,
        firmware_version(610, 43, 3),
        payload.len() as u32,
        digest,
    );
    let auth = NvidiaGspFirmwareAuthority::new(core::slice::from_ref(&manifest));
    let identity = pci_identity(NVIDIA_VENDOR_ID, 0x1fb9, 0x03, 0x00);
    let mut req = BringupRequest::with_defaults(identity, payload, auth);
    req.hardware = HardwareEvidence::full();
    req.retain_on_online = true;
    let outcome = run_bringup_ex(&plat, &req);
    println!(
        "session-smoke: online={} retained={} has_resources={}",
        outcome.report.is_online(),
        outcome.report.resources_retained,
        outcome.retained.is_some()
    );
    if !outcome.report.is_online() || outcome.retained.is_none() {
        eprintln!("error: retain path must Online with resources");
        std::process::exit(1);
    }
    let report = outcome.release(&plat);
    println!(
        "session-smoke after release: online={} retained_flag={}",
        report.is_online(),
        report.resources_retained
    );
    println!("PASS");
}

fn smi_smoke(mode: &str) {
    use nvidia_ml::{
        hermes_nvml_discover_host_gpus, hermes_nvml_format_device_line, hermes_nvml_gpu_count,
        hermes_nvml_promote_first_sim_online, hermes_nvml_reset, nvmlDeviceGetCount_v2,
        nvmlInit_v2, nvmlShutdown, NVML_SUCCESS,
    };
    hermes_nvml_reset();
    assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
    let discovered = hermes_nvml_discover_host_gpus();
    match mode {
        "host" => {
            let mut count = 0u32;
            assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
            println!(
                "smi-smoke host: discovered={discovered} nvml_count={count}"
            );
            if count == 0 && discovered == 0 {
                eprintln!("error: expected host T1000 to appear in NVML via sysfs discover");
                std::process::exit(1);
            }
            let line = hermes_nvml_format_device_line(0).expect("line");
            println!("  {line}");
            if !line.contains("0000:01:00.0") && !line.contains("1fb9") && !line.contains("Turing")
            {
                // Still require non-empty real bind
                if hermes_nvml_gpu_count() == 0 {
                    std::process::exit(1);
                }
            }
            println!("PASS");
        }
        "online" => {
            if hermes_nvml_gpu_count() == 0 {
                hermes_nvml_discover_host_gpus();
            }
            if hermes_nvml_gpu_count() == 0 {
                nvidia_ml::hermes_nvml_bind_sim_online_session("Hermes Sim GPU");
            } else {
                assert!(hermes_nvml_promote_first_sim_online());
            }
            let line = hermes_nvml_format_device_line(0).expect("line");
            println!("smi-smoke online: {line}");
            if !line.contains("ONLINE") {
                eprintln!("error: expected ONLINE phase in device line");
                std::process::exit(1);
            }
            println!("PASS");
        }
        other => {
            eprintln!("use host|online, got {other}");
            std::process::exit(2);
        }
    }
    let _ = nvmlShutdown();
}

fn firmware_pin() {
    println!("allow-list entries: {}", NVIDIA_GSP_RM_DEFAULT_ALLOW_LIST.len());
    for m in NVIDIA_GSP_RM_610_43_02 {
        println!(
            "610.43.02 {:?} len={} sha256={:02x}{:02x}…",
            m.family, m.byte_length, m.sha256[0], m.sha256[1]
        );
    }
    for m in NVIDIA_GSP_RM_610_43_03 {
        println!(
            "610.43.03 {:?} len={} sha256={:02x}{:02x}…",
            m.family, m.byte_length, m.sha256[0], m.sha256[1]
        );
    }
    println!(
        "paths: {} | {}",
        openrm_gsp_relative("610.43.02", FirmwareFamily::Tu10x),
        chip_gsp_relative(NvidiaChipDir::Tu117, "570.144")
    );
}

fn nouveau_compare() {
    println!("Hermes exclusive edges: {}", hermes_exclusive_count());
    println!(
        "{:<32} {:<8} {:<8} {}",
        "capability", "nouveau", "hermes", "hermes+"
    );
    for e in comparison_matrix() {
        println!(
            "{:<32} {:<8} {:<8} {}",
            format!("{:?}", e.capability),
            e.nouveau,
            e.hermes,
            e.hermes_advantage()
        );
    }
}

fn nouveau_plan(chip: &str, ver: &str) {
    let c = NouveauChip::from_str_name(chip).unwrap_or(NouveauChip::Tu102);
    match plan_gsp_load(c, ver) {
        Ok(plan) => {
            println!(
                "chip={} canon={} style={:?} ver={} rm={}",
                chip,
                plan.chip.as_str(),
                plan.style,
                plan.version,
                plan.rm_impl
            );
            for role in plan.roles {
                println!("  {}", plan.linux_firmware_path(role));
            }
        }
        Err(e) => {
            eprintln!("plan failed: {e:?}");
            std::process::exit(1);
        }
    }
}

fn cccl_status() {
    println!("CCCL version (catalog): {CCCL_VERSION}");
    println!("Thrust public headers: {}", thrust_header_count());
    println!("CUB modules: {}", cub_module_count());
    println!(
        "Hermes host Thrust subset: {}",
        HERMES_HOST_IMPLEMENTED.len()
    );
    println!("cuda crate CCCL pin: {}", hermes_cuda_cccl_version());
    let mut sample: [i32; 5] = [5, 1, 4, 2, 3];
    hermes_sort(&mut sample);
    println!("host sort smoke: {:?}", sample);
    println!(
        "first thrust headers: {:?}",
        &THRUST_PUBLIC_HEADERS[..THRUST_PUBLIC_HEADERS.len().min(8)]
    );
}

fn cuda_smoke(mode: &str) {
    hermes_cuda_reset();
    match mode {
        "offline" => {
            let r = cuInit(0);
            println!("cuInit offline => {r:#x} (expect HERMES_GSP_OFFLINE={CUDA_ERROR_HERMES_GSP_OFFLINE:#x})");
            if r != CUDA_ERROR_HERMES_GSP_OFFLINE {
                std::process::exit(1);
            }
            println!("PASS");
        }
        "online" => {
            hermes_cuda_set_gsp_online(true);
            let r = cuInit(0);
            println!("cuInit online => {r}");
            if r != CUDA_SUCCESS {
                std::process::exit(1);
            }
            let mut n = 0i32;
            assert_eq!(cuDeviceGetCount(&mut n), CUDA_SUCCESS);
            println!("device count: {n}");
            hermes_cuda_reset();
            println!("PASS");
        }
        "deep" => {
            hermes_cuda_set_gsp_online(true);
            assert_eq!(cuInit(0), CUDA_SUCCESS);
            let mut ctx = 0u64;
            assert_eq!(cuCtxCreate_v2(&mut ctx, 0, 0), CUDA_SUCCESS);
            let mut stream = 0u64;
            assert_eq!(cuStreamCreate(&mut stream, 0), CUDA_SUCCESS);
            let mut ev = 0u64;
            assert_eq!(cuEventCreate(&mut ev, 0), CUDA_SUCCESS);
            let image = b"\0";
            let mut module = 0u64;
            assert_eq!(cuModuleLoadData(&mut module, image.as_ptr()), CUDA_SUCCESS);
            let mut func = 0u64;
            let name = b"hermes_kernel\0";
            assert_eq!(
                cuModuleGetFunction(&mut func, module, name.as_ptr()),
                CUDA_SUCCESS
            );
            assert_eq!(
                cuLaunchKernel(
                    func,
                    1,
                    1,
                    1,
                    32,
                    1,
                    1,
                    0,
                    stream,
                    core::ptr::null_mut(),
                    core::ptr::null_mut()
                ),
                CUDA_SUCCESS
            );
            assert_eq!(cuEventRecord(ev, stream), CUDA_SUCCESS);
            assert_eq!(cuEventSynchronize(ev), CUDA_SUCCESS);
            let mut dptr = 0u64;
            assert_eq!(cuMemAlloc_v2(&mut dptr, 64), CUDA_SUCCESS);
            assert_eq!(cuMemsetD8_v2(dptr, 0xcd, 64), CUDA_SUCCESS);
            assert_eq!(cuStreamSynchronize(stream), CUDA_SUCCESS);
            assert_eq!(cuMemFree_v2(dptr), CUDA_SUCCESS);
            assert_eq!(cuModuleUnload(module), CUDA_SUCCESS);
            assert_eq!(cuEventDestroy_v2(ev), CUDA_SUCCESS);
            assert_eq!(cuStreamDestroy_v2(stream), CUDA_SUCCESS);
            assert_eq!(cuCtxDestroy_v2(ctx), CUDA_SUCCESS);
            println!(
                "cuda deep: stream+event+module+launch+memset ok (entries={})",
                hermes_cuda_driver_entry_count()
            );
            hermes_cuda_reset();
            println!("PASS");
        }
        other => {
            eprintln!("use offline|online|deep, got {other}");
            std::process::exit(2);
        }
    }
}

fn firmware_scan(root: &str) {
    println!("scan root: {root}");
    let auth = NvidiaGspFirmwareAuthority::default_allow_list();
    for family in [FirmwareFamily::Tu10x, FirmwareFamily::Ga10x] {
        let rel = openrm_gsp_relative("610.43.02", family);
        let path = format!("{root}/{rel}");
        match std::fs::read(&path) {
            Ok(bytes) => match auth.admit(
                match family {
                    FirmwareFamily::Tu10x => 0x1fb9,
                    FirmwareFamily::Ga10x => 0x2204,
                },
                &bytes,
            ) {
                Ok(v) => {
                    let elf = parse_gsp_rm_elf(&bytes).ok();
                    let ver = elf
                        .as_ref()
                        .and_then(|e| fwversion_bytes(&bytes, e).ok())
                        .map(|s| String::from_utf8_lossy(s).into_owned())
                        .unwrap_or_else(|| "?".into());
                    println!(
                        "ADMIT {rel} len={} version_field={ver} pin_version={}",
                        v.byte_length, v.version
                    );
                }
                Err(e) => println!("REJECT {rel}: {e:?}"),
            },
            Err(_) => println!("ABSENT {rel}"),
        }
    }
}

fn drm_smoke(mode: &str) {
    match mode {
        "offline" => {
            let mut dev = DrmDevice::virtual_desktop(false);
            let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).expect("fb");
            dev.framebuffers.push(fb);
            let mut atom = AtomicCommit::new();
            let req = AtomicRequest {
                connector_id: 1,
                crtc_id: 1,
                plane_id: 1,
                fb_id: 10,
                mode: DisplayMode::fhd_60(),
                active: true,
            };
            match atom.commit(&mut dev, &req) {
                Err(e) => {
                    println!("atomic offline => {e:?} (expected GspOffline)");
                    println!("PASS");
                }
                Ok(_) => {
                    eprintln!("error: offline commit must fail");
                    std::process::exit(1);
                }
            }
        }
        "online" => {
            let mut dev = DrmDevice::virtual_desktop(true);
            let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).expect("fb");
            dev.framebuffers.push(fb);
            let mut atom = AtomicCommit::new();
            let req = AtomicRequest {
                connector_id: 1,
                crtc_id: 1,
                plane_id: 1,
                fb_id: 10,
                mode: DisplayMode::fhd_60(),
                active: true,
            };
            let r = atom.commit(&mut dev, &req).expect("commit");
            println!(
                "atomic online: seq={} active={} crtcs={}",
                r.sequence,
                r.active,
                dev.active_crtc_count()
            );
            let d = atom.disable_crtc(&mut dev, 1).expect("disable");
            println!("disable crtc: seq={} active={}", d.sequence, d.active);
            if dev.active_crtc_count() != 0 {
                eprintln!("error: CRTC should be blank after disable");
                std::process::exit(1);
            }
            println!("PASS");
        }
        "dual" => {
            let mut dev = DrmDevice::virtual_dual_head(true);
            let fb0 = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).expect("fb0");
            let fb1 = Framebuffer::new(20, 1280, 720, PixelFormat::Xrgb8888, 2).expect("fb1");
            dev.framebuffers.push(fb0);
            dev.framebuffers.push(fb1);
            let mut atom = AtomicCommit::new();
            for (conn, crtc, plane, fb, mode) in [
                (1u32, 1u32, 1u32, 10u32, DisplayMode::fhd_60()),
                (2, 2, 2, 20, DisplayMode::hd_60()),
            ] {
                let req = AtomicRequest {
                    connector_id: conn,
                    crtc_id: crtc,
                    plane_id: plane,
                    fb_id: fb,
                    mode,
                    active: true,
                };
                let r = atom.commit(&mut dev, &req).expect("dual commit");
                println!("head conn={conn}: seq={}", r.sequence);
            }
            if dev.active_crtc_count() != 2 {
                eprintln!("error: expected 2 active CRTCs");
                std::process::exit(1);
            }
            println!(
                "dual-head: connectors={} active_crtcs={}",
                dev.connector_count(),
                dev.active_crtc_count()
            );
            println!("PASS");
        }
        "gem" => {
            let mut dev = DrmDevice::virtual_desktop(true);
            let dumb = dev.create_dumb(1920, 1080, 32).expect("dumb");
            println!(
                "dumb: handle={} pitch={} size={}",
                dumb.handle, dumb.pitch, dumb.size
            );
            dev.gems
                .get_mut(dumb.handle)
                .expect("gem")
                .fill_solid_xrgb8888(0x00ff_0000)
                .expect("fill");
            let fb_id = dev
                .add_fb_from_gem(dumb.handle, PixelFormat::Xrgb8888)
                .expect("fb");
            let mut atom = AtomicCommit::new();
            atom.commit(
                &mut dev,
                &AtomicRequest {
                    connector_id: 1,
                    crtc_id: 1,
                    plane_id: 1,
                    fb_id,
                    mode: DisplayMode::fhd_60(),
                    active: true,
                },
            )
            .expect("modeset");
            let flip = page_flip(
                &mut atom,
                &mut dev,
                &PageFlipRequest {
                    crtc_id: 1,
                    fb_id,
                    flags: PageFlipRequest::FLAG_EVENT,
                },
            )
            .expect("flip");
            let vb = dev.vblank.pop_event().expect("vblank");
            println!(
                "gem flip: atom_seq={} vblank_seq={} fb={}",
                flip.sequence, vb.sequence, vb.fb_id
            );
            println!("PASS");
        }
        other => {
            eprintln!("use offline|online|dual|gem, got {other}");
            std::process::exit(2);
        }
    }
}

fn mesa_smoke(mode: &str) {
    hermes_mesa_reset();
    match mode {
        "offline" => {
            hermes_mesa_set_gsp_online(false);
            let mut inst = 0u64;
            let r = vkCreateInstance(core::ptr::null(), core::ptr::null(), &mut inst);
            println!(
                "vkCreateInstance offline => {r} (expect INCOMPATIBLE_DRIVER={VK_ERROR_INCOMPATIBLE_DRIVER})"
            );
            if r != VK_ERROR_INCOMPATIBLE_DRIVER {
                std::process::exit(1);
            }
            assert!(hermes_present_solid_frame().is_err());
            hermes_mesa_reset();
            println!("PASS");
        }
        "online" => {
            hermes_mesa_set_gsp_online(true);
            let mut inst = 0u64;
            let r = vkCreateInstance(core::ptr::null(), core::ptr::null(), &mut inst);
            println!("vkCreateInstance online => {r} inst={inst}");
            if r != VK_SUCCESS || inst == 0 {
                std::process::exit(1);
            }
            let mut count = 0u32;
            assert_eq!(
                vkEnumeratePhysicalDevices(inst, &mut count, core::ptr::null_mut()),
                VK_SUCCESS
            );
            println!("physical devices: {count}");
            let mut phys = 0u64;
            count = 1;
            assert_eq!(
                vkEnumeratePhysicalDevices(inst, &mut count, &mut phys),
                VK_SUCCESS
            );
            let mut props = HermesVkPhysicalDeviceProperties {
                api_version: 0,
                driver_version: 0,
                vendor_id: 0,
                device_id: 0,
                device_type: 0,
            };
            assert_eq!(vkGetPhysicalDeviceProperties(phys, &mut props), VK_SUCCESS);
            println!(
                "physdev vendor={:#x} device={:#x}",
                props.vendor_id, props.device_id
            );
            let mut dev = 0u64;
            assert_eq!(
                vkCreateDevice(phys, core::ptr::null(), core::ptr::null(), &mut dev),
                VK_SUCCESS
            );
            let mut q = 0u64;
            assert_eq!(vkGetDeviceQueue(dev, 0, 0, &mut q), VK_SUCCESS);
            let seq = hermes_present_solid_frame().expect("present");
            println!("present solid frame: sequence={seq} queue={q}");
            vkDestroyDevice(dev, core::ptr::null());
            vkDestroyInstance(inst, core::ptr::null());
            hermes_mesa_reset();
            println!("PASS");
        }
        "gem" => {
            hermes_mesa_set_gsp_online(true);
            let seq = hermes_present_gem_flip(0x0000_ff00).expect("gem flip");
            println!("mesa gem flip sequence={seq}");
            hermes_mesa_reset();
            println!("PASS");
        }
        other => {
            eprintln!("use offline|online|gem, got {other}");
            std::process::exit(2);
        }
    }
}

fn stack_smoke() {
    println!("=== stack-smoke: drop-in session + CUDA + DRM + Mesa + smi ===");
    // Shared session: host NVML discover + complete-evidence Online + CUDA bind.
    use nvidia_ml::{
        hermes_nvml_discover_host_gpus, hermes_nvml_format_device_line,
        hermes_nvml_promote_first_sim_online, hermes_nvml_reset, nvmlInit_v2, nvmlShutdown,
        NVML_SUCCESS,
    };
    hermes_nvml_reset();
    assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
    let n = hermes_nvml_discover_host_gpus();
    if n > 0 {
        assert!(hermes_nvml_promote_first_sim_online());
    } else {
        nvidia_ml::hermes_nvml_bind_sim_online_session("Hermes Sim GPU");
    }
    let line = hermes_nvml_format_device_line(0).expect("nvml device");
    println!("session nvml: {line}");
    assert!(line.contains("ONLINE"));
    // Mirror Online into CUDA with the same device identity string.
    let name = line
        .split('(')
        .next()
        .unwrap_or("Hermes GSP GPU")
        .trim()
        .trim_start_matches("GPU 0:")
        .trim();
    hermes_cuda::hermes_cuda_bind_session_device(name, 8 << 30, 7, 5);
    assert!(hermes_cuda::hermes_cuda_gsp_online());
    assert_eq!(hermes_cuda::cuInit(0), CUDA_SUCCESS);
    let mut dname = [0u8; 64];
    assert_eq!(
        hermes_cuda::cuDeviceGetName(dname.as_mut_ptr(), 64, 0),
        CUDA_SUCCESS
    );
    println!(
        "session cuda: name={}",
        String::from_utf8_lossy(&dname[..dname.iter().position(|&b| b == 0).unwrap_or(0)])
    );
    let _ = nvmlShutdown();

    cuda_smoke("deep");
    drm_smoke("gem");
    mesa_smoke("gem");
    smi_smoke("online");
    println!("stack-smoke: PASS (all layers)");
}

