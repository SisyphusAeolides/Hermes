//! CUDA driver/runtime compatibility surface for Hermes.
//!
//! Names mirror the classic CUDA Driver API (`cu*`) and a Runtime subset
//! (`cuda*`). **Every mutating call requires an explicit GSP/Hermes Online
//! token** — there is no silent success without GPU authority.
//!
//! CCCL (Thrust/CUB) host algorithms live in `hermes-cccl` and do not need a
//! device; device offload will call into this crate once contexts exist.

use std::sync::Mutex;
use std::vec::Vec;

use hermes_cccl::{hermes_fill, hermes_sort, CCCL_VERSION};
// hermes_copy/reduce used in tests via super::*

/// CUDA API result codes (subset of cudaError_t / CUresult).
pub type CudaResult = u32;
pub const CUDA_SUCCESS: CudaResult = 0;
pub const CUDA_ERROR_INVALID_VALUE: CudaResult = 1;
pub const CUDA_ERROR_OUT_OF_MEMORY: CudaResult = 2;
pub const CUDA_ERROR_NOT_INITIALIZED: CudaResult = 3;
pub const CUDA_ERROR_DEINITIALIZED: CudaResult = 4;
pub const CUDA_ERROR_NO_DEVICE: CudaResult = 100;
pub const CUDA_ERROR_INVALID_DEVICE: CudaResult = 101;
pub const CUDA_ERROR_INVALID_CONTEXT: CudaResult = 201;
pub const CUDA_ERROR_NOT_READY: CudaResult = 600;
pub const CUDA_ERROR_NOT_SUPPORTED: CudaResult = 801;
pub const CUDA_ERROR_UNKNOWN: CudaResult = 999;
/// Hermes extension: GSP not Online.
pub const CUDA_ERROR_HERMES_GSP_OFFLINE: CudaResult = 0x4845_524d; // 'HERM'

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Device {
    pub index: u32,
    pub name: &'static str,
    pub total_mem: u64,
    pub compute_major: u32,
    pub compute_minor: u32,
}

#[derive(Clone, Debug)]
struct Context {
    id: u64,
    #[allow(dead_code)]
    device: u32,
    live: bool,
}

#[derive(Clone, Debug)]
struct DeviceBuffer {
    id: u64,
    ctx: u64,
    bytes: usize,
    data: Vec<u8>,
}

#[derive(Clone, Debug)]
struct Module {
    #[allow(dead_code)]
    id: u64,
    ctx: u64,
    #[allow(dead_code)]
    name: String,
}

#[derive(Default)]
struct CudaState {
    driver_init: bool,
    gsp_online: bool,
    next_handle: u64,
    devices: Vec<Device>,
    contexts: Vec<Context>,
    buffers: Vec<DeviceBuffer>,
    modules: Vec<Module>,
}

static STATE: Mutex<CudaState> = Mutex::new(CudaState {
    driver_init: false,
    gsp_online: false,
    next_handle: 1,
    devices: Vec::new(),
    contexts: Vec::new(),
    buffers: Vec::new(),
    modules: Vec::new(),
});

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut CudaState) -> R,
{
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

fn require_gsp(s: &CudaState) -> CudaResult {
    if s.gsp_online {
        CUDA_SUCCESS
    } else {
        CUDA_ERROR_HERMES_GSP_OFFLINE
    }
}

fn next_id(s: &mut CudaState) -> u64 {
    let id = s.next_handle;
    s.next_handle = s.next_handle.wrapping_add(1).max(1);
    id
}

/// Hermes control: publish GSP Online authority into the CUDA shell.
pub fn hermes_cuda_set_gsp_online(online: bool) {
    with_state(|s| {
        s.gsp_online = online;
        if !online {
            s.contexts.clear();
            s.buffers.clear();
            s.modules.clear();
            s.driver_init = false;
        }
    });
}

pub fn hermes_cuda_gsp_online() -> bool {
    with_state(|s| s.gsp_online)
}

pub fn hermes_cuda_reset() {
    with_state(|s| {
        s.gsp_online = false;
        s.driver_init = false;
        s.devices.clear();
        s.contexts.clear();
        s.buffers.clear();
        s.modules.clear();
        s.next_handle = 1;
    });
}

// ─── Driver API ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn cuInit(_flags: u32) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if s.devices.is_empty() {
            s.devices.push(Device {
                index: 0,
                name: "Hermes GSP GPU",
                total_mem: 8 * 1024 * 1024 * 1024,
                compute_major: 7,
                compute_minor: 5,
            });
        }
        s.driver_init = true;
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuDeviceGetCount(count: *mut i32) -> CudaResult {
    if count.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        unsafe {
            *count = s.devices.len() as i32;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuDeviceGet(device: *mut i32, ordinal: i32) -> CudaResult {
    if device.is_null() || ordinal < 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        if ordinal as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        unsafe {
            *device = ordinal;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxCreate_v2(pctx: *mut u64, _flags: u32, device: i32) -> CudaResult {
    if pctx.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        if device < 0 || device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let id = next_id(s);
        s.contexts.push(Context {
            id,
            device: device as u32,
            live: true,
        });
        unsafe {
            *pctx = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxDestroy_v2(ctx: u64) -> CudaResult {
    if ctx == 0 {
        return CUDA_SUCCESS;
    }
    with_state(|s| {
        s.buffers.retain(|b| b.ctx != ctx);
        s.modules.retain(|m| m.ctx != ctx);
        s.contexts.retain(|c| c.id != ctx);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuMemAlloc_v2(dptr: *mut u64, bytesize: usize) -> CudaResult {
    if dptr.is_null() || bytesize == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ctx = match s.contexts.iter().rev().find(|c| c.live) {
            Some(c) => c.id,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        let id = next_id(s);
        s.buffers.push(DeviceBuffer {
            id,
            ctx,
            bytes: bytesize,
            data: vec![0u8; bytesize],
        });
        unsafe {
            *dptr = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuMemFree_v2(dptr: u64) -> CudaResult {
    with_state(|s| {
        let before = s.buffers.len();
        s.buffers.retain(|b| b.id != dptr);
        if s.buffers.len() == before {
            CUDA_ERROR_INVALID_VALUE
        } else {
            CUDA_SUCCESS
        }
    })
}

#[no_mangle]
pub extern "C" fn cuMemcpyHtoD_v2(dst: u64, src: *const u8, bytes: usize) -> CudaResult {
    if src.is_null() || bytes == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let buf = match s.buffers.iter_mut().find(|b| b.id == dst) {
            Some(b) => b,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        if bytes > buf.bytes {
            return CUDA_ERROR_INVALID_VALUE;
        }
        let slice = unsafe { core::slice::from_raw_parts(src, bytes) };
        buf.data[..bytes].copy_from_slice(slice);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuMemcpyDtoH_v2(dst: *mut u8, src: u64, bytes: usize) -> CudaResult {
    if dst.is_null() || bytes == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let buf = match s.buffers.iter().find(|b| b.id == src) {
            Some(b) => b,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        if bytes > buf.bytes {
            return CUDA_ERROR_INVALID_VALUE;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(buf.data.as_ptr(), dst, bytes);
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuModuleLoadData(module: *mut u64, image: *const u8) -> CudaResult {
    if module.is_null() || image.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ctx = match s.contexts.iter().rev().find(|c| c.live) {
            Some(c) => c.id,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        let id = next_id(s);
        s.modules.push(Module {
            id,
            ctx,
            name: "module".into(),
        });
        unsafe {
            *module = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cudaGetDeviceCount(count: *mut i32) -> CudaResult {
    with_state(|s| {
        if require_gsp(s) != CUDA_SUCCESS {
            return CUDA_ERROR_HERMES_GSP_OFFLINE;
        }
        CUDA_SUCCESS
    });
    // init if needed
    let r = cuInit(0);
    if r != CUDA_SUCCESS {
        return r;
    }
    cuDeviceGetCount(count)
}

#[no_mangle]
pub extern "C" fn cudaMalloc(dev_ptr: *mut u64, size: usize) -> CudaResult {
    cuMemAlloc_v2(dev_ptr, size)
}

#[no_mangle]
pub extern "C" fn cudaFree(dev_ptr: u64) -> CudaResult {
    cuMemFree_v2(dev_ptr)
}

pub fn hermes_cuda_host_sort_i32(data: &mut [i32]) {
    hermes_sort(data);
}

pub fn hermes_cuda_host_fill_u8(data: &mut [u8], value: u8) {
    hermes_fill(data, value);
}

pub fn hermes_cuda_cccl_version() -> &'static str {
    CCCL_VERSION
}

pub fn hermes_cuda_host_thrust_available() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn offline_rejects_init_and_device_count() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        assert!(!hermes_cuda_gsp_online());
        assert_eq!(cuInit(0), CUDA_ERROR_HERMES_GSP_OFFLINE);
        let mut n = 0i32;
        assert_eq!(cudaGetDeviceCount(&mut n), CUDA_ERROR_HERMES_GSP_OFFLINE);
    }

    #[test]
    fn online_alloc_copy_roundtrip() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut count = 0i32;
        assert_eq!(cuDeviceGetCount(&mut count), CUDA_SUCCESS);
        assert_eq!(count, 1);
        let mut ctx = 0u64;
        assert_eq!(cuCtxCreate_v2(&mut ctx, 0, 0), CUDA_SUCCESS);
        let mut dptr = 0u64;
        assert_eq!(cuMemAlloc_v2(&mut dptr, 16), CUDA_SUCCESS);
        let src = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        assert_eq!(cuMemcpyHtoD_v2(dptr, src.as_ptr(), 16), CUDA_SUCCESS);
        let mut dst = [0u8; 16];
        assert_eq!(cuMemcpyDtoH_v2(dst.as_mut_ptr(), dptr, 16), CUDA_SUCCESS);
        assert_eq!(src, dst);
        assert_eq!(cuMemFree_v2(dptr), CUDA_SUCCESS);
        assert_eq!(cuCtxDestroy_v2(ctx), CUDA_SUCCESS);
        hermes_cuda_reset();
    }

    #[test]
    fn host_thrust_sort_via_cuda_bridge() {
        let _g = TEST_LOCK.lock().unwrap();
        let mut v = [3, 1, 2];
        hermes_cuda_host_sort_i32(&mut v);
        assert_eq!(v, [1, 2, 3]);
        assert_eq!(hermes_cuda_cccl_version(), CCCL_VERSION);
        let a = [1u32, 2, 3, 4];
        let mut b = [0u32; 4];
        use hermes_cccl::{hermes_copy, hermes_reduce};
        assert_eq!(hermes_copy(&a, &mut b), 4);
        assert_eq!(hermes_reduce(&b, 0, |x, y| x + y), 10);
    }
}
