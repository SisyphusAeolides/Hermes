//! `nvidia-persistenced` drop-in — persistence mode helper via NVML.
//!
//! Classic daemon keeps the driver loaded. Hermes sets NVML persistence flags
//! on discovered/session GPUs and exits (or stays foreground). Never invents
//! GSP Online.

use nvidia_ml::{
    hermes_nvml_discover_host_gpus, hermes_nvml_reset, nvmlDeviceGetCount_v2,
    nvmlDeviceGetHandleByIndex_v2, nvmlDeviceGetName, nvmlDeviceGetPersistenceMode,
    nvmlDeviceSetPersistenceMode, nvmlInit_v2, nvmlShutdown, NVML_SUCCESS,
};
use std::env;
use std::process;
use std::thread;
use std::time::Duration;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "-v" || a == "--version") {
        println!("nvidia-persistenced (Hermes GSP) 0.1.0");
        return;
    }

    let verbose = args.iter().any(|a| a == "--verbose");
    let disable = args
        .iter()
        .any(|a| a == "--no-persistence-mode" || a == "--disable");
    let mode: u32 = if disable { 0 } else { 1 };
    let foreground = args.iter().any(|a| a == "--foreground" || a == "-f");
    let stay = args
        .iter()
        .find_map(|a| a.strip_prefix("--stay-seconds="))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    hermes_nvml_reset();
    assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
    let discovered = hermes_nvml_discover_host_gpus();
    if verbose {
        eprintln!("discovered host gpus: {discovered}");
    }

    let mut count = 0u32;
    assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
    if count == 0 {
        eprintln!("nvidia-persistenced: no devices in NVML session (discover={discovered})");
        let _ = nvmlShutdown();
        // Classic may still exit 0 when no GPU; Hermes reports honestly with 1.
        process::exit(1);
    }

    for i in 0..count {
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
        let r = nvmlDeviceSetPersistenceMode(h, mode);
        if r != NVML_SUCCESS {
            eprintln!("nvidia-persistenced: set mode failed for GPU {i}: {r}");
            let _ = nvmlShutdown();
            process::exit(1);
        }
        let mut cur = 0u32;
        let _ = nvmlDeviceGetPersistenceMode(h, &mut cur);
        let mut name = [0i8; 64];
        let _ = nvmlDeviceGetName(h, name.as_mut_ptr(), 64);
        let name_s = cstr(&name);
        if verbose || !foreground {
            println!(
                "GPU {i}: persistence={} name={}",
                if cur != 0 { "Enabled" } else { "Disabled" },
                name_s
            );
        }
    }

    if stay > 0 || foreground {
        if verbose {
            eprintln!("staying foreground for {stay}s (0 = one-second foreground hold)");
        }
        let secs = if stay == 0 { 1 } else { stay };
        thread::sleep(Duration::from_secs(secs));
    }

    let _ = nvmlShutdown();
    // Persistence flags remain in process NVML state only for this process;
    // a real daemon would hold the driver open. Hermes does not invent Online.
}

fn cstr(buf: &[i8]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|&c| c as u8)
        .take_while(|&b| b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn print_help() {
    println!(
        "nvidia-persistenced — Hermes drop-in (NVML persistence helper)\n\
         \n\
         Usage: nvidia-persistenced [OPTIONS]\n\
         \n\
           --verbose                 Print per-GPU results\n\
           --no-persistence-mode     Disable persistence (default: enable)\n\
           --disable                 Alias for --no-persistence-mode\n\
           --foreground / -f         Stay in foreground briefly\n\
           --stay-seconds=N          Sleep N seconds before exit\n\
           -v, --version\n\
           -h, --help\n\
         \n\
         Sets NVML persistence mode on discovered GPUs. Does not claim GSP Online."
    );
}
