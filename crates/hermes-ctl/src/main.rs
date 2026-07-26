//! Hermes GSP control and host inspection.
//! Reports phase from the real shared sequencer (never invents Online).

use hermes_core::{
    HermesManifold, NVIDIA_VENDOR_ID, admit_display_device, is_nvidia_turing_or_newer,
    nvidia_architecture, pci_identity,
};
use hermes_gsp::{
    BringupRequest, FirmwareFamily, HardwareEvidence, NVIDIA_GSP_RM_610_43_02,
    NVIDIA_GSP_RM_610_43_03, NVIDIA_GSP_RM_DEFAULT_ALLOW_LIST, NvidiaGspFirmwareAuthority,
    NvidiaGspFirmwareManifest, chip_gsp_relative, default_negotiated_features, drive_full_success,
    firmware_family_for_device, firmware_version, openrm_gsp_relative, parse_gsp_rm_elf,
    plan_activation, sha256_bytes, NvidiaChipDir, fwversion_bytes,
};
use hermes_linux::{
    MODULE_SURFACES, SimPlatform, linux_bringup, modules, sim_full_hardware,
};
use hermes_nouveau::{
    comparison_matrix, plan_gsp_load, hermes_exclusive_count, NouveauChip,
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
        _ => {
            println!("hermes-ctl — Hermes GSP control\n");
            println!(
                "commands: status | admit <pci_id> | test-gates | bringup <fail|ok|both> | modules | firmware-pin | firmware-scan [root] | nouveau-compare | nouveau-plan <chip> <ver>"
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
        other => {
            eprintln!("unknown bringup mode: {other} (use fail|ok|both)");
            std::process::exit(2);
        }
    }
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

