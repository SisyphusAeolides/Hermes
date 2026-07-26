//! Drop-in `nvidia-smi` surface driven by Hermes NVML session state.
//!
//! Discovers host NVIDIA Turing+ devices into NVML, optionally promotes a
//! complete-evidence Online session, and renders tables from real `nvml*`
//! queries — never a static “No devices” string when devices are bound.

use hermes_core::HermesPhase;
// Crate package hermes-nvml exports library name `nvidia_ml`.
use nvidia_ml::{
    hermes_nvml_bind_sim_online_session, hermes_nvml_brand_name, hermes_nvml_discover_host_gpus,
    hermes_nvml_format_device_line, hermes_nvml_format_process_lines, hermes_nvml_gpu_count,
    hermes_nvml_gpu_phase, hermes_nvml_promote_first_sim_online, hermes_nvml_register_process,
    hermes_nvml_reset, nvmlDeviceGetArchitecture, nvmlDeviceGetBrand, nvmlDeviceGetClockInfo,
    nvmlDeviceGetCount_v2, nvmlDeviceGetCudaComputeCapability, nvmlDeviceGetEnforcedPowerLimit,
    nvmlDeviceGetFanSpeed, nvmlDeviceGetHandleByIndex_v2, nvmlDeviceGetMemoryInfo,
    nvmlDeviceGetName, nvmlDeviceGetPCIBusId, nvmlDeviceGetPersistenceMode,
    nvmlDeviceGetPowerUsage, nvmlDeviceGetTemperature, nvmlDeviceGetUtilizationRates,
    nvmlInit_v2, nvmlShutdown, nvmlSystemGetCudaDriverVersion_v2, nvmlSystemGetDriverVersion,
    NvmlMemory_t, NvmlUtilization_t, NVML_CLOCK_GRAPHICS, NVML_CLOCK_MEM, NVML_SUCCESS,
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
    let csv = args.iter().any(|a| a == "--format=csv" || a.starts_with("--format=csv,"));
    let noheader = args.iter().any(|a| a.contains("noheader"))
        || args
            .iter()
            .any(|a| a == "--format=csv,noheader" || a == "--format=csv,noheader,nounits");
    let nounits = args.iter().any(|a| a.contains("nounits"));

    if want_reset {
        hermes_nvml_reset();
    }

    assert_eq!(nvmlInit_v2(), NVML_SUCCESS);

    // Discover live host GPUs into NVML (Offline slots until promoted).
    let discovered = hermes_nvml_discover_host_gpus();
    if want_sim_online && hermes_nvml_gpu_count() > 0 {
        let _ = hermes_nvml_promote_first_sim_online();
        let _ = hermes_nvml_register_process(
            0,
            std::process::id(),
            64 * 1024 * 1024,
            "nvidia-smi",
        );
    } else if want_sim_online && hermes_nvml_gpu_count() == 0 {
        // No host GPU: still allow a sim Online bind for CI/smoke.
        hermes_nvml_bind_sim_online_session("Hermes Sim GPU");
        let _ = hermes_nvml_register_process(
            0,
            std::process::id(),
            64 * 1024 * 1024,
            "nvidia-smi",
        );
    }

    if args.iter().any(|a| a == "-L" || a == "--list-gpus") {
        list_gpus();
        let _ = nvmlShutdown();
        return;
    }

    if let Some(q) = args.iter().find_map(|a| a.strip_prefix("--query-gpu=")) {
        query_gpu_fields(q, QueryFormat {
            csv,
            header: !noheader,
            units: !nounits,
        });
        let _ = nvmlShutdown();
        return;
    }
    if args.iter().any(|a| a == "--query-gpu") {
        query_gpu_fields(
            "name",
            QueryFormat {
                csv,
                header: !noheader,
                units: !nounits,
            },
        );
        let _ = nvmlShutdown();
        return;
    }

    print_summary_table(discovered);
    let _ = nvmlShutdown();
}

#[derive(Clone, Copy)]
struct QueryFormat {
    csv: bool,
    header: bool,
    units: bool,
}

fn print_help() {
    println!(
        "nvidia-smi (Hermes GSP drop-in)\n\n\
         Usage:\n\
           nvidia-smi                 Summary table from NVML session state\n\
           nvidia-smi -L              List GPUs\n\
           nvidia-smi --query-gpu=name,temperature.gpu,fan.speed,power.draw,memory.total,brand\n\
           nvidia-smi --query-gpu=name,fan.speed --format=csv\n\
           nvidia-smi --query-gpu=name --format=csv,noheader,nounits\n\
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

/// `--query-gpu=field1,field2` with optional CSV formatting (classic smi shape).
fn query_gpu_fields(spec: &str, fmt: QueryFormat) {
    let fields: Vec<&str> = spec
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if fields.is_empty() {
        return;
    }
    let sep = if fmt.csv { ", " } else { ", " };
    if fmt.header {
        if fmt.csv {
            // Classic: name [MHz], temperature.gpu [C], ...
            let headers: Vec<String> = fields
                .iter()
                .map(|f| csv_header(f, fmt.units))
                .collect();
            println!("{}", headers.join(sep));
        } else {
            println!("{}", fields.join(sep));
        }
    }
    let mut count = 0u32;
    assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
    for i in 0..count {
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(i, &mut h), NVML_SUCCESS);
        let online = hermes_nvml_gpu_phase(i as usize) == Some(HermesPhase::Online);
        let mut cells = Vec::new();
        for f in &fields {
            cells.push(query_one_field(h, f, online, fmt.units));
        }
        println!("{}", cells.join(sep));
    }
}

fn csv_header(field: &str, units: bool) -> String {
    if !units {
        return field.to_string();
    }
    match field {
        "temperature.gpu" | "temp" => format!("{field} [C]"),
        "fan.speed" | "fan" => format!("{field} [%]"),
        "power.draw" | "power" | "power.limit" => format!("{field} [W]"),
        "memory.total" | "memory.used" | "memory.free" => format!("{field} [MiB]"),
        "utilization.gpu" => format!("{field} [%]"),
        other => other.to_string(),
    }
}

fn query_one_field(h: u64, field: &str, online: bool, units: bool) -> String {
    let na = "[N/A]";
    match field {
        "name" | "gpu_name" => {
            let mut name = [0i8; 96];
            if nvmlDeviceGetName(h, name.as_mut_ptr(), 96) == NVML_SUCCESS {
                cstr_buf(&name)
            } else {
                na.into()
            }
        }
        "pci.bus_id" | "bus_id" => {
            let mut bus = [0i8; 32];
            if nvmlDeviceGetPCIBusId(h, bus.as_mut_ptr(), 32) == NVML_SUCCESS {
                cstr_buf(&bus)
            } else {
                na.into()
            }
        }
        "temperature.gpu" | "temp" => {
            if !online {
                return na.into();
            }
            let mut t = 0u32;
            if nvmlDeviceGetTemperature(h, 0, &mut t) == NVML_SUCCESS {
                if units {
                    format!("{t}")
                } else {
                    format!("{t}")
                }
            } else {
                na.into()
            }
        }
        "fan.speed" | "fan" => {
            if !online {
                return na.into();
            }
            let mut fan = 0u32;
            if nvmlDeviceGetFanSpeed(h, &mut fan) == NVML_SUCCESS {
                format!("{fan}")
            } else {
                na.into()
            }
        }
        "power.draw" | "power" => {
            if !online {
                return na.into();
            }
            let mut mw = 0u32;
            if nvmlDeviceGetPowerUsage(h, &mut mw) == NVML_SUCCESS {
                format!("{:.2}", mw as f64 / 1000.0)
            } else {
                na.into()
            }
        }
        "power.limit" => {
            let mut lim = 0u32;
            if nvmlDeviceGetEnforcedPowerLimit(h, &mut lim) == NVML_SUCCESS {
                format!("{:.0}", lim as f64 / 1000.0)
            } else {
                na.into()
            }
        }
        "memory.total" => {
            let mut mem = NvmlMemory_t {
                total: 0,
                free: 0,
                used: 0,
            };
            if nvmlDeviceGetMemoryInfo(h, &mut mem) == NVML_SUCCESS {
                format!("{}", mem.total / (1024 * 1024))
            } else {
                na.into()
            }
        }
        "memory.used" => {
            let mut mem = NvmlMemory_t {
                total: 0,
                free: 0,
                used: 0,
            };
            if nvmlDeviceGetMemoryInfo(h, &mut mem) == NVML_SUCCESS {
                format!("{}", mem.used / (1024 * 1024))
            } else {
                na.into()
            }
        }
        "memory.free" => {
            let mut mem = NvmlMemory_t {
                total: 0,
                free: 0,
                used: 0,
            };
            if nvmlDeviceGetMemoryInfo(h, &mut mem) == NVML_SUCCESS {
                format!("{}", mem.free / (1024 * 1024))
            } else {
                na.into()
            }
        }
        "utilization.gpu" => {
            if !online {
                return na.into();
            }
            let mut u = NvmlUtilization_t { gpu: 0, memory: 0 };
            if nvmlDeviceGetUtilizationRates(h, &mut u) == NVML_SUCCESS {
                format!("{}", u.gpu)
            } else {
                na.into()
            }
        }
        "brand" => {
            let mut brand = 0u32;
            if nvmlDeviceGetBrand(h, &mut brand) == NVML_SUCCESS {
                hermes_nvml_brand_name(brand).to_string()
            } else {
                na.into()
            }
        }
        "compute_cap" => {
            let mut maj = 0i32;
            let mut min = 0i32;
            if nvmlDeviceGetCudaComputeCapability(h, &mut maj, &mut min) == NVML_SUCCESS {
                format!("{maj}.{min}")
            } else {
                na.into()
            }
        }
        "clocks.current.graphics" | "clocks.gr" => {
            if !online {
                return na.into();
            }
            let mut mhz = 0u32;
            if nvmlDeviceGetClockInfo(h, NVML_CLOCK_GRAPHICS, &mut mhz) == NVML_SUCCESS {
                format!("{mhz}")
            } else {
                na.into()
            }
        }
        "clocks.current.memory" | "clocks.mem" => {
            if !online {
                return na.into();
            }
            let mut mhz = 0u32;
            if nvmlDeviceGetClockInfo(h, NVML_CLOCK_MEM, &mut mhz) == NVML_SUCCESS {
                format!("{mhz}")
            } else {
                na.into()
            }
        }
        "architecture" | "arch" => {
            let mut a = 0u32;
            if nvmlDeviceGetArchitecture(h, &mut a) == NVML_SUCCESS {
                match a {
                    6 => "Turing".into(),
                    7 => "Ampere".into(),
                    8 => "Ada".into(),
                    9 => "Hopper".into(),
                    _ => format!("{a}"),
                }
            } else {
                na.into()
            }
        }
        other => format!("[unknown:{other}]"),
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
        let mut fan_s = "N/A".to_string();
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
            let mut fan = 0u32;
            if nvmlDeviceGetFanSpeed(h, &mut fan) == NVML_SUCCESS {
                fan_s = format!("{fan}%");
            }
        }

        let mut cap_mw = 70_000u32;
        let _ = nvmlDeviceGetEnforcedPowerLimit(h, &mut cap_mw);
        let cap_w = cap_mw / 1000;

        let mut brand = 0u32;
        let brand_s = if nvmlDeviceGetBrand(h, &mut brand) == NVML_SUCCESS {
            hermes_nvml_brand_name(brand)
        } else {
            "?"
        };

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
             | {fan_s:>4} {temp_s:>5}  P0   {pwr_s:>6} /{cap_w:>4}W |   {used_mib:>5}MiB / {total_mib:>5}MiB |   {util_s:>5}      Default |\n\
             |                               |                      |                  N/A |",
            truncate(&name_s, 18),
        );
        if let Some(line) = hermes_nvml_format_device_line(i as usize) {
            println!("| Hermes: {:70} |", truncate(&line, 70));
        }
        println!(
            "| brand={brand_s} sm{maj}.{min} phase={}                                        |",
            phase.label()
        );
    }
    println!("+-------------------------------+----------------------+----------------------+");
    println!();
    println!("+-----------------------------------------------------------------------------+");
    println!("| Processes:                                                                  |");
    println!("|  GPU   GI   CI        PID   Type   Process name                  GPU Memory |");
    println!("|        ID   ID                                                   Usage      |");
    println!("|=============================================================================|");
    let mut any_proc = false;
    for i in 0..count {
        let lines = hermes_nvml_format_process_lines(i as usize);
        for line in lines {
            // Pad to process table shape.
            println!(
                "|    {i}   N/A  N/A  {} |",
                line.trim_start_matches('|').trim()
            );
            any_proc = true;
        }
    }
    if !any_proc {
        println!("|  No running processes found                                                 |");
    }
    println!("+-----------------------------------------------------------------------------+");
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
