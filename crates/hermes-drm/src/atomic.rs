//! Atomic modeset commit (DRM atomic uAPI shape).

use crate::device::DrmDevice;
use crate::mode::DisplayMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitError {
    GspOffline,
    InvalidConnector,
    InvalidCrtc,
    InvalidPlane,
    InvalidFramebuffer,
    ModeInvalid,
    PlaneNotCompatible,
    NotConnected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomicRequest {
    pub connector_id: u32,
    pub crtc_id: u32,
    pub plane_id: u32,
    pub fb_id: u32,
    pub mode: DisplayMode,
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitResult {
    pub sequence: u64,
    pub active: bool,
}

#[derive(Clone, Debug, Default)]
pub struct AtomicCommit {
    pub sequence: u64,
}

impl AtomicCommit {
    pub const fn new() -> Self {
        Self { sequence: 0 }
    }

    /// Test-only check + apply. Requires `device.gsp_online`.
    pub fn commit(
        &mut self,
        device: &mut DrmDevice,
        req: &AtomicRequest,
    ) -> Result<CommitResult, CommitError> {
        if !device.gsp_online {
            return Err(CommitError::GspOffline);
        }
        if !req.mode.valid() {
            return Err(CommitError::ModeInvalid);
        }

        let conn = device
            .connectors
            .iter()
            .find(|c| c.id == req.connector_id)
            .ok_or(CommitError::InvalidConnector)?;
        if conn.status != crate::connector::ConnectorStatus::Connected {
            return Err(CommitError::NotConnected);
        }

        let crtc_idx = device
            .crtcs
            .iter()
            .position(|c| c.id == req.crtc_id)
            .ok_or(CommitError::InvalidCrtc)?;
        let plane_idx = device
            .planes
            .iter()
            .position(|p| p.id == req.plane_id)
            .ok_or(CommitError::InvalidPlane)?;
        let fb = device
            .framebuffers
            .iter()
            .find(|f| f.id == req.fb_id)
            .ok_or(CommitError::InvalidFramebuffer)?;

        let plane = &device.planes[plane_idx];
        if plane.possible_crtcs & (1 << crtc_idx) == 0 {
            return Err(CommitError::PlaneNotCompatible);
        }

        // Apply
        {
            let plane = &mut device.planes[plane_idx];
            plane.crtc_id = Some(req.crtc_id);
            plane.fb_id = Some(req.fb_id);
            plane.src_w = fb.width << 16;
            plane.src_h = fb.height << 16;
            plane.crtc_w = req.mode.hdisplay as u32;
            plane.crtc_h = req.mode.vdisplay as u32;
        }
        {
            let crtc = &mut device.crtcs[crtc_idx];
            crtc.active = req.active;
            crtc.mode = Some(req.mode);
            crtc.fb_id = Some(req.fb_id);
        }
        if let Some(c) = device
            .connectors
            .iter_mut()
            .find(|c| c.id == req.connector_id)
        {
            c.crtc_id = Some(req.crtc_id);
        }

        self.sequence = self.sequence.wrapping_add(1);
        Ok(CommitResult {
            sequence: self.sequence,
            active: req.active,
        })
    }

    /// Disable a CRTC (blank) — still requires GSP Online.
    pub fn disable_crtc(
        &mut self,
        device: &mut DrmDevice,
        crtc_id: u32,
    ) -> Result<CommitResult, CommitError> {
        if !device.gsp_online {
            return Err(CommitError::GspOffline);
        }
        let crtc_idx = device
            .crtcs
            .iter()
            .position(|c| c.id == crtc_id)
            .ok_or(CommitError::InvalidCrtc)?;

        {
            let crtc = &mut device.crtcs[crtc_idx];
            crtc.active = false;
            crtc.mode = None;
            crtc.fb_id = None;
        }
        for p in &mut device.planes {
            if p.crtc_id == Some(crtc_id) {
                p.crtc_id = None;
                p.fb_id = None;
            }
        }
        for c in &mut device.connectors {
            if c.crtc_id == Some(crtc_id) {
                c.crtc_id = None;
            }
        }

        self.sequence = self.sequence.wrapping_add(1);
        Ok(CommitResult {
            sequence: self.sequence,
            active: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DrmDevice;
    use crate::framebuffer::{Framebuffer, PixelFormat};
    use crate::mode::DisplayMode;

    #[test]
    fn offline_commit_fails() {
        let mut dev = DrmDevice::virtual_desktop(false);
        let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).unwrap();
        dev.framebuffers.push(fb);
        let mut atom = AtomicCommit::new();
        let req = AtomicRequest {
            connector_id: 1,
            crtc_id: 1,
            plane_id: 1,
            fb_id: 10,
            mode: DisplayMode::fhd_60(),
            active: true,
        };
        assert_eq!(atom.commit(&mut dev, &req), Err(CommitError::GspOffline));
    }

    #[test]
    fn online_commit_succeeds() {
        let mut dev = DrmDevice::virtual_desktop(true);
        let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).unwrap();
        dev.framebuffers.push(fb);
        let mut atom = AtomicCommit::new();
        let req = AtomicRequest {
            connector_id: 1,
            crtc_id: 1,
            plane_id: 1,
            fb_id: 10,
            mode: DisplayMode::fhd_60(),
            active: true,
        };
        let r = atom.commit(&mut dev, &req).unwrap();
        assert!(r.active);
        assert_eq!(r.sequence, 1);
        assert!(dev.crtcs[0].active);
    }

    #[test]
    fn disable_crtc_after_commit() {
        let mut dev = DrmDevice::virtual_desktop(true);
        let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).unwrap();
        dev.framebuffers.push(fb);
        let mut atom = AtomicCommit::new();
        let req = AtomicRequest {
            connector_id: 1,
            crtc_id: 1,
            plane_id: 1,
            fb_id: 10,
            mode: DisplayMode::fhd_60(),
            active: true,
        };
        atom.commit(&mut dev, &req).unwrap();
        let r = atom.disable_crtc(&mut dev, 1).unwrap();
        assert!(!r.active);
        assert!(!dev.crtcs[0].active);
        assert!(dev.planes[0].fb_id.is_none());
    }

    #[test]
    fn dual_head_second_output() {
        let mut dev = DrmDevice::virtual_dual_head(true);
        let fb = Framebuffer::new(20, 1280, 720, PixelFormat::Xrgb8888, 2).unwrap();
        dev.framebuffers.push(fb);
        let mut atom = AtomicCommit::new();
        let req = AtomicRequest {
            connector_id: 2,
            crtc_id: 2,
            plane_id: 2,
            fb_id: 20,
            mode: DisplayMode::hd_60(),
            active: true,
        };
        let r = atom.commit(&mut dev, &req).unwrap();
        assert!(r.active);
        assert!(dev.crtcs[1].active);
        assert!(!dev.crtcs[0].active);
    }
}
