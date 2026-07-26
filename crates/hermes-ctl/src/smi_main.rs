//! Drop-in `nvidia-smi` surface driven by Hermes NVML session state.
//!
//! Discovers host NVIDIA Turing+ devices into NVML, optionally promotes a
//! complete-evidence Online session, and renders tables from real `nvml*`
//! queries — never a static “No devices” string when devices are bound.

use hermes_core::HermesPhase;
// Crate package hermes-nvml exports library name `nvidia_ml`.
use nvidia_ml::{
    hermes_nvml_bind_sim_online_session, hermes_nvml_discover_host_gpus,
    hermes_nvml_format_device_line, hermes_nvml_gpu_count, hermes_nvml_gpu_phase,
    hermes_nvml_promote_first_sim_online, hermes_nvml_reset, nvmlDeviceGetCount_v2,
    nvmlDeviceGetCudaComputeCapability, nvmlDeviceGetHandleByIndex_v2, nvmlDeviceGetMemoryInfo,
    nvmlDeviceGetName, nvmlDeviceGetPCIBusId, nvmlDeviceGetPersistenceMode,
    nvmlDeviceGetPowerUsage, nvmlDeviceGetTemperature, nvmlDeviceGetUtilizationRates, nvmlInit_v2,
    nvmlShutdown, nvmlSystemGetCudaDriverVersion_v2, nvmlSystemGetDriverVersion, NvmlMemory_t,
    NvmlUtilization_t, NVML_SUCCESS,
};

fn cstr_buf(buf: &[i8]) -> String {
    let bytes: Vec<u8> = buf
        .iter()
        .map(|&c| c as u8)
        .take_while(|&b| b != 0)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return;
    }

    // Session controls (Hermes extensions, not proprietary flags).
    let want_sim_online = args.iter().any(|a| a == "--hermes-sim-online")
        || std::env::var("HERMES_SMI_SIM_ONLINE").ok().as_deref() == Some("1");
    let want_reset = args.iter().any(|a| a == "--hermes-reset");

    if want_reset {
        hermes_nvml_reset();
    }

    assert_eq!(nvmlInit_v2(), NVML_SUCCESS);

    // Discover live host GPUs into NVML (Offline slots until promoted).
    let discovered = hermes_nvml_discover_host_gpus();
    if want_sim_online && hermes_nvml_gpu_count() > 0 {
        let _ = hermes_nvml_promote_first_sim_online();
    } else if want_sim_online && hermes_nvml_gpu_count() == 0 {
        // No host GPU: still allow a sim Online bind for CI/smoke.
        hermes_nvml_bind_sim_online_session("Hermes Sim GPU");
    }

    if args.iter().any(|a| a == "-L" || a == "--list-gpus") {
        list_gpus();
        let _ = nvmlShutdown();
        return;
    }

    if args.iter().any(|a| a == "--query-gpu=name" || a == "--query-gpu") {
        query_names();
        let _ = nvmlShutdown();
        return;
    }

    print_summary_table(discovered);
    let _ = nvmlShutdown();
}

fn print_help() {
    println!(
        "nvidia-smi (Hermes GSP drop-in)\n\n\
         Usage:\n\
           nvidia-smi                 Summary table from NVML session state\n\
           nvidia-smi -L              List GPUs\n\
           nvidia-smi --query-gpu=name\n\
           nvidia-smi --hermes-sim-online   Promote first GPU with complete-evidence Online\n\
           nvidia-smi --hermes-reset\n\n\
         Devices come from host PCI discovery and/or session binds.\n\
         Online telemetry requires a real Online manifold (sim or silicon path)."
    );
}

fn list_gpus() {
    let mut count = 0u32;
    assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
    if count == 0 {
        println!("No devices found.");
        return;
    }
    for i in 0..count {
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
        let mut name = [0i8; 96];
        let mut bus = [0i8; 32];
        assert_eq!(nvmlDeviceGetName(h, name.as_mut_ptr(), 96), NVML_SUCCESS);
        assert_eq!(nvmlDeviceGetPCIBusId(h, bus.as_mut_ptr(), 32), NVML_SUCCESS);
        let phase = hermes_nvml_gpu_phase(i as usize)
            .map(|p| p.label())
            .unwrap_or("?");
        println!(
            "GPU {}: {} (UUID n/a) Bus {} [{}]",
            i,
            cstr_buf(&name),
            cstr_buf(&bus),
            phase
        );
    }
}

fn query_names() {
    let mut count = 0u32;
    assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
    for i in 0..count {
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
        let mut name = [0i8; 96];
        assert_eq!(nvmlDeviceGetName(h, name.as_mut_ptr(), 96), NVML_SUCCESS);
        println!("{}", cstr_buf(&name));
    }
}

fn print_summary_table(discovered: usize) {
    let mut drv = [0i8; 64];
    assert_eq!(nvmlSystemGetDriverVersion(drv.as_mut_ptr(), 64), NVML_SUCCESS);
    let mut cuda_ver = 0i32;
    let _ = nvmlSystemGetCudaDriverVersion_v2(&mut cuda_ver);
    let cuda_s = format!("{}.{}", cuda_ver / 1000, (cuda_ver % 1000) / 10);

    let now = chrono_lite_now();
    println!("{now}");
    println!(
        "+-----------------------------------------------------------------------------+\n\
         | NVIDIA-SMI Hermes-GSP  Driver Version: {:20}  CUDA Version: {:6} |\n\
         +-------------------------------+----------------------+----------------------+\n\
         | GPU  Name        Persistence-M| Bus-Id        Disp.A | Volatile Uncorr. ECC |\n\
         | Fan  Temp  Perf  Pwr:Usage/Cap|         Memory-Usage | GPU-Util  Compute M. |\n\
         |                               |                      |               MIG M. |\n\
         |===============================+======================+======================|",
        cstr_buf(&drv),
        cuda_s
    );

    let mut count = 0u32;
    assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
    if count == 0 {
        println!("| No devices were found                                                         |");
        println!("+-----------------------------------------------------------------------------+\n");
        println!("Note: discovered_host_gpus={discovered} (none bound into NVML this process)");
        return;
    }

    for i in 0..count {
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
        let mut name = [0i8; 64];
        let mut bus = [0i8; 32];
        assert_eq!(nvmlDeviceGetName(h, name.as_mut_ptr(), 64), NVML_SUCCESS);
        assert_eq!(nvmlDeviceGetPCIBusId(h, bus.as_mut_ptr(), 32), NVML_SUCCESS);
        let mut mem = NvmlMemory_t {
            total: 0,
            free: 0,
            used: 0,
        };
        assert_eq!(nvmlDeviceGetMemoryInfo(h, &mut mem), NVML_SUCCESS);
        let mut pers = 0u32;
        let _ = nvmlDeviceGetPersistenceMode(h, &mut pers);
        let phase = hermes_nvml_gpu_phase(i as usize).unwrap_or(HermesPhase::Offline);
        let online = phase == HermesPhase::Online;

        let mut temp_s = "N/A".to_string();
        let mut pwr_s = "N/A".to_string();
        let mut util_s = "N/A".to_string();
        if online {
            let mut t = 0u32;
            if nvmlDeviceGetTemperature(h, 0, &mut t) == NVML_SUCCESS {
                temp_s = format!("{t}C");
            }
            let mut mw = 0u32;
            if nvmlDeviceGetPowerUsage(h, &mut mw) == NVML_SUCCESS {
                pwr_s = format!("{}W", mw / 1000);
            }
            let mut u = NvmlUtilization_t { gpu: 0, memory: 0 };
            if nvmlDeviceGetUtilizationRates(h, &mut u) == NVML_SUCCESS {
                util_s = format!("{}%", u.gpu);
            }
        }

        let mut maj = 0i32;
        let mut min = 0i32;
        let _ = nvmlDeviceGetCudaComputeCapability(h, &mut maj, &mut min);

        let name_s = cstr_buf(&name);
        let bus_s = cstr_buf(&bus);
        let used_mib = mem.used / (1024 * 1024);
        let total_mib = mem.total / (1024 * 1024);
        let pers_s = if pers != 0 { "On" } else { "Off" };

        println!(
            "| {i:>3}  {:18}  {pers_s:>3}  | {bus_s:16}  On |                  N/A |\n\
             | N/A  {temp_s:>5}  P0   {pwr_s:>6} /  70W |   {used_mib:>5}MiB / {total_mib:>5}MiB |   {util_s:>5}      Default |\n\
             |                               |                      |                  N/A |",
            truncate(&name_s, 18),
        );
        if let Some(line) = hermes_nvml_format_device_line(i as usize) {
            println!("| Hermes: {:70} |", truncate(&line, 70));
        }
    }
    println!("+-------------------------------+----------------------+----------------------+");
    println!("Processes: Hermes GSP process list not yet implemented");
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        format!("{s:<n$}")
    } else {
        let t: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

fn chrono_lite_now() -> String {
    // Avoid chrono dep: use UTC-ish from system clock seconds.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("Hermes-SMI epoch={secs}")
}
