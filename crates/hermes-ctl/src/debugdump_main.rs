//! `nvidia-debugdump` drop-in — dumps Hermes session/host facts, not proprietary blobs.

use nvidia_ml::{
    hermes_nvml_discover_host_gpus, hermes_nvml_format_device_line, hermes_nvml_gpu_phase,
    hermes_nvml_reset, nvmlDeviceGetArchitecture, nvmlDeviceGetCount_v2,
    nvmlDeviceGetHandleByIndex_v2, nvmlDeviceGetName, nvmlDeviceGetPCIBusId, nvmlInit_v2,
    nvmlShutdown, NVML_SUCCESS,
};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "nvidia-debugdump (Hermes GSP)\n\
             Usage: nvidia-debugdump [--verbose]\n\
             Prints host NVML census + phase; never invents Online telemetry."
        );
        return;
    }
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");

    hermes_nvml_reset();
    if nvmlInit_v2() != NVML_SUCCESS {
        eprintln!("nvmlInit failed");
        process::exit(1);
    }
    let n = hermes_nvml_discover_host_gpus();
    println!("Hermes debugdump");
    println!("discovered_host_gpus={n}");
    let mut count = 0u32;
    assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
    println!("nvml_count={count}");
    for i in 0..count {
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
        let mut name = [0i8; 96];
        let mut bus = [0i8; 32];
        let _ = nvmlDeviceGetName(h, name.as_mut_ptr(), 96);
        let _ = nvmlDeviceGetPCIBusId(h, bus.as_mut_ptr(), 32);
        let mut arch = 0u32;
        let _ = nvmlDeviceGetArchitecture(h, &mut arch);
        let phase = hermes_nvml_gpu_phase(i as usize)
            .map(|p| p.label())
            .unwrap_or("?");
        println!(
            "GPU{i}: name={} bus={} arch={} phase={}",
            cstr(&name),
            cstr(&bus),
            arch,
            phase
        );
        if verbose {
            if let Some(line) = hermes_nvml_format_device_line(i as usize) {
                println!("  {line}");
            }
        }
    }
    let _ = nvmlShutdown();
    println!("note: dump is session/sysfs based; no proprietary register dump");
}

fn cstr(buf: &[i8]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|&c| c as u8)
        .take_while(|&b| b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}
