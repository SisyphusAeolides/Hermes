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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Device {
    pub index: u32,
    pub name: String,
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
    /// Primary context retained via `cuDevicePrimaryCtxRetain`.
    primary: bool,
    /// Reference count for primary contexts only.
    primary_refs: u32,
}

#[derive(Clone, Debug)]
struct DeviceBuffer {
    id: u64,
    ctx: u64,
    bytes: usize,
    data: Vec<u8>,
}

#[derive(Clone, Debug)]
struct HostBuffer {
    #[allow(dead_code)]
    id: u64,
    /// Boxed so as_mut_ptr stays stable when HostBuffer moves in the Vec.
    data: Box<[u8]>,
}

#[derive(Clone, Debug)]
struct Module {
    id: u64,
    ctx: u64,
    #[allow(dead_code)]
    name: String,
    functions: Vec<String>,
}

#[derive(Clone, Debug)]
struct Function {
    id: u64,
    module: u64,
    #[allow(dead_code)]
    name: String,
}

#[derive(Clone, Debug)]
struct Stream {
    id: u64,
    ctx: u64,
    live: bool,
}

#[derive(Clone, Debug)]
struct Event {
    id: u64,
    recorded_on: Option<u64>,
    completed: bool,
}

#[derive(Default)]
struct CudaState {
    driver_init: bool,
    gsp_online: bool,
    next_handle: u64,
    devices: Vec<Device>,
    contexts: Vec<Context>,
    /// Stack of current contexts (top = `cuCtxGetCurrent`).
    current_stack: Vec<u64>,
    buffers: Vec<DeviceBuffer>,
    host_buffers: Vec<HostBuffer>,
    modules: Vec<Module>,
    functions: Vec<Function>,
    streams: Vec<Stream>,
    events: Vec<Event>,
    /// Enabled peer pairs (device_a, device_b) after EnablePeerAccess.
    peer_enabled: Vec<(u32, u32)>,
}

static STATE: Mutex<CudaState> = Mutex::new(CudaState {
    driver_init: false,
    gsp_online: false,
    next_handle: 1,
    devices: Vec::new(),
    contexts: Vec::new(),
    current_stack: Vec::new(),
    buffers: Vec::new(),
    host_buffers: Vec::new(),
    modules: Vec::new(),
    functions: Vec::new(),
    streams: Vec::new(),
    events: Vec::new(),
    peer_enabled: Vec::new(),
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
            s.current_stack.clear();
            s.buffers.clear();
            s.host_buffers.clear();
            s.modules.clear();
            s.functions.clear();
            s.streams.clear();
            s.events.clear();
            s.peer_enabled.clear();
            s.driver_init = false;
        }
    });
}

pub fn hermes_cuda_gsp_online() -> bool {
    with_state(|s| s.gsp_online)
}

/// Register a device visible after Online `cuInit` (host/session glue).
pub fn hermes_cuda_register_device(
    name: &str,
    total_mem: u64,
    compute_major: u32,
    compute_minor: u32,
) -> u32 {
    with_state(|s| {
        let index = s.devices.len() as u32;
        s.devices.push(Device {
            index,
            name: name.into(),
            total_mem,
            compute_major,
            compute_minor,
        });
        index
    })
}

/// Clear devices then register one Online session GPU and set GSP Online.
pub fn hermes_cuda_bind_session_device(
    name: &str,
    total_mem: u64,
    compute_major: u32,
    compute_minor: u32,
) {
    hermes_cuda_reset();
    hermes_cuda_register_device(name, total_mem, compute_major, compute_minor);
    hermes_cuda_set_gsp_online(true);
}

pub fn hermes_cuda_device_count() -> usize {
    with_state(|s| s.devices.len())
}

pub fn hermes_cuda_reset() {
    with_state(|s| {
        s.gsp_online = false;
        s.driver_init = false;
        s.devices.clear();
        s.contexts.clear();
        s.current_stack.clear();
        s.buffers.clear();
        s.host_buffers.clear();
        s.modules.clear();
        s.functions.clear();
        s.streams.clear();
        s.events.clear();
        s.peer_enabled.clear();
        s.next_handle = 1;
    });
}

/// Register a second (or Nth) Online session GPU for multi-device / peer tests.
pub fn hermes_cuda_register_peer_device(
    name: &str,
    total_mem: u64,
    compute_major: u32,
    compute_minor: u32,
) -> u32 {
    hermes_cuda_register_device(name, total_mem, compute_major, compute_minor)
}

fn live_ctx(s: &CudaState) -> Result<u64, CudaResult> {
    if let Some(&id) = s.current_stack.last() {
        if s.contexts.iter().any(|c| c.id == id && c.live) {
            return Ok(id);
        }
    }
    s.contexts
        .iter()
        .rev()
        .find(|c| c.live)
        .map(|c| c.id)
        .ok_or(CUDA_ERROR_INVALID_CONTEXT)
}

fn push_current(s: &mut CudaState, id: u64) {
    s.current_stack.retain(|&x| x != id);
    s.current_stack.push(id);
}

fn destroy_ctx_resources(s: &mut CudaState, ctx: u64) {
    s.buffers.retain(|b| b.ctx != ctx);
    let mod_ids: Vec<u64> = s
        .modules
        .iter()
        .filter(|m| m.ctx == ctx)
        .map(|m| m.id)
        .collect();
    s.functions.retain(|f| !mod_ids.contains(&f.module));
    s.modules.retain(|m| m.ctx != ctx);
    s.streams.retain(|st| st.ctx != ctx);
    s.contexts.retain(|c| c.id != ctx);
    s.current_stack.retain(|&x| x != ctx);
}

fn device_mem_used(s: &CudaState, device: u32) -> u64 {
    let ctx_ids: Vec<u64> = s
        .contexts
        .iter()
        .filter(|c| c.device == device && c.live)
        .map(|c| c.id)
        .collect();
    s.buffers
        .iter()
        .filter(|b| ctx_ids.contains(&b.ctx))
        .map(|b| b.bytes as u64)
        .sum()
}

// ─── Driver API ───────────────────────────────────────────────────────────

/// Report driver version even before Online (library identity, not device Online).
#[no_mangle]
pub extern "C" fn cuDriverGetVersion(driver_version: *mut i32) -> CudaResult {
    if driver_version.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    // 12.0 encoded as 12000 (matches NVML cuda driver version surface).
    unsafe {
        *driver_version = 12_000;
    }
    CUDA_SUCCESS
}

fn error_name(code: CudaResult) -> &'static str {
    match code {
        CUDA_SUCCESS => "CUDA_SUCCESS",
        CUDA_ERROR_INVALID_VALUE => "CUDA_ERROR_INVALID_VALUE",
        CUDA_ERROR_OUT_OF_MEMORY => "CUDA_ERROR_OUT_OF_MEMORY",
        CUDA_ERROR_NOT_INITIALIZED => "CUDA_ERROR_NOT_INITIALIZED",
        CUDA_ERROR_DEINITIALIZED => "CUDA_ERROR_DEINITIALIZED",
        CUDA_ERROR_NO_DEVICE => "CUDA_ERROR_NO_DEVICE",
        CUDA_ERROR_INVALID_DEVICE => "CUDA_ERROR_INVALID_DEVICE",
        CUDA_ERROR_INVALID_CONTEXT => "CUDA_ERROR_INVALID_CONTEXT",
        CUDA_ERROR_NOT_READY => "CUDA_ERROR_NOT_READY",
        CUDA_ERROR_NOT_SUPPORTED => "CUDA_ERROR_NOT_SUPPORTED",
        CUDA_ERROR_UNKNOWN => "CUDA_ERROR_UNKNOWN",
        CUDA_ERROR_HERMES_GSP_OFFLINE => "CUDA_ERROR_HERMES_GSP_OFFLINE",
        _ => "CUDA_ERROR_UNKNOWN",
    }
}

fn error_string(code: CudaResult) -> &'static str {
    match code {
        CUDA_SUCCESS => "no error",
        CUDA_ERROR_INVALID_VALUE => "invalid argument",
        CUDA_ERROR_OUT_OF_MEMORY => "out of memory",
        CUDA_ERROR_NOT_INITIALIZED => "driver not initialized",
        CUDA_ERROR_DEINITIALIZED => "driver deinitialized",
        CUDA_ERROR_NO_DEVICE => "no CUDA-capable device",
        CUDA_ERROR_INVALID_DEVICE => "invalid device ordinal",
        CUDA_ERROR_INVALID_CONTEXT => "invalid context",
        CUDA_ERROR_NOT_READY => "device not ready",
        CUDA_ERROR_NOT_SUPPORTED => "operation not supported",
        CUDA_ERROR_HERMES_GSP_OFFLINE => "Hermes GSP not Online",
        _ => "unknown error",
    }
}

// Static C strings for cuGetError* (process lifetime).
macro_rules! cstr_static {
    ($s:expr) => {{
        concat!($s, "\0").as_ptr()
    }};
}

#[no_mangle]
pub extern "C" fn cuGetErrorName(error: CudaResult, pstr: *mut *const i8) -> CudaResult {
    if pstr.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    let s = error_name(error);
    // Leak-free: point into static tables via match arms with statics.
    let ptr: *const i8 = match error {
        CUDA_SUCCESS => cstr_static!("CUDA_SUCCESS") as *const i8,
        CUDA_ERROR_INVALID_VALUE => cstr_static!("CUDA_ERROR_INVALID_VALUE") as *const i8,
        CUDA_ERROR_OUT_OF_MEMORY => cstr_static!("CUDA_ERROR_OUT_OF_MEMORY") as *const i8,
        CUDA_ERROR_NOT_INITIALIZED => cstr_static!("CUDA_ERROR_NOT_INITIALIZED") as *const i8,
        CUDA_ERROR_INVALID_CONTEXT => cstr_static!("CUDA_ERROR_INVALID_CONTEXT") as *const i8,
        CUDA_ERROR_NOT_SUPPORTED => cstr_static!("CUDA_ERROR_NOT_SUPPORTED") as *const i8,
        CUDA_ERROR_HERMES_GSP_OFFLINE => cstr_static!("CUDA_ERROR_HERMES_GSP_OFFLINE") as *const i8,
        _ => {
            let _ = s;
            cstr_static!("CUDA_ERROR_UNKNOWN") as *const i8
        }
    };
    unsafe {
        *pstr = ptr;
    }
    CUDA_SUCCESS
}

#[no_mangle]
pub extern "C" fn cuGetErrorString(error: CudaResult, pstr: *mut *const i8) -> CudaResult {
    if pstr.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    let _ = error_string(error);
    let ptr: *const i8 = match error {
        CUDA_SUCCESS => cstr_static!("no error") as *const i8,
        CUDA_ERROR_INVALID_VALUE => cstr_static!("invalid argument") as *const i8,
        CUDA_ERROR_OUT_OF_MEMORY => cstr_static!("out of memory") as *const i8,
        CUDA_ERROR_NOT_INITIALIZED => cstr_static!("driver not initialized") as *const i8,
        CUDA_ERROR_INVALID_CONTEXT => cstr_static!("invalid context") as *const i8,
        CUDA_ERROR_NOT_SUPPORTED => cstr_static!("operation not supported") as *const i8,
        CUDA_ERROR_HERMES_GSP_OFFLINE => cstr_static!("Hermes GSP not Online") as *const i8,
        _ => cstr_static!("unknown error") as *const i8,
    };
    unsafe {
        *pstr = ptr;
    }
    CUDA_SUCCESS
}

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
                name: "Hermes GSP GPU".into(),
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
            primary: false,
            primary_refs: 0,
        });
        push_current(s, id);
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
        if let Some(c) = s.contexts.iter().find(|c| c.id == ctx) {
            if c.primary {
                // Primary contexts are released via PrimaryCtxRelease.
                return CUDA_ERROR_INVALID_CONTEXT;
            }
        } else {
            return CUDA_ERROR_INVALID_CONTEXT;
        }
        destroy_ctx_resources(s, ctx);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxGetCurrent(pctx: *mut u64) -> CudaResult {
    if pctx.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let id = s
            .current_stack
            .last()
            .copied()
            .filter(|&id| s.contexts.iter().any(|c| c.id == id && c.live))
            .unwrap_or(0);
        unsafe {
            *pctx = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxSetCurrent(ctx: u64) -> CudaResult {
    with_state(|s| {
        if ctx == 0 {
            s.current_stack.clear();
            return CUDA_SUCCESS;
        }
        if !s.contexts.iter().any(|c| c.id == ctx && c.live) {
            return CUDA_ERROR_INVALID_CONTEXT;
        }
        push_current(s, ctx);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxGetDevice(device: *mut i32) -> CudaResult {
    if device.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let ctx_id = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        let dev = s
            .contexts
            .iter()
            .find(|c| c.id == ctx_id)
            .map(|c| c.device as i32)
            .unwrap_or(0);
        unsafe {
            *device = dev;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxPushCurrent_v2(ctx: u64) -> CudaResult {
    if ctx == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.contexts.iter().any(|c| c.id == ctx && c.live) {
            return CUDA_ERROR_INVALID_CONTEXT;
        }
        s.current_stack.push(ctx);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxPopCurrent_v2(pctx: *mut u64) -> CudaResult {
    with_state(|s| {
        let id = match s.current_stack.pop() {
            Some(id) => id,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        if !pctx.is_null() {
            unsafe {
                *pctx = id;
            }
        }
        CUDA_SUCCESS
    })
}

/// Retain the primary context for `device` (creates on first retain).
#[no_mangle]
pub extern "C" fn cuDevicePrimaryCtxRetain(pctx: *mut u64, device: i32) -> CudaResult {
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
        let dev = device as u32;
        let existing = s
            .contexts
            .iter()
            .find(|c| c.device == dev && c.primary && c.live)
            .map(|c| c.id);
        if let Some(id) = existing {
            if let Some(c) = s.contexts.iter_mut().find(|c| c.id == id) {
                c.primary_refs = c.primary_refs.saturating_add(1);
            }
            push_current(s, id);
            unsafe {
                *pctx = id;
            }
            return CUDA_SUCCESS;
        }
        let id = next_id(s);
        s.contexts.push(Context {
            id,
            device: dev,
            live: true,
            primary: true,
            primary_refs: 1,
        });
        push_current(s, id);
        unsafe {
            *pctx = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuDevicePrimaryCtxRelease(device: i32) -> CudaResult {
    with_state(|s| {
        if device < 0 || device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let dev = device as u32;
        let ctx_id = match s
            .contexts
            .iter()
            .find(|c| c.device == dev && c.primary && c.live)
            .map(|c| c.id)
        {
            Some(id) => id,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        let refs = {
            let c = s.contexts.iter_mut().find(|c| c.id == ctx_id).unwrap();
            c.primary_refs = c.primary_refs.saturating_sub(1);
            c.primary_refs
        };
        if refs == 0 {
            destroy_ctx_resources(s, ctx_id);
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuDevicePrimaryCtxGetState(
    device: i32,
    flags: *mut u32,
    active: *mut i32,
) -> CudaResult {
    if flags.is_null() || active.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        if device < 0 || device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let dev = device as u32;
        let is_active = s
            .contexts
            .iter()
            .any(|c| c.device == dev && c.primary && c.live && c.primary_refs > 0);
        unsafe {
            *flags = 0;
            *active = if is_active { 1 } else { 0 };
        }
        CUDA_SUCCESS
    })
}

/// Device attributes (subset of CUdevice_attribute).
pub const CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_BLOCK: i32 = 1;
pub const CU_DEVICE_ATTRIBUTE_MAX_BLOCK_DIM_X: i32 = 2;
pub const CU_DEVICE_ATTRIBUTE_MAX_BLOCK_DIM_Y: i32 = 3;
pub const CU_DEVICE_ATTRIBUTE_MAX_BLOCK_DIM_Z: i32 = 4;
pub const CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_X: i32 = 5;
pub const CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_Y: i32 = 6;
pub const CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_Z: i32 = 7;
pub const CU_DEVICE_ATTRIBUTE_MAX_SHARED_MEMORY_PER_BLOCK: i32 = 8;
pub const CU_DEVICE_ATTRIBUTE_TOTAL_CONSTANT_MEMORY: i32 = 9;
pub const CU_DEVICE_ATTRIBUTE_WARP_SIZE: i32 = 10;
pub const CU_DEVICE_ATTRIBUTE_MAX_PITCH: i32 = 11;
pub const CU_DEVICE_ATTRIBUTE_MAX_REGISTERS_PER_BLOCK: i32 = 12;
pub const CU_DEVICE_ATTRIBUTE_CLOCK_RATE: i32 = 13;
pub const CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT: i32 = 16;
pub const CU_DEVICE_ATTRIBUTE_INTEGRATED: i32 = 18;
pub const CU_DEVICE_ATTRIBUTE_CAN_MAP_HOST_MEMORY: i32 = 19;
pub const CU_DEVICE_ATTRIBUTE_COMPUTE_MODE: i32 = 20;
pub const CU_DEVICE_ATTRIBUTE_CONCURRENT_KERNELS: i32 = 31;
pub const CU_DEVICE_ATTRIBUTE_PCI_BUS_ID: i32 = 33;
pub const CU_DEVICE_ATTRIBUTE_PCI_DEVICE_ID: i32 = 34;
pub const CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE: i32 = 36;
pub const CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH: i32 = 37;
pub const CU_DEVICE_ATTRIBUTE_L2_CACHE_SIZE: i32 = 38;
pub const CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_MULTIPROCESSOR: i32 = 39;
pub const CU_DEVICE_ATTRIBUTE_ASYNC_ENGINE_COUNT: i32 = 40;
pub const CU_DEVICE_ATTRIBUTE_UNIFIED_ADDRESSING: i32 = 41;
pub const CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR: i32 = 75;
pub const CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR: i32 = 76;

#[no_mangle]
pub extern "C" fn cuDeviceGetAttribute(
    pi: *mut i32,
    attrib: i32,
    device: i32,
) -> CudaResult {
    if pi.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        if device < 0 || device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let dev = &s.devices[device as usize];
        let val = match attrib {
            CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_BLOCK => 1024,
            CU_DEVICE_ATTRIBUTE_MAX_BLOCK_DIM_X => 1024,
            CU_DEVICE_ATTRIBUTE_MAX_BLOCK_DIM_Y => 1024,
            CU_DEVICE_ATTRIBUTE_MAX_BLOCK_DIM_Z => 64,
            CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_X => 2_147_483_647,
            CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_Y => 65535,
            CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_Z => 65535,
            CU_DEVICE_ATTRIBUTE_MAX_SHARED_MEMORY_PER_BLOCK => 49152,
            CU_DEVICE_ATTRIBUTE_TOTAL_CONSTANT_MEMORY => 65536,
            CU_DEVICE_ATTRIBUTE_WARP_SIZE => 32,
            CU_DEVICE_ATTRIBUTE_MAX_PITCH => 2_147_483_647,
            CU_DEVICE_ATTRIBUTE_MAX_REGISTERS_PER_BLOCK => 65536,
            CU_DEVICE_ATTRIBUTE_CLOCK_RATE => 1_395_000, // kHz-class host sim
            CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT => 40,
            CU_DEVICE_ATTRIBUTE_INTEGRATED => 0,
            CU_DEVICE_ATTRIBUTE_CAN_MAP_HOST_MEMORY => 1,
            CU_DEVICE_ATTRIBUTE_COMPUTE_MODE => 0, // default
            CU_DEVICE_ATTRIBUTE_CONCURRENT_KERNELS => 1,
            CU_DEVICE_ATTRIBUTE_PCI_BUS_ID => 3,
            CU_DEVICE_ATTRIBUTE_PCI_DEVICE_ID => 0,
            CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE => 5_001_000,
            CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH => 256,
            CU_DEVICE_ATTRIBUTE_L2_CACHE_SIZE => 4 * 1024 * 1024,
            CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_MULTIPROCESSOR => 1024,
            CU_DEVICE_ATTRIBUTE_ASYNC_ENGINE_COUNT => 3,
            CU_DEVICE_ATTRIBUTE_UNIFIED_ADDRESSING => 1,
            CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR => dev.compute_major as i32,
            CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR => dev.compute_minor as i32,
            _ => return CUDA_ERROR_INVALID_VALUE,
        };
        unsafe {
            *pi = val;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuDeviceGetName(name: *mut u8, len: i32, device: i32) -> CudaResult {
    if name.is_null() || len <= 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        if device < 0 || device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let src = s.devices[device as usize].name.as_bytes();
        let n = core::cmp::min(src.len(), (len as usize).saturating_sub(1));
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), name, n);
            *name.add(n) = 0;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuDeviceTotalMem_v2(bytes: *mut u64, device: i32) -> CudaResult {
    if bytes.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        if !s.driver_init {
            return CUDA_ERROR_NOT_INITIALIZED;
        }
        if device < 0 || device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        unsafe {
            *bytes = s.devices[device as usize].total_mem;
        }
        CUDA_SUCCESS
    })
}

/// Peer access capability: same Online session, distinct devices → yes (software).
#[no_mangle]
pub extern "C" fn cuDeviceCanAccessPeer(
    can_access: *mut i32,
    device: i32,
    peer_device: i32,
) -> CudaResult {
    if can_access.is_null() {
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
        if device < 0
            || peer_device < 0
            || device as usize >= s.devices.len()
            || peer_device as usize >= s.devices.len()
        {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let ok = device != peer_device;
        unsafe {
            *can_access = if ok { 1 } else { 0 };
        }
        CUDA_SUCCESS
    })
}

/// Enable peer access from the current context's device to `peer_device`.
#[no_mangle]
pub extern "C" fn cuCtxEnablePeerAccess(peer_device: i32, _flags: u32) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ctx_id = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        let device = match s.contexts.iter().find(|c| c.id == ctx_id) {
            Some(c) => c.device,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        if peer_device < 0 || peer_device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        if peer_device as u32 == device {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let pair = (device, peer_device as u32);
        if !s.peer_enabled.contains(&pair) {
            s.peer_enabled.push(pair);
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuCtxDisablePeerAccess(peer_device: i32) -> CudaResult {
    with_state(|s| {
        let ctx_id = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        let device = match s.contexts.iter().find(|c| c.id == ctx_id) {
            Some(c) => c.device,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        let before = s.peer_enabled.len();
        s.peer_enabled
            .retain(|(a, b)| !(*a == device && *b == peer_device as u32));
        if s.peer_enabled.len() == before {
            CUDA_ERROR_INVALID_VALUE
        } else {
            CUDA_SUCCESS
        }
    })
}

/// Hermes helper: true when peer access is enabled for (device, peer).
pub fn hermes_cuda_peer_enabled(device: u32, peer: u32) -> bool {
    with_state(|s| s.peer_enabled.contains(&(device, peer)))
}

/// Page-locked host allocation (software pin — stable Box heap pointer).
#[no_mangle]
pub extern "C" fn cuMemHostAlloc(pp: *mut *mut u8, bytesize: usize, _flags: u32) -> CudaResult {
    if pp.is_null() || bytesize == 0 {
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
        let id = next_id(s);
        let mut data = vec![0u8; bytesize].into_boxed_slice();
        let ptr = data.as_mut_ptr();
        s.host_buffers.push(HostBuffer { id, data });
        unsafe {
            *pp = ptr;
        }
        let _ = id;
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuMemFreeHost(p: *mut u8) -> CudaResult {
    if p.is_null() {
        return CUDA_SUCCESS;
    }
    with_state(|s| {
        let before = s.host_buffers.len();
        s.host_buffers.retain(|b| b.data.as_ptr() as *mut u8 != p);
        if s.host_buffers.len() == before {
            CUDA_ERROR_INVALID_VALUE
        } else {
            CUDA_SUCCESS
        }
    })
}

/// Peer device-to-device copy (requires EnablePeerAccess when multi-device).
#[no_mangle]
pub extern "C" fn cuMemcpyPeer(
    dst: u64,
    dst_device: i32,
    src: u64,
    src_device: i32,
    bytes: usize,
) -> CudaResult {
    if bytes == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if src_device < 0
            || dst_device < 0
            || src_device as usize >= s.devices.len()
            || dst_device as usize >= s.devices.len()
        {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        if src_device != dst_device
            && !s
                .peer_enabled
                .contains(&(src_device as u32, dst_device as u32))
            && !s
                .peer_enabled
                .contains(&(dst_device as u32, src_device as u32))
        {
            // Same-device always ok; cross-device needs peer enable.
            return CUDA_ERROR_NOT_SUPPORTED;
        }
        let src_idx = match s.buffers.iter().position(|b| b.id == src) {
            Some(i) => i,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        let dst_idx = match s.buffers.iter().position(|b| b.id == dst) {
            Some(i) => i,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        if bytes > s.buffers[src_idx].bytes || bytes > s.buffers[dst_idx].bytes {
            return CUDA_ERROR_INVALID_VALUE;
        }
        let tmp = s.buffers[src_idx].data[..bytes].to_vec();
        s.buffers[dst_idx].data[..bytes].copy_from_slice(&tmp);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cudaHostAlloc(p_host: *mut *mut u8, size: usize, flags: u32) -> CudaResult {
    cuMemHostAlloc(p_host, size, flags)
}

#[no_mangle]
pub extern "C" fn cudaFreeHost(ptr: *mut u8) -> CudaResult {
    cuMemFreeHost(ptr)
}

#[no_mangle]
pub extern "C" fn cudaDeviceCanAccessPeer(
    can_access: *mut i32,
    device: i32,
    peer_device: i32,
) -> CudaResult {
    cuDeviceCanAccessPeer(can_access, device, peer_device)
}

#[no_mangle]
pub extern "C" fn cudaDeviceEnablePeerAccess(peer_device: i32, flags: u32) -> CudaResult {
    cuCtxEnablePeerAccess(peer_device, flags)
}

#[no_mangle]
pub extern "C" fn cudaDeviceDisablePeerAccess(peer_device: i32) -> CudaResult {
    cuCtxDisablePeerAccess(peer_device)
}

#[no_mangle]
pub extern "C" fn cudaMemcpyPeer(
    dst: u64,
    dst_device: i32,
    src: u64,
    src_device: i32,
    count: usize,
) -> CudaResult {
    cuMemcpyPeer(dst, dst_device, src, src_device, count)
}

#[no_mangle]
pub extern "C" fn cuMemGetInfo_v2(free: *mut u64, total: *mut u64) -> CudaResult {
    if free.is_null() || total.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ctx_id = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        let device = match s.contexts.iter().find(|c| c.id == ctx_id) {
            Some(c) => c.device,
            None => return CUDA_ERROR_INVALID_CONTEXT,
        };
        if device as usize >= s.devices.len() {
            return CUDA_ERROR_INVALID_DEVICE;
        }
        let tot = s.devices[device as usize].total_mem;
        let used = device_mem_used(s, device);
        let free_b = tot.saturating_sub(used);
        unsafe {
            *free = free_b;
            *total = tot;
        }
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
        let ctx = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        let device = s
            .contexts
            .iter()
            .find(|c| c.id == ctx)
            .map(|c| c.device)
            .unwrap_or(0);
        if (device as usize) < s.devices.len() {
            let tot = s.devices[device as usize].total_mem;
            let used = device_mem_used(s, device);
            if used.saturating_add(bytesize as u64) > tot {
                return CUDA_ERROR_OUT_OF_MEMORY;
            }
        }
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
pub extern "C" fn cuMemcpyDtoD_v2(dst: u64, src: u64, bytes: usize) -> CudaResult {
    if bytes == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let src_idx = match s.buffers.iter().position(|b| b.id == src) {
            Some(i) => i,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        let dst_idx = match s.buffers.iter().position(|b| b.id == dst) {
            Some(i) => i,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        if bytes > s.buffers[src_idx].bytes || bytes > s.buffers[dst_idx].bytes {
            return CUDA_ERROR_INVALID_VALUE;
        }
        // Split borrows via clone of source slice.
        let tmp = s.buffers[src_idx].data[..bytes].to_vec();
        s.buffers[dst_idx].data[..bytes].copy_from_slice(&tmp);
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuMemsetD8_v2(dst: u64, value: u8, n: usize) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let buf = match s.buffers.iter_mut().find(|b| b.id == dst) {
            Some(b) => b,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        if n > buf.bytes {
            return CUDA_ERROR_INVALID_VALUE;
        }
        buf.data[..n].fill(value);
        CUDA_SUCCESS
    })
}

fn validate_async_stream(hstream: u64) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if hstream != 0 && !s.streams.iter().any(|st| st.id == hstream && st.live) {
            return CUDA_ERROR_INVALID_VALUE;
        }
        CUDA_SUCCESS
    })
}

/// Async HtoD — host sim completes immediately (stream validated).
#[no_mangle]
pub extern "C" fn cuMemcpyHtoDAsync_v2(
    dst: u64,
    src: *const u8,
    bytes: usize,
    hstream: u64,
) -> CudaResult {
    let r = validate_async_stream(hstream);
    if r != CUDA_SUCCESS {
        return r;
    }
    cuMemcpyHtoD_v2(dst, src, bytes)
}

#[no_mangle]
pub extern "C" fn cuMemcpyDtoHAsync_v2(
    dst: *mut u8,
    src: u64,
    bytes: usize,
    hstream: u64,
) -> CudaResult {
    let r = validate_async_stream(hstream);
    if r != CUDA_SUCCESS {
        return r;
    }
    cuMemcpyDtoH_v2(dst, src, bytes)
}

#[no_mangle]
pub extern "C" fn cuMemcpyDtoDAsync_v2(
    dst: u64,
    src: u64,
    bytes: usize,
    hstream: u64,
) -> CudaResult {
    let r = validate_async_stream(hstream);
    if r != CUDA_SUCCESS {
        return r;
    }
    cuMemcpyDtoD_v2(dst, src, bytes)
}

/// Load module with explicit image size (fatbin / cubin / PTX / stub).
#[no_mangle]
pub extern "C" fn cuModuleLoadDataEx(
    module: *mut u64,
    image: *const u8,
    _num_options: u32,
    _options: *mut u32,
    _option_values: *mut *mut u8,
) -> CudaResult {
    // Size-unknown classic API; use 256-byte peek like LoadData.
    cuModuleLoadData(module, image)
}

/// Load module image with known length (Hermes extension path for real fatbins).
pub fn hermes_cuda_module_load_sized(image: &[u8]) -> Result<u64, CudaResult> {
    if image.is_empty() {
        return Err(CUDA_ERROR_INVALID_VALUE);
    }
    let kind = classify_module_image(image);
    if matches!(kind, ModuleImageKind::Unknown) {
        return Err(CUDA_ERROR_NOT_SUPPORTED);
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return Err(g);
        }
        let ctx = live_ctx(s)?;
        let id = next_id(s);
        let name = match kind {
            ModuleImageKind::Fatbin => "fatbin",
            ModuleImageKind::CubinElf => "cubin",
            ModuleImageKind::PtxText => "ptx",
            ModuleImageKind::HermesStub => "stub",
            ModuleImageKind::Unknown => "unknown",
        };
        s.modules.push(Module {
            id,
            ctx,
            name: name.into(),
            functions: vec!["hermes_kernel".into(), "main".into()],
        });
        Ok(id)
    })
}

/// Classify a module image without executing it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleImageKind {
    /// Hermes test image (empty or non-magic) — allowed for software launch shell.
    HermesStub,
    /// CUDA fatbinary magic `0xBA55ED50` (little-endian first u32).
    Fatbin,
    /// ELF cubin (`\x7fELF`).
    CubinElf,
    /// ASCII PTX starting with `.version` or `//`.
    PtxText,
    Unknown,
}

pub fn classify_module_image(image: &[u8]) -> ModuleImageKind {
    if image.is_empty() {
        return ModuleImageKind::HermesStub;
    }
    if image.len() >= 4 {
        let mag = u32::from_le_bytes([image[0], image[1], image[2], image[3]]);
        if mag == 0xBA55_ED50 {
            return ModuleImageKind::Fatbin;
        }
        if image.starts_with(&[0x7f, b'E', b'L', b'F']) {
            return ModuleImageKind::CubinElf;
        }
    }
    let head = core::str::from_utf8(&image[..image.len().min(64)]).unwrap_or("");
    if head.contains(".version") || head.trim_start().starts_with("//") {
        return ModuleImageKind::PtxText;
    }
    // Single NUL used by unit tests as stub.
    if image == [0] {
        return ModuleImageKind::HermesStub;
    }
    ModuleImageKind::Unknown
}

#[no_mangle]
pub extern "C" fn cuModuleLoadData(module: *mut u64, image: *const u8) -> CudaResult {
    if module.is_null() || image.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    // Peek up to 256 bytes for magic (host pointer lifetime is caller's).
    let peek = unsafe { core::slice::from_raw_parts(image, 256) };
    // Find actual length only for NUL-terminated stubs; fatbin needs real size API later.
    let kind = if peek[0] == 0 {
        ModuleImageKind::HermesStub
    } else {
        classify_module_image(peek)
    };
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ctx = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        // Unknown blobs rejected — no silent accept of garbage as "loaded".
        if matches!(kind, ModuleImageKind::Unknown) {
            return CUDA_ERROR_NOT_SUPPORTED;
        }
        let id = next_id(s);
        let name = match kind {
            ModuleImageKind::Fatbin => "fatbin",
            ModuleImageKind::CubinElf => "cubin",
            ModuleImageKind::PtxText => "ptx",
            ModuleImageKind::HermesStub => "stub",
            ModuleImageKind::Unknown => "unknown",
        };
        s.modules.push(Module {
            id,
            ctx,
            name: name.into(),
            // Default export so GetFunction can resolve a kernel without real SM.
            functions: vec!["hermes_kernel".into(), "main".into()],
        });
        unsafe {
            *module = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuModuleUnload(module: u64) -> CudaResult {
    with_state(|s| {
        s.functions.retain(|f| f.module != module);
        let before = s.modules.len();
        s.modules.retain(|m| m.id != module);
        if s.modules.len() == before {
            CUDA_ERROR_INVALID_VALUE
        } else {
            CUDA_SUCCESS
        }
    })
}

#[no_mangle]
pub extern "C" fn cuModuleGetFunction(
    hfunc: *mut u64,
    module: u64,
    name: *const u8,
) -> CudaResult {
    if hfunc.is_null() || name.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let m = match s.modules.iter().find(|m| m.id == module) {
            Some(m) => m,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        let cstr = unsafe {
            let mut len = 0usize;
            while *name.add(len) != 0 && len < 256 {
                len += 1;
            }
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(name, len))
        };
        if !m.functions.iter().any(|f| f == cstr) {
            return CUDA_ERROR_NOT_SUPPORTED;
        }
        let id = next_id(s);
        s.functions.push(Function {
            id,
            module,
            name: cstr.into(),
        });
        unsafe {
            *hfunc = id;
        }
        CUDA_SUCCESS
    })
}

/// Software "launch" — validates grid/block and records success (no real SM).
#[no_mangle]
pub extern "C" fn cuLaunchKernel(
    f: u64,
    grid_x: u32,
    grid_y: u32,
    grid_z: u32,
    block_x: u32,
    block_y: u32,
    block_z: u32,
    _shared_mem: u32,
    hstream: u64,
    _kernel_params: *mut *mut u8,
    _extra: *mut *mut u8,
) -> CudaResult {
    if grid_x == 0 || block_x == 0 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    if block_x as u64 * block_y as u64 * block_z as u64 > 1024 {
        return CUDA_ERROR_INVALID_VALUE;
    }
    let _ = (grid_y, grid_z);
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if !s.functions.iter().any(|fn_| fn_.id == f) {
            return CUDA_ERROR_INVALID_VALUE;
        }
        if hstream != 0 && !s.streams.iter().any(|st| st.id == hstream && st.live) {
            return CUDA_ERROR_INVALID_VALUE;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuStreamCreate(phstream: *mut u64, _flags: u32) -> CudaResult {
    if phstream.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ctx = match live_ctx(s) {
            Ok(c) => c,
            Err(e) => return e,
        };
        let id = next_id(s);
        s.streams.push(Stream {
            id,
            ctx,
            live: true,
        });
        unsafe {
            *phstream = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuStreamDestroy_v2(hstream: u64) -> CudaResult {
    with_state(|s| {
        let before = s.streams.len();
        s.streams.retain(|st| st.id != hstream);
        if s.streams.len() == before {
            CUDA_ERROR_INVALID_VALUE
        } else {
            CUDA_SUCCESS
        }
    })
}

#[no_mangle]
pub extern "C" fn cuStreamSynchronize(hstream: u64) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if hstream != 0 && !s.streams.iter().any(|st| st.id == hstream && st.live) {
            return CUDA_ERROR_INVALID_VALUE;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuEventCreate(phevent: *mut u64, _flags: u32) -> CudaResult {
    if phevent.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let id = next_id(s);
        s.events.push(Event {
            id,
            recorded_on: None,
            completed: false,
        });
        unsafe {
            *phevent = id;
        }
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuEventDestroy_v2(hevent: u64) -> CudaResult {
    with_state(|s| {
        let before = s.events.len();
        s.events.retain(|e| e.id != hevent);
        if s.events.len() == before {
            CUDA_ERROR_INVALID_VALUE
        } else {
            CUDA_SUCCESS
        }
    })
}

#[no_mangle]
pub extern "C" fn cuEventRecord(hevent: u64, hstream: u64) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if hstream != 0 && !s.streams.iter().any(|st| st.id == hstream && st.live) {
            return CUDA_ERROR_INVALID_VALUE;
        }
        let ev = match s.events.iter_mut().find(|e| e.id == hevent) {
            Some(e) => e,
            None => return CUDA_ERROR_INVALID_VALUE,
        };
        ev.recorded_on = Some(hstream);
        ev.completed = true; // host sim completes immediately
        CUDA_SUCCESS
    })
}

#[no_mangle]
pub extern "C" fn cuEventSynchronize(hevent: u64) -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        match s.events.iter().find(|e| e.id == hevent) {
            Some(e) if e.completed => CUDA_SUCCESS,
            Some(_) => CUDA_ERROR_NOT_READY,
            None => CUDA_ERROR_INVALID_VALUE,
        }
    })
}

#[no_mangle]
pub extern "C" fn cuEventElapsedTime(ms: *mut f32, start: u64, end: u64) -> CudaResult {
    if ms.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        let ok = s.events.iter().any(|e| e.id == start && e.completed)
            && s.events.iter().any(|e| e.id == end && e.completed);
        if !ok {
            return CUDA_ERROR_NOT_READY;
        }
        unsafe {
            *ms = 0.001; // synthetic host elapsed
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

#[no_mangle]
pub extern "C" fn cudaStreamCreate(pstream: *mut u64) -> CudaResult {
    cuStreamCreate(pstream, 0)
}

#[no_mangle]
pub extern "C" fn cudaStreamDestroy(stream: u64) -> CudaResult {
    cuStreamDestroy_v2(stream)
}

#[no_mangle]
pub extern "C" fn cudaStreamSynchronize(stream: u64) -> CudaResult {
    cuStreamSynchronize(stream)
}

#[no_mangle]
pub extern "C" fn cudaMemset(dev_ptr: u64, value: i32, count: usize) -> CudaResult {
    cuMemsetD8_v2(dev_ptr, value as u8, count)
}

#[no_mangle]
pub extern "C" fn cudaMemGetInfo(free: *mut usize, total: *mut usize) -> CudaResult {
    if free.is_null() || total.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    let mut f = 0u64;
    let mut t = 0u64;
    let r = cuMemGetInfo_v2(&mut f, &mut t);
    if r == CUDA_SUCCESS {
        unsafe {
            *free = f as usize;
            *total = t as usize;
        }
    }
    r
}

#[no_mangle]
pub extern "C" fn cudaGetDevice(device: *mut i32) -> CudaResult {
    cuCtxGetDevice(device)
}

#[no_mangle]
pub extern "C" fn cudaSetDevice(device: i32) -> CudaResult {
    let mut ctx = 0u64;
    let r = cuDevicePrimaryCtxRetain(&mut ctx, device);
    if r != CUDA_SUCCESS {
        return r;
    }
    // Primary retain already pushed current; release extra ref if set device only
    // reuses retain semantics — keep one ref for the primary as "set".
    CUDA_SUCCESS
}

#[no_mangle]
pub extern "C" fn cudaDeviceSynchronize() -> CudaResult {
    with_state(|s| {
        let g = require_gsp(s);
        if g != CUDA_SUCCESS {
            return g;
        }
        if live_ctx(s).is_err() {
            return CUDA_ERROR_INVALID_CONTEXT;
        }
        CUDA_SUCCESS
    })
}

/// Runtime memcpy kinds (cudaMemcpyKind subset).
pub const CUDA_MEMCPY_HOST_TO_HOST: i32 = 0;
pub const CUDA_MEMCPY_HOST_TO_DEVICE: i32 = 1;
pub const CUDA_MEMCPY_DEVICE_TO_HOST: i32 = 2;
pub const CUDA_MEMCPY_DEVICE_TO_DEVICE: i32 = 3;

#[no_mangle]
pub extern "C" fn cudaMemcpy(
    dst: u64,
    src: u64,
    count: usize,
    kind: i32,
) -> CudaResult {
    match kind {
        CUDA_MEMCPY_HOST_TO_DEVICE => cuMemcpyHtoD_v2(dst, src as *const u8, count),
        CUDA_MEMCPY_DEVICE_TO_HOST => cuMemcpyDtoH_v2(dst as *mut u8, src, count),
        CUDA_MEMCPY_DEVICE_TO_DEVICE => cuMemcpyDtoD_v2(dst, src, count),
        CUDA_MEMCPY_HOST_TO_HOST => {
            if count == 0 || src == 0 || dst == 0 {
                return CUDA_ERROR_INVALID_VALUE;
            }
            unsafe {
                core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, count);
            }
            CUDA_SUCCESS
        }
        _ => CUDA_ERROR_INVALID_VALUE,
    }
}

#[no_mangle]
pub extern "C" fn cudaGetLastError() -> CudaResult {
    CUDA_SUCCESS
}

#[no_mangle]
pub extern "C" fn cudaPeekAtLastError() -> CudaResult {
    CUDA_SUCCESS
}

#[no_mangle]
pub extern "C" fn cudaDriverGetVersion(driver_version: *mut i32) -> CudaResult {
    cuDriverGetVersion(driver_version)
}

#[no_mangle]
pub extern "C" fn cudaRuntimeGetVersion(runtime_version: *mut i32) -> CudaResult {
    if runtime_version.is_null() {
        return CUDA_ERROR_INVALID_VALUE;
    }
    unsafe {
        *runtime_version = 12_000;
    }
    CUDA_SUCCESS
}

/// Count of driver entry points Hermes currently exports (for drop-in dashboards).
pub fn hermes_cuda_driver_entry_count() -> usize {
    // + host alloc, peer memcpy, runtime peer
    62
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

    #[test]
    fn stream_event_module_launch() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut ctx = 0u64;
        assert_eq!(cuCtxCreate_v2(&mut ctx, 0, 0), CUDA_SUCCESS);

        let mut attr = 0i32;
        assert_eq!(
            cuDeviceGetAttribute(&mut attr, CU_DEVICE_ATTRIBUTE_WARP_SIZE, 0),
            CUDA_SUCCESS
        );
        assert_eq!(attr, 32);

        let mut name = [0u8; 64];
        assert_eq!(cuDeviceGetName(name.as_mut_ptr(), 64, 0), CUDA_SUCCESS);

        let mut stream = 0u64;
        assert_eq!(cuStreamCreate(&mut stream, 0), CUDA_SUCCESS);
        let mut ev_s = 0u64;
        let mut ev_e = 0u64;
        assert_eq!(cuEventCreate(&mut ev_s, 0), CUDA_SUCCESS);
        assert_eq!(cuEventCreate(&mut ev_e, 0), CUDA_SUCCESS);
        assert_eq!(cuEventRecord(ev_s, stream), CUDA_SUCCESS);

        let image = b"\0";
        let mut module = 0u64;
        assert_eq!(cuModuleLoadData(&mut module, image.as_ptr()), CUDA_SUCCESS);
        let mut func = 0u64;
        let kname = b"hermes_kernel\0";
        assert_eq!(
            cuModuleGetFunction(&mut func, module, kname.as_ptr()),
            CUDA_SUCCESS
        );
        assert_eq!(
            cuLaunchKernel(func, 1, 1, 1, 32, 1, 1, 0, stream, core::ptr::null_mut(), core::ptr::null_mut()),
            CUDA_SUCCESS
        );
        assert_eq!(cuEventRecord(ev_e, stream), CUDA_SUCCESS);
        assert_eq!(cuEventSynchronize(ev_e), CUDA_SUCCESS);
        let mut ms = 0f32;
        assert_eq!(cuEventElapsedTime(&mut ms, ev_s, ev_e), CUDA_SUCCESS);
        assert!(ms > 0.0);

        let mut a = 0u64;
        let mut b = 0u64;
        assert_eq!(cuMemAlloc_v2(&mut a, 8), CUDA_SUCCESS);
        assert_eq!(cuMemAlloc_v2(&mut b, 8), CUDA_SUCCESS);
        assert_eq!(cuMemsetD8_v2(a, 0xab, 8), CUDA_SUCCESS);
        assert_eq!(cuMemcpyDtoD_v2(b, a, 8), CUDA_SUCCESS);
        let mut host = [0u8; 8];
        assert_eq!(cuMemcpyDtoH_v2(host.as_mut_ptr(), b, 8), CUDA_SUCCESS);
        assert_eq!(host, [0xab; 8]);

        assert_eq!(cuStreamSynchronize(stream), CUDA_SUCCESS);
        assert_eq!(cuModuleUnload(module), CUDA_SUCCESS);
        assert_eq!(cuEventDestroy_v2(ev_s), CUDA_SUCCESS);
        assert_eq!(cuEventDestroy_v2(ev_e), CUDA_SUCCESS);
        assert_eq!(cuStreamDestroy_v2(stream), CUDA_SUCCESS);
        assert_eq!(cuCtxDestroy_v2(ctx), CUDA_SUCCESS);
        assert!(hermes_cuda_driver_entry_count() >= 20);
        hermes_cuda_reset();
    }

    #[test]
    fn offline_rejects_stream_create() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        let mut s = 0u64;
        assert_eq!(cuStreamCreate(&mut s, 0), CUDA_ERROR_HERMES_GSP_OFFLINE);
    }

    #[test]
    fn session_bind_device_name_visible_after_init() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_bind_session_device("NVIDIA Turing [1fb9]", 8 << 30, 7, 5);
        assert!(hermes_cuda_gsp_online());
        assert_eq!(hermes_cuda_device_count(), 1);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut name = [0u8; 64];
        assert_eq!(cuDeviceGetName(name.as_mut_ptr(), 64, 0), CUDA_SUCCESS);
        let s = core::str::from_utf8(&name[..name.iter().position(|&b| b == 0).unwrap_or(0)])
            .unwrap_or("");
        assert!(s.contains("1fb9") || s.contains("Turing"));
        hermes_cuda_reset();
    }

    #[test]
    fn classifies_fatbin_and_ptx() {
        let fat = 0xBA55_ED50u32.to_le_bytes();
        assert_eq!(classify_module_image(&fat), ModuleImageKind::Fatbin);
        let ptx = b".version 7.0\n.target sm_75\n";
        assert_eq!(classify_module_image(ptx), ModuleImageKind::PtxText);
        assert_eq!(
            classify_module_image(b"\x7fELF...."),
            ModuleImageKind::CubinElf
        );
        assert_eq!(classify_module_image(b"garbage!!"), ModuleImageKind::Unknown);
    }

    #[test]
    fn primary_context_retain_release_and_memgetinfo() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        assert_eq!(cuInit(0), CUDA_SUCCESS);

        let mut flags = 1u32;
        let mut active = 1i32;
        assert_eq!(
            cuDevicePrimaryCtxGetState(0, &mut flags, &mut active),
            CUDA_SUCCESS
        );
        assert_eq!(active, 0);

        let mut ctx = 0u64;
        assert_eq!(cuDevicePrimaryCtxRetain(&mut ctx, 0), CUDA_SUCCESS);
        assert_ne!(ctx, 0);
        assert_eq!(
            cuDevicePrimaryCtxGetState(0, &mut flags, &mut active),
            CUDA_SUCCESS
        );
        assert_eq!(active, 1);

        let mut cur = 0u64;
        assert_eq!(cuCtxGetCurrent(&mut cur), CUDA_SUCCESS);
        assert_eq!(cur, ctx);

        let mut free = 0u64;
        let mut total = 0u64;
        assert_eq!(cuMemGetInfo_v2(&mut free, &mut total), CUDA_SUCCESS);
        assert_eq!(total, 8 * 1024 * 1024 * 1024);
        assert_eq!(free, total);

        let mut dptr = 0u64;
        assert_eq!(cuMemAlloc_v2(&mut dptr, 1024), CUDA_SUCCESS);
        assert_eq!(cuMemGetInfo_v2(&mut free, &mut total), CUDA_SUCCESS);
        assert_eq!(free, total - 1024);

        let mut dev = -1i32;
        assert_eq!(cuCtxGetDevice(&mut dev), CUDA_SUCCESS);
        assert_eq!(dev, 0);

        let mut attr = 0i32;
        assert_eq!(
            cuDeviceGetAttribute(&mut attr, CU_DEVICE_ATTRIBUTE_MAX_GRID_DIM_X, 0),
            CUDA_SUCCESS
        );
        assert!(attr > 0);

        assert_eq!(cuMemFree_v2(dptr), CUDA_SUCCESS);
        assert_eq!(cuDevicePrimaryCtxRelease(0), CUDA_SUCCESS);
        assert_eq!(
            cuDevicePrimaryCtxGetState(0, &mut flags, &mut active),
            CUDA_SUCCESS
        );
        assert_eq!(active, 0);
        assert!(hermes_cuda_driver_entry_count() >= 40);
        hermes_cuda_reset();
    }

    #[test]
    fn ctx_push_pop_and_set_current() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut a = 0u64;
        let mut b = 0u64;
        assert_eq!(cuCtxCreate_v2(&mut a, 0, 0), CUDA_SUCCESS);
        assert_eq!(cuCtxCreate_v2(&mut b, 0, 0), CUDA_SUCCESS);
        assert_eq!(cuCtxSetCurrent(a), CUDA_SUCCESS);
        let mut cur = 0u64;
        assert_eq!(cuCtxGetCurrent(&mut cur), CUDA_SUCCESS);
        assert_eq!(cur, a);
        assert_eq!(cuCtxPushCurrent_v2(b), CUDA_SUCCESS);
        assert_eq!(cuCtxGetCurrent(&mut cur), CUDA_SUCCESS);
        assert_eq!(cur, b);
        let mut popped = 0u64;
        assert_eq!(cuCtxPopCurrent_v2(&mut popped), CUDA_SUCCESS);
        assert_eq!(popped, b);
        assert_eq!(cuCtxGetCurrent(&mut cur), CUDA_SUCCESS);
        assert_eq!(cur, a);
        assert_eq!(cuCtxDestroy_v2(b), CUDA_SUCCESS);
        assert_eq!(cuCtxDestroy_v2(a), CUDA_SUCCESS);
        hermes_cuda_reset();
    }

    #[test]
    fn offline_primary_retain_fails() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        let mut ctx = 0u64;
        assert_eq!(
            cuDevicePrimaryCtxRetain(&mut ctx, 0),
            CUDA_ERROR_HERMES_GSP_OFFLINE
        );
    }

    #[test]
    fn driver_version_and_error_strings() {
        let mut ver = 0i32;
        assert_eq!(cuDriverGetVersion(&mut ver), CUDA_SUCCESS);
        assert_eq!(ver, 12_000);
        let mut p: *const i8 = core::ptr::null();
        assert_eq!(
            cuGetErrorName(CUDA_ERROR_HERMES_GSP_OFFLINE, &mut p),
            CUDA_SUCCESS
        );
        assert!(!p.is_null());
        let s = unsafe { std::ffi::CStr::from_ptr(p) };
        assert!(s.to_bytes().starts_with(b"CUDA_ERROR_HERMES"));
        assert_eq!(cuGetErrorString(CUDA_SUCCESS, &mut p), CUDA_SUCCESS);
    }

    #[test]
    fn async_memcpy_and_sized_module() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut ctx = 0u64;
        assert_eq!(cuCtxCreate_v2(&mut ctx, 0, 0), CUDA_SUCCESS);
        let mut stream = 0u64;
        assert_eq!(cuStreamCreate(&mut stream, 0), CUDA_SUCCESS);
        let mut d = 0u64;
        assert_eq!(cuMemAlloc_v2(&mut d, 8), CUDA_SUCCESS);
        let src = [9u8; 8];
        assert_eq!(
            cuMemcpyHtoDAsync_v2(d, src.as_ptr(), 8, stream),
            CUDA_SUCCESS
        );
        let mut dst = [0u8; 8];
        assert_eq!(
            cuMemcpyDtoHAsync_v2(dst.as_mut_ptr(), d, 8, stream),
            CUDA_SUCCESS
        );
        assert_eq!(src, dst);
        let ptx = b".version 7.0\n.target sm_75\n";
        let mid = hermes_cuda_module_load_sized(ptx).unwrap();
        assert_ne!(mid, 0);
        assert_eq!(cuModuleUnload(mid), CUDA_SUCCESS);
        hermes_cuda_reset();
    }

    #[test]
    fn multi_device_peer_access() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        hermes_cuda_register_device("GPU0", 8 << 30, 7, 5);
        hermes_cuda_register_peer_device("GPU1", 8 << 30, 7, 5);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut n = 0i32;
        assert_eq!(cuDeviceGetCount(&mut n), CUDA_SUCCESS);
        assert_eq!(n, 2);
        let mut can = 0i32;
        assert_eq!(cuDeviceCanAccessPeer(&mut can, 0, 1), CUDA_SUCCESS);
        assert_eq!(can, 1);
        assert_eq!(cuDeviceCanAccessPeer(&mut can, 0, 0), CUDA_SUCCESS);
        assert_eq!(can, 0);
        let mut ctx = 0u64;
        assert_eq!(cuCtxCreate_v2(&mut ctx, 0, 0), CUDA_SUCCESS);
        assert_eq!(cuCtxEnablePeerAccess(1, 0), CUDA_SUCCESS);
        assert!(hermes_cuda_peer_enabled(0, 1));
        assert_eq!(cuCtxDisablePeerAccess(1), CUDA_SUCCESS);
        assert!(!hermes_cuda_peer_enabled(0, 1));
        hermes_cuda_reset();
        assert_eq!(
            cuDeviceCanAccessPeer(&mut can, 0, 1),
            CUDA_ERROR_HERMES_GSP_OFFLINE
        );
    }

    #[test]
    fn host_alloc_and_same_device_peer_copy() {
        let _g = TEST_LOCK.lock().unwrap();
        hermes_cuda_reset();
        hermes_cuda_set_gsp_online(true);
        assert_eq!(cuInit(0), CUDA_SUCCESS);
        let mut ctx = 0u64;
        assert_eq!(cuCtxCreate_v2(&mut ctx, 0, 0), CUDA_SUCCESS);
        let mut hp: *mut u8 = core::ptr::null_mut();
        assert_eq!(cuMemHostAlloc(&mut hp, 16, 0), CUDA_SUCCESS);
        assert!(!hp.is_null());
        unsafe {
            for i in 0..16 {
                *hp.add(i) = i as u8;
            }
        }
        let mut a = 0u64;
        let mut b = 0u64;
        assert_eq!(cuMemAlloc_v2(&mut a, 16), CUDA_SUCCESS);
        assert_eq!(cuMemAlloc_v2(&mut b, 16), CUDA_SUCCESS);
        assert_eq!(cuMemcpyHtoD_v2(a, hp, 16), CUDA_SUCCESS);
        assert_eq!(cuMemcpyPeer(b, 0, a, 0, 16), CUDA_SUCCESS);
        let mut out = [0u8; 16];
        assert_eq!(cuMemcpyDtoH_v2(out.as_mut_ptr(), b, 16), CUDA_SUCCESS);
        assert_eq!(out[15], 15);
        assert_eq!(cuMemFreeHost(hp), CUDA_SUCCESS);
        hermes_cuda_reset();
    }
}
