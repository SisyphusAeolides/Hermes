//! NVML-compatible library surface backed by Hermes admission state.
//!
//! Exports the classic `nvml*` entry points used by nvidia-smi and management
//! tools. Until a live GSP session is bound, queries return NOT_FOUND / UNAVAILABLE
//! rather than fabricating device state.

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
pub const NVML_ERROR_UNKNOWN: NvmlReturn = 999;

struct NvmlState {
    initialized: bool,
    /// Bound manifolds per GPU index (empty until host binds devices).
    gpus: Vec<HermesManifold>,
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

/// Initialize NVML (drop-in for nvmlInit_v2).
#[no_mangle]
pub extern "C" fn nvmlInit_v2() -> NvmlReturn {
    with_state(|s| {
        s.initialized = true;
        // Do not invent GPUs. Count stays zero until bind_test_gpu / kernel path.
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
pub extern "C" fn nvmlSystemGetDriverVersion(buffer: *mut i8, length: u32) -> NvmlReturn {
    if buffer.is_null() || length == 0 {
        return NVML_ERROR_INVALID_ARGUMENT;
    }
    let version = b"Hermes-GSP 0.1.0\0";
    if (length as usize) < version.len() {
        return NVML_ERROR_INSUFFICIENT_SIZE;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(version.as_ptr(), buffer as *mut u8, version.len());
    }
    NVML_SUCCESS
}

#[no_mangle]
pub extern "C" fn nvmlSystemGetNVMLVersion(buffer: *mut i8, length: u32) -> NvmlReturn {
    nvmlSystemGetDriverVersion(buffer, length)
}

/// Test/host helper: bind a dark manifold so count can be non-zero without claiming Online.
pub fn hermes_nvml_bind_offline_gpu(generation: u32) {
    with_state(|s| {
        s.initialized = true;
        s.gpus.push(HermesManifold::dark(generation));
    });
}

pub fn hermes_nvml_gpu_phase(index: usize) -> Option<HermesPhase> {
    with_state(|s| s.gpus.get(index).map(|g| g.phase))
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

    #[test]
    fn init_count_shutdown_without_inventing_online() {
        hermes_nvml_reset();
        assert_eq!(nvmlInit_v2(), NVML_SUCCESS);
        let mut count = 99u32;
        assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
        assert_eq!(count, 0, "must not invent GPUs");

        hermes_nvml_bind_offline_gpu(1);
        assert_eq!(nvmlDeviceGetCount_v2(&mut count), NVML_SUCCESS);
        assert_eq!(count, 1);
        assert_eq!(hermes_nvml_gpu_phase(0), Some(HermesPhase::Offline));

        let mut buf = [0i8; 64];
        assert_eq!(nvmlSystemGetDriverVersion(buf.as_mut_ptr(), 64), NVML_SUCCESS);
        assert_eq!(nvmlShutdown(), NVML_SUCCESS);
    }

    #[test]
    fn uninitialized_count_errors() {
        hermes_nvml_reset();
        let mut count = 0u32;
        assert_eq!(
            nvmlDeviceGetCount_v2(&mut count),
            NVML_ERROR_UNINITIALIZED
        );
    }
}
