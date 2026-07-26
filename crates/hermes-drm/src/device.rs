//! DRM device aggregate — connectors, CRTCs, planes, framebuffers, GEM.

use alloc::vec::Vec;

use crate::connector::Connector;
use crate::crtc::Crtc;
use crate::framebuffer::{Framebuffer, FramebufferError, PixelFormat};
use crate::gem::{DumbCreateRequest, DumbCreateResult, GemError, GemManager};
use crate::pageflip::VblankState;
use crate::plane::Plane;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrmError {
    GspOffline,
    NotFound,
    Busy,
    Gem(GemError),
    Framebuffer(FramebufferError),
}

#[derive(Clone, Debug)]
pub struct DrmDevice {
    pub gsp_online: bool,
    pub connectors: Vec<Connector>,
    pub crtcs: Vec<Crtc>,
    pub planes: Vec<Plane>,
    pub framebuffers: Vec<Framebuffer>,
    pub gems: GemManager,
    pub vblank: VblankState,
    next_fb_id: u32,
}

impl DrmDevice {
    /// Minimal virtual desktop topology (1 CRTC, 1 primary plane, 1 connector).
    pub fn virtual_desktop(gsp_online: bool) -> Self {
        Self {
            gsp_online,
            connectors: alloc::vec![Connector::virtual_fhd(1)],
            crtcs: alloc::vec![Crtc::new(1, 1)],
            planes: alloc::vec![Plane::primary(1, 0b1)],
            framebuffers: Vec::new(),
            gems: GemManager::new(),
            vblank: VblankState::new(),
            next_fb_id: 1,
        }
    }

    /// Dual-head virtual topology (2 CRTCs / planes / connectors).
    pub fn virtual_dual_head(gsp_online: bool) -> Self {
        Self {
            gsp_online,
            connectors: alloc::vec![
                Connector::virtual_fhd(1),
                Connector::virtual_fhd(2),
            ],
            crtcs: alloc::vec![Crtc::new(1, 1), Crtc::new(2, 2)],
            planes: alloc::vec![
                Plane::primary(1, 0b01),
                Plane::primary(2, 0b10),
            ],
            framebuffers: Vec::new(),
            gems: GemManager::new(),
            vblank: VblankState::new(),
            next_fb_id: 1,
        }
    }

    pub fn set_gsp_online(&mut self, online: bool) {
        self.gsp_online = online;
        if !online {
            for c in &mut self.crtcs {
                c.active = false;
            }
            for p in &mut self.planes {
                p.crtc_id = None;
                p.fb_id = None;
            }
            for c in &mut self.connectors {
                c.crtc_id = None;
            }
            self.gems.clear();
            self.framebuffers.clear();
            self.vblank = VblankState::new();
        }
    }

    pub fn active_crtc_count(&self) -> usize {
        self.crtcs.iter().filter(|c| c.active).count()
    }

    pub fn connector_count(&self) -> usize {
        self.connectors.len()
    }

    /// Create a dumb BO (GSP-gated).
    pub fn create_dumb(
        &mut self,
        width: u32,
        height: u32,
        bpp: u32,
    ) -> Result<DumbCreateResult, DrmError> {
        self.gems
            .create_dumb(
                self.gsp_online,
                &DumbCreateRequest {
                    width,
                    height,
                    bpp,
                },
            )
            .map_err(DrmError::Gem)
    }

    /// Create a framebuffer backed by an existing GEM handle.
    pub fn add_fb_from_gem(
        &mut self,
        gem_handle: u32,
        format: PixelFormat,
    ) -> Result<u32, DrmError> {
        if !self.gsp_online {
            return Err(DrmError::GspOffline);
        }
        let gem = self
            .gems
            .get(gem_handle)
            .ok_or(DrmError::Gem(GemError::InvalidHandle))?;
        let id = self.next_fb_id;
        self.next_fb_id = self.next_fb_id.wrapping_add(1).max(1);
        let fb = Framebuffer::from_gem(id, gem, format).map_err(DrmError::Framebuffer)?;
        self.framebuffers.push(fb);
        Ok(id)
    }

    pub fn destroy_dumb(&mut self, handle: u32) -> Result<(), DrmError> {
        self.gems.destroy(handle).map_err(DrmError::Gem)
    }

    pub fn gem_flink(&mut self, handle: u32) -> Result<u32, DrmError> {
        self.gems
            .flink(self.gsp_online, handle)
            .map_err(DrmError::Gem)
    }

    pub fn gem_open_name(&mut self, name: u32) -> Result<u32, DrmError> {
        self.gems
            .open_name(self.gsp_online, name)
            .map_err(DrmError::Gem)
    }

    pub fn gem_prime_export(
        &mut self,
        handle: u32,
    ) -> Result<crate::gem::PrimeExport, DrmError> {
        self.gems
            .prime_export(self.gsp_online, handle)
            .map_err(DrmError::Gem)
    }

    pub fn gem_prime_import(&mut self, token: u64) -> Result<u32, DrmError> {
        self.gems
            .prime_import(self.gsp_online, token)
            .map_err(DrmError::Gem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_desktop_shape() {
        let d = DrmDevice::virtual_desktop(true);
        assert_eq!(d.connectors.len(), 1);
        assert_eq!(d.crtcs.len(), 1);
        assert_eq!(d.planes.len(), 1);
        assert!(d.gsp_online);
    }

    #[test]
    fn virtual_dual_head_shape() {
        let d = DrmDevice::virtual_dual_head(true);
        assert_eq!(d.connector_count(), 2);
        assert_eq!(d.crtcs.len(), 2);
        assert_eq!(d.planes.len(), 2);
    }

    #[test]
    fn offline_clears_active_state() {
        let mut d = DrmDevice::virtual_desktop(true);
        d.crtcs[0].active = true;
        d.set_gsp_online(false);
        assert!(!d.gsp_online);
        assert_eq!(d.active_crtc_count(), 0);
    }

    #[test]
    fn dumb_to_fb_pipeline() {
        let mut d = DrmDevice::virtual_desktop(true);
        let dumb = d.create_dumb(1920, 1080, 32).unwrap();
        let fb_id = d
            .add_fb_from_gem(dumb.handle, PixelFormat::Xrgb8888)
            .unwrap();
        assert_eq!(fb_id, 1);
        assert_eq!(d.framebuffers[0].bo_handle, dumb.handle as u64);
        d.set_gsp_online(false);
        assert_eq!(d.gems.count(), 0);
        assert!(d.framebuffers.is_empty());
    }
}
