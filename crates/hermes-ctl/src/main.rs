//! Hermes GSP control and host inspection.

use hermes_core::{
    HermesManifold, NVIDIA_VENDOR_ID, admit_display_device, is_nvidia_turing_or_newer,
    nvidia_architecture, pci_identity,
};
use hermes_gsp::{
    NVIDIA_GSP_RM_610_43_03, NvidiaGspFirmwareAuthority, default_negotiated_features,
    drive_full_success, firmware_family_for_device, plan_activation, sha256_bytes,
    FirmwareFamily, NvidiaGspFirmwareManifest, firmware_version,
};
use hermes_linux::{MODULE_SURFACES, modules};

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("status") => status(),
        Some("admit") => {
            let id = parse_hex(args.next().as_deref().unwrap_or("0x1fb9"));
            admit_cmd(id);
        }
        Some("test-gates") => test_gates(),
        Some("modules") => {
            for s in MODULE_SURFACES {
                println!("{} -> {}", s.name, s.replaces);
            }
        }
        Some("firmware-pin") => firmware_pin(),
        _ => {
            println!("hermes-ctl — Hermes GSP control\n");
            println!("commands: status | admit <pci_id> | test-gates | modules | firmware-pin");
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
    // Drive the real shipped gate chain.
    let online = drive_full_success(1, 7, default_negotiated_features()).expect("gates");
    assert!(online.is_online());
    println!("gate-chain: ONLINE generation={} domain={}", online.generation, online.evidence.dma_domain);

    // Pre-Turing reject via real admit.
    let volta = pci_identity(NVIDIA_VENDOR_ID, 0x1db6, 0x03, 0x00);
    assert!(admit_display_device(&volta).is_err());
    println!("pre-turing: REJECT 0x1db6");

    // Firmware hash path.
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
    println!("firmware: admitted family={:?} steps={}", plan.firmware_family, plan.steps.len());
    println!("PASS");
}

fn firmware_pin() {
    for m in NVIDIA_GSP_RM_610_43_03 {
        println!("{:?} version={} len={}", m.family, m.version, m.byte_length);
    }
}
