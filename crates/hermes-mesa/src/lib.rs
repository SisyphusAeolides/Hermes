//! Mesa / NVK-facing userspace surface for Hermes.
//!
//! Provides a Vulkan ICD-shaped loader entry and a minimal GL dispatch table.
//! Real shader compilers and full NVK are not claimed — this is the Hermes
//! attach point that only advertises a GPU when DRM + GSP Online agree.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use hermes_drm::{
    page_flip, AtomicCommit, AtomicRequest, DisplayMode, DrmDevice, Framebuffer,
    PageFlipRequest, PixelFormat,
};

pub mod icd;

pub use icd::{
    default_icd_json, vulkan_icd_json, HERMES_ICD_JSON_NAME, ICD_LIBRARY_BASENAME,
    ICD_SEARCH_PATHS, NVIDIA_ICD_JSON_NAME, NVIDIA_VULKAN_SONAME,
};

static GSP_ONLINE: AtomicBool = AtomicBool::new(false);
static INSTANCE_COUNT: AtomicU32 = AtomicU32::new(0);
static DEVICE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
struct MesaState {
    drm: Option<DrmDevice>,
    vk_instances: u32,
    vk_devices: u32,
    gl_contexts: u32,
}

static STATE: Mutex<MesaState> = Mutex::new(MesaState {
    drm: None,
    vk_instances: 0,
    vk_devices: 0,
    gl_contexts: 0,
});

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut MesaState) -> R,
{
    let mut g = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

pub fn hermes_mesa_set_gsp_online(online: bool) {
    GSP_ONLINE.store(online, Ordering::SeqCst);
    with_state(|s| {
        if let Some(d) = s.drm.as_mut() {
            d.set_gsp_online(online);
        } else if online {
            s.drm = Some(DrmDevice::virtual_desktop(true));
        }
        if !online {
            s.vk_instances = 0;
            s.gl_contexts = 0;
        }
    });
}

pub fn hermes_mesa_gsp_online() -> bool {
    GSP_ONLINE.load(Ordering::SeqCst)
}

pub fn hermes_mesa_reset() {
    GSP_ONLINE.store(false, Ordering::SeqCst);
    with_state(|s| {
        s.drm = None;
        s.vk_instances = 0;
        s.vk_devices = 0;
        s.gl_contexts = 0;
    });
    INSTANCE_COUNT.store(0, Ordering::SeqCst);
    DEVICE_COUNT.store(0, Ordering::SeqCst);
}

// ─── Vulkan ICD-shaped API (subset) ───────────────────────────────────────

pub type VkResult = i32;
pub const VK_SUCCESS: VkResult = 0;
pub const VK_ERROR_INITIALIZATION_FAILED: VkResult = -3;
pub const VK_ERROR_INCOMPATIBLE_DRIVER: VkResult = -9;
pub const VK_ERROR_DEVICE_LOST: VkResult = -4;

/// `vkCreateInstance` (simplified — no pCreateInfo parse yet).
#[no_mangle]
pub extern "C" fn vkCreateInstance(
    _p_create_info: *const u8,
    _p_allocator: *const u8,
    p_instance: *mut u64,
) -> VkResult {
    if p_instance.is_null() {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    if !hermes_mesa_gsp_online() {
        return VK_ERROR_INCOMPATIBLE_DRIVER;
    }
    let id = INSTANCE_COUNT.fetch_add(1, Ordering::SeqCst) as u64 + 1;
    with_state(|s| s.vk_instances = s.vk_instances.saturating_add(1));
    unsafe {
        *p_instance = id;
    }
    VK_SUCCESS
}

#[no_mangle]
pub extern "C" fn vkDestroyInstance(instance: u64, _p_allocator: *const u8) {
    if instance == 0 {
        return;
    }
    with_state(|s| {
        s.vk_instances = s.vk_instances.saturating_sub(1);
    });
}

#[no_mangle]
pub extern "C" fn vkEnumeratePhysicalDevices(
    _instance: u64,
    p_count: *mut u32,
    p_devices: *mut u64,
) -> VkResult {
    if p_count.is_null() {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    if !hermes_mesa_gsp_online() {
        unsafe {
            *p_count = 0;
        }
        return VK_SUCCESS;
    }
    unsafe {
        if p_devices.is_null() {
            *p_count = 1;
        } else if *p_count >= 1 {
            *p_devices = 1;
            *p_count = 1;
        } else {
            *p_count = 1;
        }
    }
    VK_SUCCESS
}

/// Physical device properties (compact host view).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HermesVkPhysicalDeviceProperties {
    pub api_version: u32,
    pub driver_version: u32,
    pub vendor_id: u32,
    pub device_id: u32,
    pub device_type: u32, // 2 = DISCRETE_GPU
}

#[no_mangle]
pub extern "C" fn vkGetPhysicalDeviceProperties(
    physical_device: u64,
    props: *mut HermesVkPhysicalDeviceProperties,
) -> VkResult {
    if props.is_null() || physical_device == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    if !hermes_mesa_gsp_online() {
        return VK_ERROR_DEVICE_LOST;
    }
    unsafe {
        *props = HermesVkPhysicalDeviceProperties {
            api_version: hermes_vulkan_api_version(),
            driver_version: 1,
            vendor_id: 0x10de,
            device_id: 0x1fb9, // sample Turing
            device_type: 2,
        };
    }
    VK_SUCCESS
}

#[no_mangle]
pub extern "C" fn vkCreateDevice(
    physical_device: u64,
    _p_create_info: *const u8,
    _p_allocator: *const u8,
    p_device: *mut u64,
) -> VkResult {
    if p_device.is_null() || physical_device == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    if !hermes_mesa_gsp_online() {
        return VK_ERROR_DEVICE_LOST;
    }
    let id = DEVICE_COUNT.fetch_add(1, Ordering::SeqCst) as u64 + 1;
    with_state(|s| s.vk_devices = s.vk_devices.saturating_add(1));
    unsafe {
        *p_device = id;
    }
    VK_SUCCESS
}

#[no_mangle]
pub extern "C" fn vkDestroyDevice(device: u64, _p_allocator: *const u8) {
    if device == 0 {
        return;
    }
    with_state(|s| {
        s.vk_devices = s.vk_devices.saturating_sub(1);
    });
}

#[no_mangle]
pub extern "C" fn vkGetDeviceQueue(
    device: u64,
    _queue_family_index: u32,
    _queue_index: u32,
    p_queue: *mut u64,
) -> VkResult {
    if p_queue.is_null() || device == 0 {
        return VK_ERROR_INITIALIZATION_FAILED;
    }
    if !hermes_mesa_gsp_online() {
        return VK_ERROR_DEVICE_LOST;
    }
    unsafe {
        *p_queue = device; // single software queue token
    }
    VK_SUCCESS
}

// ─── GL dispatch subset ───────────────────────────────────────────────────

pub type GLenum = u32;
pub type GLuint = u32;
pub type GLsizei = i32;

static mut CURRENT_CLEAR: [f32; 4] = [0.0; 4];

/// `glClearColor`
#[no_mangle]
pub unsafe extern "C" fn glClearColor(r: f32, g: f32, b: f32, a: f32) {
    CURRENT_CLEAR = [r, g, b, a];
}

/// `glGetError` — returns 0 (GL_NO_ERROR) when online, else invalid operation.
#[no_mangle]
pub extern "C" fn glGetError() -> GLenum {
    if hermes_mesa_gsp_online() {
        0
    } else {
        0x0502 // GL_INVALID_OPERATION
    }
}

// glGetString name tokens (subset of GL).
pub const GL_VENDOR: GLenum = 0x1F00;
pub const GL_RENDERER: GLenum = 0x1F01;
pub const GL_VERSION: GLenum = 0x1F02;
pub const GL_EXTENSIONS: GLenum = 0x1F03;
pub const GL_SHADING_LANGUAGE_VERSION: GLenum = 0x8B8C;

// Stable C strings for the GL identity surface (valid for process lifetime).
static GL_STR_VENDOR: &[u8] = b"Hermes GSP\0";
static GL_STR_RENDERER: &[u8] = b"Hermes Mesa/NVK attach (software)\0";
static GL_STR_VERSION: &[u8] = b"4.6 Hermes GSP\0";
static GL_STR_EXTENSIONS: &[u8] = b"GL_ARB_vertex_buffer_object GL_ARB_framebuffer_object\0";
static GL_STR_GLSL: &[u8] = b"4.60 Hermes\0";
static GL_STR_EMPTY: &[u8] = b"\0";

/// `glGetString` — GSP Online required; Offline returns null (classic invalid).
#[no_mangle]
pub extern "C" fn glGetString(name: GLenum) -> *const u8 {
    if !hermes_mesa_gsp_online() {
        return core::ptr::null();
    }
    match name {
        GL_VENDOR => GL_STR_VENDOR.as_ptr(),
        GL_RENDERER => GL_STR_RENDERER.as_ptr(),
        GL_VERSION => GL_STR_VERSION.as_ptr(),
        GL_EXTENSIONS => GL_STR_EXTENSIONS.as_ptr(),
        GL_SHADING_LANGUAGE_VERSION => GL_STR_GLSL.as_ptr(),
        _ => GL_STR_EMPTY.as_ptr(),
    }
}

/// Create a software GL context token (not full GLX/EGL).
pub fn hermes_gl_create_context() -> Option<u32> {
    if !hermes_mesa_gsp_online() {
        return None;
    }
    Some(with_state(|s| {
        s.gl_contexts = s.gl_contexts.saturating_add(1);
        s.gl_contexts
    }))
}

/// Present a solid-color frame through the DRM atomic path (software).
pub fn hermes_present_solid_frame() -> Result<u64, present::PresentError> {
    present::present_solid()
}

/// Present via dumb GEM BO + atomic + page-flip (full software display path).
pub fn hermes_present_gem_flip(color: u32) -> Result<u64, present::PresentError> {
    present::present_gem_flip(color)
}

pub mod present {
    use super::*;
    use hermes_drm::{CommitError, FlipError};

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum PresentError {
        Offline,
        Modeset(CommitError),
        Flip(FlipError),
        NoDrm,
        Gem,
    }

    pub fn present_solid() -> Result<u64, PresentError> {
        if !hermes_mesa_gsp_online() {
            return Err(PresentError::Offline);
        }
        with_state(|s| {
            let drm = s.drm.get_or_insert_with(|| DrmDevice::virtual_desktop(true));
            drm.set_gsp_online(true);
            if drm.framebuffers.is_empty() {
                let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 0x2000)
                    .map_err(|_| PresentError::NoDrm)?;
                drm.framebuffers.push(fb);
            }
            let mut atom = AtomicCommit::new();
            let req = AtomicRequest {
                connector_id: 1,
                crtc_id: 1,
                plane_id: 1,
                fb_id: drm.framebuffers[0].id,
                mode: DisplayMode::fhd_60(),
                active: true,
            };
            let r = atom
                .commit(drm, &req)
                .map_err(PresentError::Modeset)?;
            Ok(r.sequence)
        })
    }

    pub fn present_gem_flip(color: u32) -> Result<u64, PresentError> {
        if !hermes_mesa_gsp_online() {
            return Err(PresentError::Offline);
        }
        with_state(|s| {
            let drm = s.drm.get_or_insert_with(|| DrmDevice::virtual_desktop(true));
            drm.set_gsp_online(true);
            let dumb = drm
                .create_dumb(1920, 1080, 32)
                .map_err(|_| PresentError::Gem)?;
            drm.gems
                .get_mut(dumb.handle)
                .ok_or(PresentError::Gem)?
                .fill_solid_xrgb8888(color)
                .map_err(|_| PresentError::Gem)?;
            let fb_id = drm
                .add_fb_from_gem(dumb.handle, PixelFormat::Xrgb8888)
                .map_err(|_| PresentError::Gem)?;
            let mut atom = AtomicCommit::new();
            if drm.active_crtc_count() == 0 {
                atom.commit(
                    drm,
                    &AtomicRequest {
                        connector_id: 1,
                        crtc_id: 1,
                        plane_id: 1,
                        fb_id,
                        mode: DisplayMode::fhd_60(),
                        active: true,
                    },
                )
                .map_err(PresentError::Modeset)?;
            }
            let r = page_flip(
                &mut atom,
                drm,
                &PageFlipRequest {
                    crtc_id: 1,
                    fb_id,
                    flags: PageFlipRequest::FLAG_EVENT,
                },
            )
            .map_err(PresentError::Flip)?;
            Ok(r.sequence)
        })
    }
}

/// ICD discovery info (for `nvidia_icd.json` / Mesa loader).
pub fn hermes_vulkan_icd_library_path() -> &'static str {
    ICD_LIBRARY_BASENAME
}

pub fn hermes_vulkan_api_version() -> u32 {
    // Vulkan 1.3 encoded
    (1 << 22) | (3 << 12)
}

pub fn hermes_vulkan_icd_json() -> String {
    default_icd_json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static T: Mutex<()> = Mutex::new(());

    #[test]
    fn vulkan_instance_requires_gsp() {
        let _g = T.lock().unwrap();
        hermes_mesa_reset();
        let mut inst = 0u64;
        assert_eq!(
            vkCreateInstance(core::ptr::null(), core::ptr::null(), &mut inst),
            VK_ERROR_INCOMPATIBLE_DRIVER
        );
        hermes_mesa_set_gsp_online(true);
        assert_eq!(
            vkCreateInstance(core::ptr::null(), core::ptr::null(), &mut inst),
            VK_SUCCESS
        );
        assert_ne!(inst, 0);
        let mut count = 0u32;
        assert_eq!(
            vkEnumeratePhysicalDevices(inst, &mut count, core::ptr::null_mut()),
            VK_SUCCESS
        );
        assert_eq!(count, 1);
        vkDestroyInstance(inst, core::ptr::null());
        hermes_mesa_reset();
    }

    #[test]
    fn present_through_drm_atomic() {
        let _g = T.lock().unwrap();
        hermes_mesa_reset();
        assert!(hermes_present_solid_frame().is_err());
        hermes_mesa_set_gsp_online(true);
        let seq = hermes_present_solid_frame().unwrap();
        assert_eq!(seq, 1);
        hermes_mesa_reset();
    }

    #[test]
    fn gl_error_reflects_online() {
        let _g = T.lock().unwrap();
        hermes_mesa_reset();
        assert_ne!(glGetError(), 0);
        hermes_mesa_set_gsp_online(true);
        assert_eq!(glGetError(), 0);
        assert!(hermes_gl_create_context().is_some());
        hermes_mesa_reset();
    }

    #[test]
    fn gl_get_string_requires_online() {
        let _g = T.lock().unwrap();
        hermes_mesa_reset();
        assert!(glGetString(GL_VENDOR).is_null());
        hermes_mesa_set_gsp_online(true);
        let p = glGetString(GL_VENDOR);
        assert!(!p.is_null());
        let s = unsafe { std::ffi::CStr::from_ptr(p as *const i8) };
        assert!(s.to_bytes().starts_with(b"Hermes"));
        let r = glGetString(GL_RENDERER);
        assert!(!r.is_null());
        hermes_mesa_reset();
    }

    #[test]
    fn device_and_queue_require_gsp() {
        let _g = T.lock().unwrap();
        hermes_mesa_reset();
        let mut dev = 0u64;
        assert_eq!(
            vkCreateDevice(1, core::ptr::null(), core::ptr::null(), &mut dev),
            VK_ERROR_DEVICE_LOST
        );
        hermes_mesa_set_gsp_online(true);
        assert_eq!(
            vkCreateDevice(1, core::ptr::null(), core::ptr::null(), &mut dev),
            VK_SUCCESS
        );
        let mut q = 0u64;
        assert_eq!(vkGetDeviceQueue(dev, 0, 0, &mut q), VK_SUCCESS);
        assert_ne!(q, 0);
        let mut props = HermesVkPhysicalDeviceProperties {
            api_version: 0,
            driver_version: 0,
            vendor_id: 0,
            device_id: 0,
            device_type: 0,
        };
        assert_eq!(vkGetPhysicalDeviceProperties(1, &mut props), VK_SUCCESS);
        assert_eq!(props.vendor_id, 0x10de);
        vkDestroyDevice(dev, core::ptr::null());
        hermes_mesa_reset();
    }

    #[test]
    fn gem_flip_present() {
        let _g = T.lock().unwrap();
        hermes_mesa_reset();
        hermes_mesa_set_gsp_online(true);
        let seq = hermes_present_gem_flip(0x0000_00ff).unwrap();
        assert!(seq >= 1);
        assert!(hermes_vulkan_icd_json().contains("libhermes_mesa"));
        hermes_mesa_reset();
    }
}
