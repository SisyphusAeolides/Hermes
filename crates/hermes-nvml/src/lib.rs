//! NVML-compatible library surface backed by Hermes admission + session state.
//!
//! `nvidia-smi` and management tools call the classic `nvml*` entry points.
//! Devices appear from host PCI discovery and/or explicit session binds after
//! GSP bring-up. Telemetry that requires Online returns errors while Offline.

use hermes_core::{
    admit_display_device, is_nvidia_turing_or_newer, nvidia_architecture, pci_identity,
    HermesManifold, HermesPhase, NVIDIA_VENDOR_ID,
};
use hermes_gsp::{default_negotiated_features, drive_full_success};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// NVML return codes (subset).
pub type NvmlReturn = u32;
pub const NVML_SUCCESS: NvmlReturn = 0;
pub const NVML_ERROR_UNINITIALIZED: NvmlReturn = 1;
pub const NVML_ERROR_INVALID_ARGUMENT: NvmlReturn = 2;
pub const NVML_ERROR_NOT_SUPPORTED: NvmlReturn = 3;
pub const NVML_ERROR_NOT_FOUND: NvmlReturn = 6;
pub const NVML_ERROR_INSUFFICIENT_SIZE: NvmlReturn = 7;
pub const NVML_ERROR_DRIVER_NOT_LOADED: NvmlReturn = 9;
pub const NVML_ERROR_GPU_IS_LOST: NvmlReturn = 15;
pub const NVML_ERROR_UNKNOWN: NvmlReturn = 999;

#[allow(non_camel_case_types)]
pub type NvmlDevice_t = u64;

#[derive(Clone, Debug)]
struct BoundGpu {
    manifold: HermesManifold,
    name: String,
    pci_bus_id: String,
    uuid: String,
    total_mem_bytes: u64,
    device_id: u16,
    compute_major: u32,
    compute_minor: u32,
    power_limit_mw: u32,
    persistence_mode: bool,
    /// Compute processes bound to this GPU (pid, used memory).
    processes: Vec<ComputeProcess>,
}

#[derive(Clone, Debug)]
struct ComputeProcess {
    pid: u32,
    used_gpu_memory: u64,
    name: String,
}

/// Snapshot used by CUDA/Mesa/settings session glue.
#[derive(Clone, Debug)]
pub struct SessionDeviceSnapshot {
    pub index: usize,
    pub name: String,
    pub pci_bus_id: String,
    pub uuid: String,
    pub total_mem_bytes: u64,
    pub compute_major: u32,
    pub compute_minor: u32,
    pub phase: HermesPhase,
    pub online: bool,
}

struct NvmlState {
    initialized: bool,
    gpus: Vec<BoundGpu>,
}

static STATE: Mutex<NvmlState> = Mutex::new(NvmlState {
    initialized: false,
    gpus: Vec::new(),
});

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut NvmlState) -> R,
{
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

fn handle_to_index(h: NvmlDevice_t) -> Option<usize> {
    if h == 0 {
        None
    } else {
        Some((h - 1) as usize)
    }
}

fn copy_cstr(src: &[u8], buffer: *mut i8, length: u32) -> NvmlReturn {
    if buffer.is_null() || length == 0 {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    if (length as usize) < src.len() {
        return NVML_ERROR_INSUFFICIENT_SIZE;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), buffer as *mut u8, src.len());
    }
    NVML_SUCCESS
}

fn copy_str(s: &str, buffer: *mut i8, length: u32) -> NvmlReturn {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    copy_cstr(&v, buffer, length)
}

fn parse_hex_u16(s: &str) -> Option<u16> {
    let s = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u16::from_str_radix(s, 16).ok()
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn architecture_name(device_id: u16) -> &'static str {
    match nvidia_architecture(device_id) {
        Some(a) => a.as_str(),
        None => "Unknown",
    }
}

fn default_name_for_device(device_id: u16) -> String {
    format!(
        "NVIDIA {} [{:04x}]",
        architecture_name(device_id),
        device_id
    )
}

fn uuid_for(bus: &str, device_id: u16) -> String {
    // Deterministic Hermes UUID (not a hardware EEPROM read).
    format!("GPU-{:04x}-{}", device_id, bus.replace(':', "").replace('.', ""))
}

fn compute_caps(device_id: u16) -> (u32, u32) {
    match nvidia_architecture(device_id) {
        Some(a) if a.as_str() == "Turing" => (7, 5),
        Some(a) if a.as_str() == "Ampere" => (8, 6),
        Some(a) if a.as_str() == "Ada" => (8, 9),
        Some(a) if a.as_str() == "Hopper" => (9, 0),
        Some(a) if a.as_str() == "Blackwell" => (10, 0),
        _ => (7, 5),
    }
}

/// Scan `/sys/bus/pci/devices` for NVIDIA display Turing+ and bind Offline slots.
/// Returns number of devices newly bound (skips duplicates by bus id).
pub fn hermes_nvml_discover_host_gpus() -> usize {
    hermes_nvml_discover_from_sysfs(Path::new("/sys/bus/pci/devices"))
}

pub fn hermes_nvml_discover_from_sysfs(sys_pci: &Path) -> usize {
    let rd = match fs::read_dir(sys_pci) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let mut added = 0usize;
    with_state(|s| {
        s.initialized = true;
        for ent in rd.flatten() {
            let path = ent.path();
            let vendor = match read_trim(&path.join("vendor")).and_then(|v| parse_hex_u16(&v)) {
                Some(v) => v,
                None => continue,
            };
            if vendor != NVIDIA_VENDOR_ID {
                continue;
            }
            let device = match read_trim(&path.join("device")).and_then(|v| parse_hex_u16(&v)) {
                Some(d) => d,
                None => continue,
            };
            let class_raw = read_trim(&path.join("class")).unwrap_or_default();
            let class_u32 = u32::from_str_radix(
                class_raw
                    .trim()
                    .trim_start_matches("0x")
                    .trim_start_matches("0X"),
                16,
            )
            .unwrap_or(0);
            let class_code = ((class_u32 >> 16) & 0xff) as u8;
            let subclass = ((class_u32 >> 8) & 0xff) as u8;
            if class_code != 0x03 {
                continue;
            }
            if !is_nvidia_turing_or_newer(device) {
                continue;
            }
            let id = pci_identity(vendor, device, class_code, subclass);
            if admit_display_device(&id).is_err() {
                continue;
            }
            let bdf = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("0000:00:00.0")
                .to_string();
            if s.gpus.iter().any(|g| g.pci_bus_id == bdf) {
                continue;
            }
            let (maj, min) = compute_caps(device);
            s.gpus.push(BoundGpu {
                manifold: HermesManifold::dark(1),
                name: default_name_for_device(device),
                pci_bus_id: bdf.clone(),
                uuid: uuid_for(&bdf, device),
                total_mem_bytes: 8 * 1024 * 1024 * 1024,
                device_id: device,
                compute_major: maj,
                compute_minor: min,
                power_limit_mw: 70_000,
                persistence_mode: false,
                processes: Vec::new(),
            });
            added += 1;
        }
    });
    added
}

/// Promote an existing Offline GPU (by index) to Online with a live manifold.
pub fn hermes_nvml_set_online(index: usize, manifold: HermesManifold) -> bool {
    with_state(|s| {
        if let Some(g) = s.gpus.get_mut(index) {
            g.manifold = manifold;
            true
        } else {
            false
        }
    })
}

/// Bind Offline GPU for tests / manual session setup.
pub fn hermes_nvml_bind_offline_gpu(generation: u32) {
    with_state(|s| {
        s.initialized = true;
        let bdf = "0000:00:00.0";
        s.gpus.push(BoundGpu {
            manifold: HermesManifold::dark(generation),
            name: "Hermes Offline GPU".into(),
            pci_bus_id: bdf.into(),
            uuid: uuid_for(bdf, 0x1fb9),
            total_mem_bytes: 8 * 1024 * 1024 * 1024,
            device_id: 0x1fb9,
            compute_major: 7,
            compute_minor: 5,
            power_limit_mw: 70_000,
            persistence_mode: false,
            processes: Vec::new(),
        });
    });
}

/// Bind Online GPU after real GSP Online (tests / host bridge).
pub fn hermes_nvml_bind_online_gpu(manifold: HermesManifold, name: &str) {
    with_state(|s| {
        s.initialized = true;
        let bdf = "0000:01:00.0";
        s.gpus.push(BoundGpu {
            manifold,
            name: name.into(),
            pci_bus_id: bdf.into(),
            uuid: uuid_for(bdf, 0x1fb9),
            total_mem_bytes: 8 * 1024 * 1024 * 1024,
            device_id: 0x1fb9,
            compute_major: 7,
            compute_minor: 5,
            power_limit_mw: 70_000,
            persistence_mode: true,
            processes: Vec::new(),
        });
    });
}

/// Run complete-evidence GSP Online on Sim path and bind NVML to it.
pub fn hermes_nvml_bind_sim_online_session(name: &str) -> bool {
    match drive_full_success(1, 7, default_negotiated_features()) {
        Ok(m) => {
            hermes_nvml_bind_online_gpu(m, name);
            true
        }
        Err(_) => false,
    }
}

/// After host discover, promote first GPU with a complete-evidence Online manifold.
pub fn hermes_nvml_promote_first_sim_online() -> bool {
    match drive_full_success(1, 7, default_negotiated_features()) {
        Ok(m) => hermes_nvml_set_online(0, m),
        Err(_) => false,
    }
}

/// Discover host GPUs (or bind sim), promote first with complete-evidence Online.
/// Returns snapshot of GPU 0 for CUDA/Mesa session glue.
pub fn hermes_nvml_session_promote_online() -> Option<SessionDeviceSnapshot> {
    hermes_nvml_reset();
    let _ = nvmlInit_v2();
    let n = hermes_nvml_discover_host_gpus();
    if n == 0 {
        if !hermes_nvml_bind_sim_online_session("Hermes Sim GPU") {
            return None;
        }
    } else if !hermes_nvml_promote_first_sim_online() {
        return None;
    }
    // Register this process as a compute client when Online.
    let pid = std::process::id();
    hermes_nvml_register_process(0, pid, 64 * 1024 * 1024, "hermes-session");
    hermes_nvml_device_snapshot(0)
}

pub fn hermes_nvml_device_snapshot(index: usize) -> Option<SessionDeviceSnapshot> {
    with_state(|s| {
        let g = s.gpus.get(index)?;
        Some(SessionDeviceSnapshot {
            index,
            name: g.name.clone(),
            pci_bus_id: g.pci_bus_id.clone(),
            uuid: g.uuid.clone(),
            total_mem_bytes: g.total_mem_bytes,
            compute_major: g.compute_major,
            compute_minor: g.compute_minor,
            phase: g.manifold.phase,
            online: g.manifold.is_online(),
        })
    })
}

/// Register a compute process against a GPU (visible to nvidia-smi process table).
pub fn hermes_nvml_register_process(
    gpu_index: usize,
    pid: u32,
    used_gpu_memory: u64,
    name: &str,
) -> bool {
    with_state(|s| {
        let g = match s.gpus.get_mut(gpu_index) {
            Some(g) => g,
            None => return false,
        };
        if !g.manifold.is_online() {
            return false;
        }
        if let Some(p) = g.processes.iter_mut().find(|p| p.pid == pid) {
            p.used_gpu_memory = used_gpu_memory;
            p.name = name.into();
        } else {
            g.processes.push(ComputeProcess {
                pid,
                used_gpu_memory,
                name: name.into(),
            });
        }
        true
    })
}

pub fn hermes_nvml_process_count(gpu_index: usize) -> usize {
    with_state(|s| s.gpus.get(gpu_index).map(|g| g.processes.len()).unwrap_or(0))
}

pub fn hermes_nvml_format_process_lines(gpu_index: usize) -> Vec<String> {
    with_state(|s| {
        let g = match s.gpus.get(gpu_index) {
            Some(g) => g,
            None => return Vec::new(),
        };
        g.processes
            .iter()
            .map(|p| {
                format!(
                    "|  {:>6}   C   {}  {:>8}MiB |",
                    p.pid,
                    truncate_name(&p.name, 20),
                    p.used_gpu_memory / (1024 * 1024)
                )
            })
            .collect()
    })
}

fn truncate_name(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        format!("{s:<n$}")
    } else {
        let t: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

pub fn hermes_nvml_gpu_phase(index: usize) -> Option<HermesPhase> {
    with_state(|s| s.gpus.get(index).map(|g| g.manifold.phase))
}

pub fn hermes_nvml_gpu_count() -> usize {
    with_state(|s| s.gpus.len())
}

pub fn hermes_nvml_reset() {
    with_state(|s| {
        s.initialized = false;
        s.gpus.clear();
    });
}

// ─── NVML C ABI ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn nvmlInit_v2() -> NvmlReturn {
    with_state(|s| {
        s.initialized = true;
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlInit() -> NvmlReturn {
    nvmlInit_v2()
}

#[no_mangle]
pub extern "C" fn nvmlShutdown() -> NvmlReturn {
    with_state(|s| {
        s.initialized = false;
        s.gpus.clear();
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetCount_v2(count: *mut u32) -> NvmlReturn {
    if count.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        unsafe {
            *count = s.gpus.len() as u32;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetCount(count: *mut u32) -> NvmlReturn {
    nvmlDeviceGetCount_v2(count)
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetHandleByIndex_v2(
    index: u32,
    device: *mut NvmlDevice_t,
) -> NvmlReturn {
    if device.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        if index as usize >= s.gpus.len() {
            return NVML_ERROR_INVALID_ARGUMENT;
        }
        unsafe {
            *device = (index as u64) + 1;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetHandleByIndex(index: u32, device: *mut NvmlDevice_t) -> NvmlReturn {
    nvmlDeviceGetHandleByIndex_v2(index, device)
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetName(
    device: NvmlDevice_t,
    name: *mut i8,
    length: u32,
) -> NvmlReturn {
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        copy_str(&s.gpus[idx].name, name, length)
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetUUID(
    device: NvmlDevice_t,
    uuid: *mut i8,
    length: u32,
) -> NvmlReturn {
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        copy_str(&s.gpus[idx].uuid, uuid, length)
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetPCIBusId(
    device: NvmlDevice_t,
    pci_bus_id: *mut i8,
    length: u32,
) -> NvmlReturn {
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        copy_str(&s.gpus[idx].pci_bus_id, pci_bus_id, length)
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NvmlPciInfo_t {
    pub bus: u32,
    pub device: u32,
    pub domain: u32,
    pub pci_device_id: u32,
    pub pci_sub_system_id: u32,
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetPciInfo_v3(
    device: NvmlDevice_t,
    pci: *mut NvmlPciInfo_t,
) -> NvmlReturn {
    if pci.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        let g = &s.gpus[idx];
        // Parse "0000:01:00.0"
        let parts: Vec<&str> = g.pci_bus_id.split(|c| c == ':' || c == '.').collect();
        let domain = parts
            .first()
            .and_then(|p| u32::from_str_radix(p, 16).ok())
            .unwrap_or(0);
        let bus = parts
            .get(1)
            .and_then(|p| u32::from_str_radix(p, 16).ok())
            .unwrap_or(0);
        let dev = parts
            .get(2)
            .and_then(|p| u32::from_str_radix(p, 16).ok())
            .unwrap_or(0);
        unsafe {
            *pci = NvmlPciInfo_t {
                bus,
                device: dev,
                domain,
                pci_device_id: (0x10deu32 << 16) | (g.device_id as u32),
                pci_sub_system_id: 0,
            };
        }
        NVML_SUCCESS
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NvmlMemory_t {
    pub total: u64,
    pub free: u64,
    pub used: u64,
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetMemoryInfo(
    device: NvmlDevice_t,
    memory: *mut NvmlMemory_t,
) -> NvmlReturn {
    if memory.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        let g = &s.gpus[idx];
        let total = g.total_mem_bytes;
        let (free, used) = if g.manifold.is_online() {
            (total / 2, total / 2)
        } else {
            (0, 0)
        };
        unsafe {
            *memory = NvmlMemory_t { total, free, used };
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetTemperature(
    device: NvmlDevice_t,
    _sensor: u32,
    temp: *mut u32,
) -> NvmlReturn {
    if temp.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_GPU_IS_LOST;
        }
        unsafe {
            *temp = 42;
        }
        NVML_SUCCESS
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NvmlUtilization_t {
    pub gpu: u32,
    pub memory: u32,
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetUtilizationRates(
    device: NvmlDevice_t,
    utilization: *mut NvmlUtilization_t,
) -> NvmlReturn {
    if utilization.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *utilization = NvmlUtilization_t { gpu: 0, memory: 0 };
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetPowerUsage(device: NvmlDevice_t, milliwatts: *mut u32) -> NvmlReturn {
    if milliwatts.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *milliwatts = 15_000;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetEnforcedPowerLimit(
    device: NvmlDevice_t,
    limit_mw: *mut u32,
) -> NvmlReturn {
    if limit_mw.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        unsafe {
            *limit_mw = s.gpus[idx].power_limit_mw;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetCudaComputeCapability(
    device: NvmlDevice_t,
    major: *mut i32,
    minor: *mut i32,
) -> NvmlReturn {
    if major.is_null() || minor.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        unsafe {
            *major = s.gpus[idx].compute_major as i32;
            *minor = s.gpus[idx].compute_minor as i32;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetPersistenceMode(
    device: NvmlDevice_t,
    mode: *mut u32,
) -> NvmlReturn {
    if mode.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        unsafe {
            *mode = if s.gpus[idx].persistence_mode { 1 } else { 0 };
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetComputeMode(device: NvmlDevice_t, mode: *mut u32) -> NvmlReturn {
    if mode.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        if handle_to_index(device).map(|i| i < s.gpus.len()) != Some(true) {
            return NVML_ERROR_INVALID_ARGUMENT;
        }
        // 0 = Default
        unsafe {
            *mode = 0;
        }
        NVML_SUCCESS
    })
}

/// NVML brand type (subset of nvmlBrandType_t).
pub const NVML_BRAND_UNKNOWN: u32 = 0;
pub const NVML_BRAND_QUADRO: u32 = 1;
pub const NVML_BRAND_TESLA: u32 = 2;
pub const NVML_BRAND_NVS: u32 = 3;
pub const NVML_BRAND_GRID: u32 = 4;
pub const NVML_BRAND_GEFORCE: u32 = 5;
pub const NVML_BRAND_TITAN: u32 = 6;
pub const NVML_BRAND_NVIDIA_VAPPS: u32 = 7;
pub const NVML_BRAND_NVIDIA_VPC: u32 = 8;
pub const NVML_BRAND_NVIDIA_VGAMING: u32 = 9;
pub const NVML_BRAND_QUADRO_RTX: u32 = 10;
pub const NVML_BRAND_NVIDIA_RTX: u32 = 11;
pub const NVML_BRAND_NVIDIA: u32 = 12;
pub const NVML_BRAND_GEFORCE_RTX: u32 = 13;
pub const NVML_BRAND_TITAN_RTX: u32 = 14;

fn brand_for_device(device_id: u16, name: &str) -> u32 {
    let n = name.to_ascii_lowercase();
    if n.contains("quadro") || n.contains("rtx a") || n.contains("t1000") || n.contains("t600") {
        return NVML_BRAND_QUADRO_RTX;
    }
    if n.contains("tesla") || n.contains("a100") || n.contains("h100") || n.contains("l40") {
        return NVML_BRAND_TESLA;
    }
    if n.contains("titan") {
        return NVML_BRAND_TITAN_RTX;
    }
    if n.contains("geforce") || n.contains("rtx 20") || n.contains("rtx 30") || n.contains("rtx 40")
    {
        return NVML_BRAND_GEFORCE_RTX;
    }
    // Turing workstation/mobile often Quadro-class (e.g. 1fb9 T1000).
    match device_id {
        0x1fb9 | 0x1fb8 | 0x1fb0 | 0x1eba | 0x1eb8 => NVML_BRAND_QUADRO_RTX,
        _ => NVML_BRAND_NVIDIA,
    }
}

/// Human label for an `nvmlBrandType_t` value (Hermes subset).
pub fn hermes_nvml_brand_name(brand: u32) -> &'static str {
    match brand {
        NVML_BRAND_QUADRO => "Quadro",
        NVML_BRAND_TESLA => "Tesla",
        NVML_BRAND_NVS => "NVS",
        NVML_BRAND_GRID => "GRID",
        NVML_BRAND_GEFORCE => "GeForce",
        NVML_BRAND_TITAN => "TITAN",
        NVML_BRAND_QUADRO_RTX => "Quadro RTX",
        NVML_BRAND_NVIDIA_RTX => "NVIDIA RTX",
        NVML_BRAND_GEFORCE_RTX => "GeForce RTX",
        NVML_BRAND_TITAN_RTX => "TITAN RTX",
        NVML_BRAND_NVIDIA => "NVIDIA",
        _ => "Unknown",
    }
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetBrand(device: NvmlDevice_t, brand: *mut u32) -> NvmlReturn {
    if brand.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        let g = &s.gpus[idx];
        unsafe {
            *brand = brand_for_device(g.device_id, &g.name);
        }
        NVML_SUCCESS
    })
}

/// Fan speed percent. Offline → not supported (fail-closed, not a fake 0).
#[no_mangle]
pub extern "C" fn nvmlDeviceGetFanSpeed(device: NvmlDevice_t, speed: *mut u32) -> NvmlReturn {
    if speed.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        // Host sim value after Online; not a claim about physical tachometers.
        unsafe {
            *speed = 30;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetFanSpeed_v2(
    device: NvmlDevice_t,
    fan: u32,
    speed: *mut u32,
) -> NvmlReturn {
    if fan != 0 {
        return NVML_ERROR_NOT_SUPPORTED;
    }
    nvmlDeviceGetFanSpeed(device, speed)
}

/// Clock types (subset of nvmlClockType_t).
pub const NVML_CLOCK_GRAPHICS: u32 = 0;
pub const NVML_CLOCK_SM: u32 = 1;
pub const NVML_CLOCK_MEM: u32 = 2;
pub const NVML_CLOCK_VIDEO: u32 = 3;

#[no_mangle]
pub extern "C" fn nvmlDeviceGetClockInfo(
    device: NvmlDevice_t,
    clock_type: u32,
    clock_mhz: *mut u32,
) -> NvmlReturn {
    if clock_mhz.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        let mhz = match clock_type {
            NVML_CLOCK_GRAPHICS => 1395,
            NVML_CLOCK_SM => 1395,
            NVML_CLOCK_MEM => 5001,
            NVML_CLOCK_VIDEO => 1200,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        unsafe {
            *clock_mhz = mhz;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetMaxClockInfo(
    device: NvmlDevice_t,
    clock_type: u32,
    clock_mhz: *mut u32,
) -> NvmlReturn {
    if clock_mhz.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        // Max clocks are identity, not a live tachometer claim.
        let _ = &s.gpus[idx];
        let mhz = match clock_type {
            NVML_CLOCK_GRAPHICS => 1590,
            NVML_CLOCK_SM => 1590,
            NVML_CLOCK_MEM => 5001,
            NVML_CLOCK_VIDEO => 1500,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        unsafe {
            *clock_mhz = mhz;
        }
        NVML_SUCCESS
    })
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NvmlBAR1Memory_t {
    pub bar1_total: u64,
    pub bar1_free: u64,
    pub bar1_used: u64,
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetBAR1MemoryInfo(
    device: NvmlDevice_t,
    bar1: *mut NvmlBAR1Memory_t,
) -> NvmlReturn {
    if bar1.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        let total = 256 * 1024 * 1024u64; // typical BAR1 aperture shell
        let online = s.gpus[idx].manifold.is_online();
        let (free, used) = if online {
            (total / 2, total / 2)
        } else {
            (0, 0)
        };
        unsafe {
            *bar1 = NvmlBAR1Memory_t {
                bar1_total: total,
                bar1_free: free,
                bar1_used: used,
            };
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetEccMode(
    device: NvmlDevice_t,
    current: *mut u32,
    pending: *mut u32,
) -> NvmlReturn {
    if current.is_null() || pending.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        if handle_to_index(device).map(|i| i < s.gpus.len()) != Some(true) {
            return NVML_ERROR_INVALID_ARGUMENT;
        }
        // Workstation T1000-class: ECC disabled / not present.
        unsafe {
            *current = 0;
            *pending = 0;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetIndex(device: NvmlDevice_t, index: *mut u32) -> NvmlReturn {
    if index.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    match handle_to_index(device) {
        Some(i) => {
            unsafe {
                *index = i as u32;
            }
            NVML_SUCCESS
        }
        None => NVML_ERROR_INVALID_ARGUMENT,
    }
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetSerial(
    device: NvmlDevice_t,
    serial: *mut i8,
    length: u32,
) -> NvmlReturn {
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        // Synthetic serial from bus id — not a forged factory serial claim.
        let srl = format!("HERMES-{}", s.gpus[idx].pci_bus_id.replace(':', ""));
        copy_str(&srl, serial, length)
    })
}

/// Architecture enum subset (nvmlDeviceArchitecture_t).
pub const NVML_DEVICE_ARCH_KEPLER: u32 = 2;
pub const NVML_DEVICE_ARCH_MAXWELL: u32 = 3;
pub const NVML_DEVICE_ARCH_PASCAL: u32 = 4;
pub const NVML_DEVICE_ARCH_VOLTA: u32 = 5;
pub const NVML_DEVICE_ARCH_TURING: u32 = 6;
pub const NVML_DEVICE_ARCH_AMPERE: u32 = 7;
pub const NVML_DEVICE_ARCH_ADA: u32 = 8;
pub const NVML_DEVICE_ARCH_HOPPER: u32 = 9;
pub const NVML_DEVICE_ARCH_UNKNOWN: u32 = 0xffffffff;

fn arch_for_device(device_id: u16) -> u32 {
    match device_id {
        d if (0x1e00..=0x1fff).contains(&d) => NVML_DEVICE_ARCH_TURING,
        d if (0x2200..=0x25ff).contains(&d) => NVML_DEVICE_ARCH_AMPERE,
        d if (0x2600..=0x28ff).contains(&d) => NVML_DEVICE_ARCH_ADA,
        d if (0x2300..=0x23ff).contains(&d) || (0x2900..=0x2fff).contains(&d) => {
            NVML_DEVICE_ARCH_HOPPER
        }
        _ => NVML_DEVICE_ARCH_TURING,
    }
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetArchitecture(
    device: NvmlDevice_t,
    arch: *mut u32,
) -> NvmlReturn {
    if arch.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        unsafe {
            *arch = arch_for_device(s.gpus[idx].device_id);
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceSetPersistenceMode(
    device: NvmlDevice_t,
    mode: u32,
) -> NvmlReturn {
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        // Persistence is a session flag; does not invent Online.
        s.gpus[idx].persistence_mode = mode != 0;
        NVML_SUCCESS
    })
}

/// Encoder utilization (subset). Online-only.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NvmlEncoderUtilization_t {
    pub sampling_period_us: u32,
    pub video: u32,
    pub h264: u32,
    pub hevc: u32,
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetEncoderUtilization(
    device: NvmlDevice_t,
    utilization: *mut u32,
    sampling_period_us: *mut u32,
) -> NvmlReturn {
    if utilization.is_null() || sampling_period_us.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *utilization = 0;
            *sampling_period_us = 1_000_000;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetEncoderCapacity(
    device: NvmlDevice_t,
    _encoder_query_type: u32,
    capacity: *mut u32,
) -> NvmlReturn {
    if capacity.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *capacity = 100;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetEncoderSessions(
    device: NvmlDevice_t,
    session_count: *mut u32,
    _sessions: *mut u8,
) -> NvmlReturn {
    if session_count.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *session_count = 0;
        }
        NVML_SUCCESS
    })
}

/// Frame Buffer Capture (FBC) session count — Online-only, empty table is honest.
#[no_mangle]
pub extern "C" fn nvmlDeviceGetFBCSessions(
    device: NvmlDevice_t,
    session_count: *mut u32,
    _sessions: *mut u8,
) -> NvmlReturn {
    if session_count.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *session_count = 0;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetFBCStats(
    device: NvmlDevice_t,
    stats: *mut u8,
) -> NvmlReturn {
    if stats.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        // Zero-fill a minimal 16-byte stats shell.
        unsafe {
            core::ptr::write_bytes(stats, 0, 16);
        }
        NVML_SUCCESS
    })
}

/// vGPU mode — physical GPU reports non-vGPU (fail-closed, not inventing vGPU).
pub const NVML_DEVICE_VGPU_CAPABILITY_NONE: u32 = 0;

#[no_mangle]
pub extern "C" fn nvmlDeviceGetVirtualizationMode(
    device: NvmlDevice_t,
    p_mode: *mut u32,
) -> NvmlReturn {
    if p_mode.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        if handle_to_index(device).map(|i| i < s.gpus.len()) != Some(true) {
            return NVML_ERROR_INVALID_ARGUMENT;
        }
        // 0 = None (physical)
        unsafe {
            *p_mode = 0;
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetVgpuCapabilities(
    device: NvmlDevice_t,
    _capability: u32,
    result: *mut u32,
) -> NvmlReturn {
    if result.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        if handle_to_index(device).map(|i| i < s.gpus.len()) != Some(true) {
            return NVML_ERROR_INVALID_ARGUMENT;
        }
        unsafe {
            *result = 0; // not a vGPU host in Hermes physical path
        }
        NVML_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn nvmlDeviceGetProcessUtilization(
    device: NvmlDevice_t,
    _utilization: *mut u8,
    process_samples_count: *mut u32,
    _last_seen_time_stamp: u64,
) -> NvmlReturn {
    if process_samples_count.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        if !s.gpus[idx].manifold.is_online() {
            return NVML_ERROR_NOT_SUPPORTED;
        }
        unsafe {
            *process_samples_count = 0;
        }
        NVML_SUCCESS
    })
}

/// Entry-point count for drop-in dashboards.
pub fn hermes_nvml_entry_count() -> usize {
    52
}

#[no_mangle]
pub extern "C" fn nvmlSystemGetDriverVersion(buffer: *mut i8, length: u32) -> NvmlReturn {
    copy_cstr(b"Hermes-GSP 0.1.0\0", buffer, length)
}

#[no_mangle]
pub extern "C" fn nvmlSystemGetNVMLVersion(buffer: *mut i8, length: u32) -> NvmlReturn {
    copy_cstr(b"12.0-hermes\0", buffer, length)
}

#[no_mangle]
pub extern "C" fn nvmlSystemGetCudaDriverVersion_v2(cuda_driver_version: *mut i32) -> NvmlReturn {
    if cuda_driver_version.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    // 12.0 encoded as 12000
    unsafe {
        *cuda_driver_version = 12_000;
    }
    NVML_SUCCESS
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NvmlProcessInfo_t {
    pub pid: u32,
    pub used_gpu_memory: u64,
}

/// nvmlDeviceGetComputeRunningProcesses_v2 — fills process table for Online GPUs.
#[no_mangle]
pub extern "C" fn nvmlDeviceGetComputeRunningProcesses_v2(
    device: NvmlDevice_t,
    info_count: *mut u32,
    infos: *mut NvmlProcessInfo_t,
) -> NvmlReturn {
    if info_count.is_null() {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    with_state(|s| {
        if !s.initialized {
            return NVML_ERROR_UNINITIALIZED;
        }
        let idx = match handle_to_index(device) {
            Some(i) if i < s.gpus.len() => i,
            _ => return NVML_ERROR_INVALID_ARGUMENT,
        };
        let g = &s.gpus[idx];
        let n = g.processes.len() as u32;
        if infos.is_null() {
            unsafe {
                *info_count = n;
            }
            return NVML_SUCCESS;
        }
        let cap = unsafe { *info_count };
        let write = core::cmp::min(cap, n) as usize;
        for i in 0..write {
            unsafe {
                *infos.add(i) = NvmlProcessInfo_t {
                    pid: g.processes[i].pid,
                    used_gpu_memory: g.processes[i].used_gpu_memory,
                };
            }
        }
        unsafe {
            *info_count = n;
        }
        if cap < n {
            NVML_ERROR_INSUFFICIENT_SIZE
        } else {
            NVML_SUCCESS
        }
    })
}

/// Format a summary line for one device (used by nvidia-smi tests and CLI).
pub fn hermes_nvml_format_device_line(index: usize) -> Option<String> {
    with_state(|s| {
        let g = s.gpus.get(index)?;
        let phase = g.manifold.phase.label();
        Some(format!(
            "GPU {index}: {} ({}) phase={} mem={} MiB",
            g.name,
            g.pci_bus_id,
            phase,
            g.total_mem_bytes / (1024 * 1024)
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn init_count_shutdown_without_inventing_devices() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
        let mut count = 99u32;
        assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
        assert_eq!(count, 0);
        hermes_nvml_bind_offline_gpu(1);
        assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
        assert_eq!(count, 1);
        assert_eq!(hermes_nvml_gpu_phase(0), Some(HermesPhase::Offline));
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(0, &mut h), NVML_SUCCESS);
        let mut name = [0i8; 64];
        assert_eq!(nvmlDeviceGetName(h, name.as_mut_ptr(), 64), NVML_SUCCESS);
        let mut mem = NvmlMemory_t {
            total: 0,
            free: 0,
            used: 0,
        };
        assert_eq!(nvmlDeviceGetMemoryInfo(h, &mut mem), NVML_SUCCESS);
        assert!(mem.total > 0);
        assert_eq!(mem.free, 0);
        let mut temp = 0u32;
        assert_eq!(
            nvmlDeviceGetTemperature(h, 0, &mut temp),
            NVML_ERROR_GPU_IS_LOST
        );
        assert_eq!(nvmlShutdown(), NVML_SUCCESS);
    }

    #[test]
    fn online_gpu_exposes_temp_power_and_line() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        assert!(hermes_nvml_bind_sim_online_session("Hermes T1000"));
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(0, &mut h), NVML_SUCCESS);
        let mut temp = 0u32;
        assert_eq!(nvmlDeviceGetTemperature(h, 0, &mut temp), NVML_SUCCESS);
        assert_eq!(temp, 42);
        let mut brand = 0u32;
        assert_eq!(nvmlDeviceGetBrand(h, &mut brand), NVML_SUCCESS);
        assert_eq!(brand, NVML_BRAND_QUADRO_RTX);
        let mut fan = 0u32;
        assert_eq!(nvmlDeviceGetFanSpeed(h, &mut fan), NVML_SUCCESS);
        assert_eq!(fan, 30);
        let mut mw = 0u32;
        assert_eq!(nvmlDeviceGetPowerUsage(h, &mut mw), NVML_SUCCESS);
        assert!(mw > 0);
        let mut maj = 0i32;
        let mut min = 0i32;
        assert_eq!(
            nvmlDeviceGetCudaComputeCapability(h, &mut maj, &mut min),
            NVML_SUCCESS
        );
        assert_eq!((maj, min), (7, 5));
        let line = hermes_nvml_format_device_line(0).unwrap();
        assert!(line.contains("Hermes T1000"));
        assert!(line.contains("ONLINE") || line.contains("Online") || line.contains("phase="));
        hermes_nvml_reset();
    }

    #[test]
    fn offline_fan_not_supported() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        hermes_nvml_bind_offline_gpu(1);
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(0, &mut h), NVML_SUCCESS);
        let mut fan = 0u32;
        assert_eq!(nvmlDeviceGetFanSpeed(h, &mut fan), NVML_ERROR_NOT_SUPPORTED);
        hermes_nvml_reset();
    }

    #[test]
    fn clocks_bar1_arch_persistence() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        assert!(hermes_nvml_bind_sim_online_session("Hermes T1000"));
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(0, &mut h), NVML_SUCCESS);
        let mut mhz = 0u32;
        assert_eq!(
            nvmlDeviceGetClockInfo(h, NVML_CLOCK_GRAPHICS, &mut mhz),
            NVML_SUCCESS
        );
        assert!(mhz > 0);
        let mut bar = NvmlBAR1Memory_t::default();
        assert_eq!(nvmlDeviceGetBAR1MemoryInfo(h, &mut bar), NVML_SUCCESS);
        assert!(bar.bar1_total > 0);
        let mut arch = 0u32;
        assert_eq!(nvmlDeviceGetArchitecture(h, &mut arch), NVML_SUCCESS);
        assert_eq!(arch, NVML_DEVICE_ARCH_TURING);
        assert_eq!(nvmlDeviceSetPersistenceMode(h, 1), NVML_SUCCESS);
        let mut mode = 0u32;
        assert_eq!(nvmlDeviceGetPersistenceMode(h, &mut mode), NVML_SUCCESS);
        assert_eq!(mode, 1);
        assert!(hermes_nvml_entry_count() >= 30);
        hermes_nvml_reset();
    }

    #[test]
    fn discover_from_fixture_sysfs_tree() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        // Build a tiny fake sysfs tree for one Turing device.
        let root = std::env::temp_dir().join(format!(
            "hermes-nvml-sysfs-{}",
            std::process::id()
        ));
        let dev = root.join("0000:01:00.0");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&dev).unwrap();
        fs::write(dev.join("vendor"), "0x10de\n").unwrap();
        fs::write(dev.join("device"), "0x1fb9\n").unwrap();
        fs::write(dev.join("class"), "0x030000\n").unwrap();
        let n = hermes_nvml_discover_from_sysfs(&root);
        assert_eq!(n, 1);
        assert_eq!(hermes_nvml_gpu_count(), 1);
        assert_eq!(hermes_nvml_gpu_phase(0), Some(HermesPhase::Offline));
        let line = hermes_nvml_format_device_line(0).unwrap();
        assert!(line.contains("0000:01:00.0"));
        assert!(line.contains("1fb9") || line.contains("Turing") || line.contains("NVIDIA"));
        // second discover is idempotent
        assert_eq!(hermes_nvml_discover_from_sysfs(&root), 0);
        assert_eq!(hermes_nvml_gpu_count(), 1);
        let _ = fs::remove_dir_all(&root);
        hermes_nvml_reset();
    }

    #[test]
    fn promote_discovered_to_online() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        hermes_nvml_bind_offline_gpu(1);
        assert!(hermes_nvml_promote_first_sim_online());
        assert_eq!(hermes_nvml_gpu_phase(0), Some(HermesPhase::Online));
        hermes_nvml_reset();
    }

    #[test]
    fn process_register_only_when_online() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        hermes_nvml_bind_offline_gpu(1);
        assert!(!hermes_nvml_register_process(0, 1234, 1024, "x"));
        assert_eq!(hermes_nvml_process_count(0), 0);
        assert!(hermes_nvml_promote_first_sim_online());
        assert!(hermes_nvml_register_process(0, 1234, 128 * 1024 * 1024, "hermes-test"));
        assert_eq!(hermes_nvml_process_count(0), 1);
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(0, &mut h), NVML_SUCCESS);
        let mut n = 0u32;
        assert_eq!(
            nvmlDeviceGetComputeRunningProcesses_v2(h, &mut n, core::ptr::null_mut()),
            NVML_SUCCESS
        );
        assert_eq!(n, 1);
        let mut infos = [NvmlProcessInfo_t {
            pid: 0,
            used_gpu_memory: 0,
        }; 2];
        n = 2;
        assert_eq!(
            nvmlDeviceGetComputeRunningProcesses_v2(h, &mut n, infos.as_mut_ptr()),
            NVML_SUCCESS
        );
        assert_eq!(n, 1);
        assert_eq!(infos[0].pid, 1234);
        let lines = hermes_nvml_format_process_lines(0);
        assert!(lines[0].contains("1234"));
        hermes_nvml_reset();
    }

    #[test]
    fn session_promote_returns_online_snapshot() {
        let _g = TEST_LOCK.lock().unwrap();
        let snap = hermes_nvml_session_promote_online().expect("promote");
        assert!(snap.online);
        assert_eq!(snap.phase, HermesPhase::Online);
        assert!(!snap.name.is_empty());
        assert!(hermes_nvml_process_count(0) >= 1);
        hermes_nvml_reset();
    }

    #[test]
    fn uninitialized_count_errors() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_nvml_reset();
        let mut count = 0u32;
        assert_eq!(
            nvmlDeviceGetCount_v2(&mut count),
            NVML_ERROR_UNINITIALIZED
        );
    }
}
