//! Drop-in control surface for `nvidia-settings`.
//!
//! Binary is emitted as both `nvidia-settings` and `hermes-settings`.

use hermes_core::{
    HermesManifold, HermesPhase, NVIDIA_VENDOR_ID, admit_display_device, is_nvidia_turing_or_newer,
    nvidia_architecture, pci_identity,
};
use hermes_gsp::{
    FirmwareFamily, firmware_family_for_device, NVIDIA_GSP_RM_610_43_03,
};
use hermes_linux::{MODULE_SURFACES, devices, drop_in_module_name, modules, userspace};

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "--help".into());
    match cmd.as_str() {
        "-v" | "--version" | "version" => {
            println!("nvidia-settings (Hermes GSP) 0.1.0");
            println!("Hermes drop-in control surface for Turing and newer GPUs");
        }
        "-q" | "--query" => {
            let attr = args.next().unwrap_or_else(|| "all".into());
            query(&attr);
        }
        "status" | "--status" => status(),
        "modules" | "--modules" => list_modules(),
        "devices" | "--devices" => list_devices(),
        "compat" | "--compat" => compat_matrix(),
        "-h" | "--help" | "help" => help(),
        other => {
            eprintln!("unknown option: {other}");
            help();
            std::process::exit(2);
        }
    }
}

fn help() {
    println!(
        "nvidia-settings — Hermes GSP drop-in\n\n\
         Usage:\n\
           nvidia-settings --version\n\
           nvidia-settings --status\n\
           nvidia-settings --query <attribute>\n\
           nvidia-settings --modules\n\
           nvidia-settings --devices\n\
           nvidia-settings --compat\n\n\
         Replaces: {}\n\
         Kernel modules: {} {} {} {} {}\n\
         Device nodes: {} {}",
        userspace::NVIDIA_SETTINGS,
        modules::NVIDIA,
        modules::NVIDIA_MODESET,
        modules::NVIDIA_UVM,
        modules::NVIDIA_DRM,
        modules::NVIDIA_PEERMEM,
        devices::NVIDIA_CTL,
        devices::NVIDIA_0,
    );
}

fn status() {
    println!("Hermes GSP control surface: active");
    println!("Drop-in module name: {}", drop_in_module_name(true));
    println!("Phase policy: fail-closed (Online requires firmware+IOMMU+WPR+mailbox+ready)");
    println!("Supported scope: NVIDIA Turing and newer (GSP-RM required)");
    println!("Firmware pin: 610.43.03 ({} manifests)", NVIDIA_GSP_RM_610_43_03.len());
    for m in NVIDIA_GSP_RM_610_43_03 {
        let fam = match m.family {
            FirmwareFamily::Tu10x => "tu10x",
            FirmwareFamily::Ga10x => "ga10x",
        };
        println!("  {fam}: {} bytes", m.byte_length);
    }
    let dark = HermesManifold::dark(0);
    println!("Default manifold phase: {}", dark.phase.label());
    assert_eq!(dark.phase, HermesPhase::Offline);
}

fn query(attr: &str) {
    match attr {
        "all" | "gpus" => {
            println!("Attribute 'gpus' : [no live device bound — host census required]");
            println!("(Hermes does not invent GPUs; bind via hermes-ctl / kernel surface)");
        }
        "ConnectedDisplays" => {
            println!("Attribute 'ConnectedDisplays' : [] (offline)");
        }
        "DigitalVibrance" | "GPULogoBrightness" => {
            println!("Attribute '{attr}' : unavailable (GPU offline)");
        }
        other => {
            println!("Attribute '{other}' : unknown or offline");
        }
    }
}

fn list_modules() {
    for surface in MODULE_SURFACES {
        println!(
            "{:<16} replaces {:<16} — {}",
            surface.name, surface.replaces, surface.description
        );
    }
}

fn list_devices() {
    println!("{}", devices::NVIDIA_CTL);
    println!("{}", devices::NVIDIA_0);
    println!("{}", devices::NVIDIA_UVM);
    println!("{}", devices::NVIDIA_MODESET);
}

fn compat_matrix() {
    let samples = [
        (0x1db6u16, "GV100 Volta"),
        (0x1e04, "TU102 Turing"),
        (0x1fb9, "TU117GLM T1000"),
        (0x2204, "GA102 Ampere"),
        (0x2684, "AD102 Ada"),
        (0x2b85, "GB202 Blackwell"),
    ];
    println!(
        "{:<8} {:<18} {:<8} {:<10} {}",
        "PCI ID", "Name", "Turing+", "FW line", "Admit"
    );
    for (id, name) in samples {
        let turing_plus = is_nvidia_turing_or_newer(id);
        let fam = firmware_family_for_device(id)
            .map(|f| match f {
                FirmwareFamily::Tu10x => "tu10x",
                FirmwareFamily::Ga10x => "ga10x",
            })
            .unwrap_or("-");
        let identity = pci_identity(NVIDIA_VENDOR_ID, id, 0x03, 0x00);
        let admit = admit_display_device(&identity).is_ok();
        let arch = nvidia_architecture(id)
            .map(|a| a.as_str())
            .unwrap_or("n/a");
        println!(
            "{id:#06x}  {name:<18} {turing_plus:<8} {fam:<10} {admit} ({arch})"
        );
    }
}
