//! DRM device aggregate — connectors, CRTCs, planes, framebuffers, GEM.

use alloc::vec::Vec;

use crate::connector::Connector;
use crate::crtc::Crtc;
use crate::edid::build_base_edid;
use crate::framebuffer::{Framebuffer, FramebufferError, PixelFormat};
use crate::gem::{DumbCreateRequest, DumbCreateResult, GemError, GemManager};
use crate::mode::DisplayMode;
use crate::pageflip::VblankState;
use crate::plane::Plane;
use crate::property::{PropType, PropertyStore};

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
    pub props: PropertyStore,
    /// Property id for connector EDID blob values.
    pub edid_prop_id: Option<u32>,
    next_fb_id: u32,
}

impl DrmDevice {
    /// Minimal virtual desktop topology (1 CRTC, 1 primary plane, 1 connector).
    pub fn virtual_desktop(gsp_online: bool) -> Self {
        let mut dev = Self {
            gsp_online,
            connectors: alloc::vec![Connector::virtual_fhd(1)],
            crtcs: alloc::vec![Crtc::new(1, 1)],
            planes: alloc::vec![Plane::primary(1, 0b1)],
            framebuffers: Vec::new(),
            gems: GemManager::new(),
            vblank: VblankState::new(),
            props: PropertyStore::new(),
            edid_prop_id: None,
            next_fb_id: 1,
        };
        if gsp_online {
            let _ = dev.attach_synthetic_edid();
        }
        dev
    }

    /// Dual-head virtual topology (2 CRTCs / planes / connectors).
    pub fn virtual_dual_head(gsp_online: bool) -> Self {
        let mut dev = Self {
            gsp_online,
            connectors: alloc::vec![Connector::virtual_fhd(1), Connector::virtual_fhd(2),],
            crtcs: alloc::vec![Crtc::new(1, 1), Crtc::new(2, 2)],
            planes: alloc::vec![Plane::primary(1, 0b01), Plane::primary(2, 0b10),],
            framebuffers: Vec::new(),
            gems: GemManager::new(),
            vblank: VblankState::new(),
            props: PropertyStore::new(),
            edid_prop_id: None,
            next_fb_id: 1,
        };
        if gsp_online {
            let _ = dev.attach_synthetic_edid();
        }
        dev
    }

    /// Attach synthetic EDID blobs to all connectors (GSP Online only).
    pub fn attach_synthetic_edid(&mut self) -> Result<(), DrmError> {
        if !self.gsp_online {
            return Err(DrmError::GspOffline);
        }
        if self.edid_prop_id.is_none() {
            self.edid_prop_id = Some(self.props.create_prop("EDID", PropType::Blob, Vec::new()));
        }
        let prop_id = self.edid_prop_id.unwrap();
        let mut pairs: Vec<(u32, DisplayMode)> = Vec::new();
        for c in &self.connectors {
            let mode = c.preferred_mode().unwrap_or_else(DisplayMode::fhd_60);
            pairs.push((c.id, mode));
        }
        for (conn_id, mode) in pairs {
            let name = alloc::format!("Hermes-{conn_id}");
            let blob = build_base_edid(mode, &name);
            let blob_id = self.props.create_blob(blob);
            self.props.set(conn_id, prop_id, blob_id as u64);
            if let Some(c) = self.connectors.iter_mut().find(|c| c.id == conn_id) {
                c.edid_blob_id = Some(blob_id);
            }
        }
        Ok(())
    }

    /// Read EDID blob bytes for a connector.
    pub fn connector_edid(&self, connector_id: u32) -> Option<&[u8]> {
        let c = self.connectors.iter().find(|c| c.id == connector_id)?;
        let blob_id = c.edid_blob_id?;
        self.props.blob(blob_id).map(|b| b.data.as_slice())
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
                c.edid_blob_id = None;
            }
            self.gems.clear();
            self.framebuffers.clear();
            self.props = PropertyStore::new();
            self.edid_prop_id = None;
            self.vblank = VblankState::new();
        } else if self.edid_prop_id.is_none() {
            let _ = self.attach_synthetic_edid();
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
            .create_dumb(self.gsp_online, &DumbCreateRequest { width, height, bpp })
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

    pub fn gem_prime_export(&mut self, handle: u32) -> Result<crate::gem::PrimeExport, DrmError> {
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
    fn virtual_desktop_has_edid_when_online() {
        let d = DrmDevice::virtual_desktop(true);
        let edid = d.connector_edid(1).expect("edid");
        assert!(crate::edid::edid_checksum_ok(edid));
        assert_eq!(crate::edid::edid_preferred_size(edid), Some((1920, 1080)));
        let off = DrmDevice::virtual_desktop(false);
        assert!(off.connector_edid(1).is_none());
    }

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
