//! Mesa / NVK-facing userspace surface for Hermes.
//!
//! Provides a Vulkan ICD-shaped loader entry and a minimal GL dispatch table.
//! Real shader compilers and full NVK are not claimed — this is the Hermes
//! attach point that only advertises a GPU when DRM + GSP Online agree.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use hermes_drm::{
    AtomicCommit, AtomicRequest, DisplayMode, DrmDevice, Framebuffer, PixelFormat,
};

static GSP_ONLINE: AtomicBool = AtomicBool::new(false);
static INSTANCE_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
struct MesaState {
    drm: Option<DrmDevice>,
    vk_instances: u32,
    gl_contexts: u32,
}

static STATE: Mutex<MesaState> = Mutex::new(MesaState {
    drm: None,
    vk_instances: 0,
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
        s.gl_contexts = 0;
    });
    INSTANCE_COUNT.store(0, Ordering::SeqCst);
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

pub mod present {
    use super::*;
    use hermes_drm::CommitError;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum PresentError {
        Offline,
        Modeset(CommitError),
        NoDrm,
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
                fb_id: 10,
                mode: DisplayMode::fhd_60(),
                active: true,
            };
            let r = atom
                .commit(drm, &req)
                .map_err(PresentError::Modeset)?;
            Ok(r.sequence)
        })
    }
}

/// ICD discovery info (for future `nvidia_icd.json` / Mesa loader).
pub fn hermes_vulkan_icd_library_path() -> &'static str {
    "libhermes_mesa.so"
}

pub fn hermes_vulkan_api_version() -> u32 {
    // Vulkan 1.3 encoded
    (1 << 22) | (3 << 12)
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
}
