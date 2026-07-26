//! NVML-compatible library surface backed by Hermes admission state.
//!
//! Exports the classic `nvml*` entry points used by nvidia-smi and management
//! tools. Until a live GSP session is bound, queries return NOT_FOUND /
//! UNAVAILABLE rather than fabricating Online device telemetry.

use hermes_core::{HermesManifold, HermesPhase};
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

/// Opaque device handle (index + magic). Classic NVML uses `nvmlDevice_t`.
#[allow(non_camel_case_types)]
pub type NvmlDevice_t = u64;

struct BoundGpu {
    manifold: HermesManifold,
    name: &'static str,
    pci_bus_id: &'static str,
    total_mem_bytes: u64,
    #[allow(dead_code)]
    device_id: u16,
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
        return None;
    }
    Some((h - 1) as usize)
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

/// Initialize NVML (drop-in for nvmlInit_v2).
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
        let mut buf = s.gpus[idx].name.as_bytes().to_vec();
        buf.push(0);
        copy_cstr(&buf, name, length)
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
        let mut buf = s.gpus[idx].pci_bus_id.as_bytes().to_vec();
        buf.push(0);
        copy_cstr(&buf, pci_bus_id, length)
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
        // Offline GPUs report capacity but zero free usable by Hermes policy.
        let online = g.manifold.is_online();
        let total = g.total_mem_bytes;
        let (free, used) = if online {
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
            *temp = 42; // placeholder telemetry only when Online
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
pub extern "C" fn nvmlSystemGetDriverVersion(buffer: *mut i8, length: u32) -> NvmlReturn {
    copy_cstr(b"Hermes-GSP 0.1.0\0", buffer, length)
}

#[no_mangle]
pub extern "C" fn nvmlSystemGetNVMLVersion(buffer: *mut i8, length: u32) -> NvmlReturn {
    copy_cstr(b"12.0-hermes\0", buffer, length)
}

/// Bind Offline GPU (count visible, telemetry gated).
pub fn hermes_nvml_bind_offline_gpu(generation: u32) {
    with_state(|s| {
        s.initialized = true;
        s.gpus.push(BoundGpu {
            manifold: HermesManifold::dark(generation),
            name: "Hermes Offline GPU",
            pci_bus_id: "0000:00:00.0",
            total_mem_bytes: 8 * 1024 * 1024 * 1024,
            device_id: 0x1fb9,
        });
    });
}

/// Bind Online GPU after real GSP Online (tests / host bridge).
pub fn hermes_nvml_bind_online_gpu(manifold: HermesManifold, name: &'static str) {
    with_state(|s| {
        s.initialized = true;
        s.gpus.push(BoundGpu {
            manifold,
            name,
            pci_bus_id: "0000:01:00.0",
            total_mem_bytes: 8 * 1024 * 1024 * 1024,
            device_id: 0x1fb9,
        });
    });
}

pub fn hermes_nvml_gpu_phase(index: usize) -> Option<HermesPhase> {
    with_state(|s| s.gpus.get(index).map(|g| g.manifold.phase))
}

pub fn hermes_nvml_reset() {
    with_state(|s| {
        s.initialized = false;
        s.gpus.clear();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_gsp::{default_negotiated_features, drive_full_success};
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn init_count_shutdown_without_inventing_online() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        hermes_nvml_reset();
        assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
        let mut count = 99u32;
        assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
        assert_eq!(count, 0, "must not invent GPUs");

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
        assert_eq!(mem.free, 0); // offline: no free claim
        let mut temp = 0u32;
        assert_eq!(
            nvmlDeviceGetTemperature(h, 0, &mut temp),
            NVML_ERROR_GPU_IS_LOST
        );

        let mut buf = [0i8; 64];
        assert_eq!(
            nvmlSystemGetDriverVersion(buf.as_mut_ptr(), 64),
            NVML_SUCCESS
        );
        assert_eq!(nvmlShutdown(), NVML_SUCCESS);
    }

    #[test]
    fn online_gpu_exposes_temp() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        hermes_nvml_reset();
        let m = drive_full_success(1, 7, default_negotiated_features()).unwrap();
        hermes_nvml_bind_online_gpu(m, "Hermes T1000");
        let mut h = 0u64;
        assert_eq!(nvmlDeviceGetHandleByIndex_v2(0, &mut h), NVML_SUCCESS);
        let mut temp = 0u32;
        assert_eq!(nvmlDeviceGetTemperature(h, 0, &mut temp), NVML_SUCCESS);
        assert_eq!(temp, 42);
        let mut util = NvmlUtilization_t { gpu: 9, memory: 9 };
        assert_eq!(nvmlDeviceGetUtilizationRates(h, &mut util), NVML_SUCCESS);
        hermes_nvml_reset();
    }

    #[test]
    fn uninitialized_count_errors() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        hermes_nvml_reset();
        let mut count = 0u32;
        assert_eq!(
            nvmlDeviceGetCount_v2(&mut count),
            NVML_ERROR_UNINITIALIZED
        );
    }
}
