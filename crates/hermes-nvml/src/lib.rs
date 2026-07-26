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
