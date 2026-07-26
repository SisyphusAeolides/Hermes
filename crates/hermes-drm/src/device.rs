//! DRM device aggregate — connectors, CRTCs, planes, framebuffers.

use alloc::vec::Vec;

use crate::connector::Connector;
use crate::crtc::Crtc;
use crate::framebuffer::Framebuffer;
use crate::plane::Plane;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DrmError {
    GspOffline,
    NotFound,
    Busy,
}

#[derive(Clone, Debug)]
pub struct DrmDevice {
    pub gsp_online: bool,
    pub connectors: Vec<Connector>,
    pub crtcs: Vec<Crtc>,
    pub planes: Vec<Plane>,
    pub framebuffers: Vec<Framebuffer>,
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
        }
    }

    pub fn active_crtc_count(&self) -> usize {
        self.crtcs.iter().filter(|c| c.active).count()
    }

    pub fn connector_count(&self) -> usize {
        self.connectors.len()
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
}
