//! Drop-in control surface for `nvidia-settings`.
//!
//! Queries live Hermes NVML session state (host PCI discovery + Online phase)
//! instead of static empty GPU lists.

use hermes_core::{
    NVIDIA_VENDOR_ID, admit_display_device, is_nvidia_turing_or_newer, nvidia_architecture,
    pci_identity, HermesPhase,
};
use hermes_gsp::{firmware_family_for_device, FirmwareFamily, NVIDIA_GSP_RM_610_43_03};
use hermes_linux::{devices, drop_in_module_name, modules, userspace, MODULE_SURFACES};
use nvidia_ml::{
    hermes_nvml_brand_name, hermes_nvml_discover_host_gpus, hermes_nvml_format_device_line,
    hermes_nvml_gpu_count, hermes_nvml_gpu_phase, hermes_nvml_promote_first_sim_online,
    hermes_nvml_reset, nvmlDeviceGetBrand, nvmlDeviceGetCount_v2, nvmlDeviceGetHandleByIndex_v2,
    nvmlDeviceGetMemoryInfo, nvmlDeviceGetName, nvmlDeviceGetPCIBusId, nvmlInit_v2, nvmlShutdown,
    NvmlMemory_t, NVML_SUCCESS,
};

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
            let sim = std::env::var("HERMES_SETTINGS_SIM_ONLINE").ok().as_deref() == Some("1")
                || args.any(|a| a == "--hermes-sim-online");
            query(&attr, sim);
        }
        "status" | "--status" => status(args.any(|a| a == "--hermes-sim-online")),
        "modules" | "--modules" => list_modules(),
        "devices" | "--devices" => list_devices(),
        "compat" | "--compat" => compat_matrix(),
        "gpus" | "--gpus" => {
            let sim = std::env::var("HERMES_SETTINGS_SIM_ONLINE").ok().as_deref() == Some("1");
            query("gpus", sim);
        }
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
           nvidia-settings --status [--hermes-sim-online]\n\
           nvidia-settings --query <gpus|all|ConnectedDisplays> [--hermes-sim-online]\n\
           nvidia-settings --gpus\n\
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

fn with_nvml_session<F: FnOnce()>(sim_online: bool, f: F) {
    hermes_nvml_reset();
    assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
    let n = hermes_nvml_discover_host_gpus();
    if sim_online {
        if n > 0 {
            let _ = hermes_nvml_promote_first_sim_online();
        } else {
            nvidia_ml::hermes_nvml_bind_sim_online_session("Hermes Sim GPU");
        }
    }
    f();
    let _ = nvmlShutdown();
}

fn status(sim_online: bool) {
    println!("Hermes GSP control surface: active");
    println!("Drop-in module name: {}", drop_in_module_name(true));
    println!("Supported scope: NVIDIA Turing and newer (GSP-RM required)");
    println!(
        "Firmware pin: 610.43.03 ({} manifests)",
        NVIDIA_GSP_RM_610_43_03.len()
    );
    with_nvml_session(sim_online, || {
        let mut count = 0u32;
        assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
        println!("NVML device count: {count}");
        for i in 0..count {
            if let Some(line) = hermes_nvml_format_device_line(i as usize) {
                println!("  {line}");
            }
            let phase = hermes_nvml_gpu_phase(i as usize).unwrap_or(HermesPhase::Offline);
            println!("  phase[{i}]={}", phase.label());
        }
        if count == 0 {
            println!("  (no host Turing+ display GPUs discovered)");
        }
    });
}

fn query(attr: &str, sim_online: bool) {
    match attr {
        "all" | "gpus" => with_nvml_session(sim_online, || {
            let mut count = 0u32;
            assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
            if count == 0 {
                println!("Attribute 'gpus' : []");
                println!("(no bound devices — host census found none)");
                return;
            }
            println!("Attribute 'gpus' : [");
            for i in 0..count {
                let mut h = 0u64;
                assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
                let mut name = [0i8; 96];
                let mut bus = [0i8; 32];
                assert_eq!(nvmlDeviceGetName(h, name.as_mut_ptr(), 96), NVML_SUCCESS);
                assert_eq!(nvmlDeviceGetPCIBusId(h, bus.as_mut_ptr(), 32), NVML_SUCCESS);
                let mut mem = NvmlMemory_t {
                    total: 0,
                    free: 0,
                    used: 0,
                };
                assert_eq!(nvmlDeviceGetMemoryInfo(h, &mut mem), NVML_SUCCESS);
                let mut brand = 0u32;
                let brand_s = if nvmlDeviceGetBrand(h, &mut brand) == NVML_SUCCESS {
                    hermes_nvml_brand_name(brand)
                } else {
                    "?"
                };
                let phase = hermes_nvml_gpu_phase(i as usize)
                    .map(|p| p.label())
                    .unwrap_or("?");
                println!(
                    "  {{ index: {i}, name: \"{}\", bus: \"{}\", phase: \"{phase}\", brand: \"{brand_s}\", mem_mib: {} }}",
                    cstr(&name),
                    cstr(&bus),
                    mem.total / (1024 * 1024)
                );
            }
            println!("]");
            println!("gpu_count={}", hermes_nvml_gpu_count());
        }),
        "ConnectedDisplays" => {
            println!("Attribute 'ConnectedDisplays' : [] (modeset path separate)");
        }
        "DigitalVibrance" | "GPULogoBrightness" => {
            with_nvml_session(sim_online, || {
                let mut count = 0u32;
                assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
                if count == 0 {
                    println!("Attribute '{attr}' : unavailable (no GPU)");
                } else if hermes_nvml_gpu_phase(0) != Some(HermesPhase::Online) {
                    println!("Attribute '{attr}' : unavailable (GPU offline)");
                } else {
                    println!("Attribute '{attr}' : 0 (default online stub)");
                }
            });
        }
        other => {
            println!("Attribute '{other}' : unknown or offline");
        }
    }
}

fn cstr(buf: &[i8]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|&c| c as u8)
        .take_while(|&b| b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
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
        "{:<8} {:<18} {:<8} {:<10} Admit",
        "PCI ID", "Name", "Turing+", "FW line"
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
        println!("{id:#06x}  {name:<18} {turing_plus:<8} {fam:<10} {admit} ({arch})");
    }
}
