//! Asynchronous page-flip and software vblank sequencing.

use crate::atomic::{AtomicCommit, AtomicRequest, CommitError, CommitResult};
use crate::device::DrmDevice;
use crate::mode::DisplayMode;
use hermes_core::chaos::ChaosScheduler;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlipError {
    GspOffline,
    InvalidCrtc,
    InvalidFramebuffer,
    NotActive,
    Modeset(CommitError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageFlipRequest {
    pub crtc_id: u32,
    pub fb_id: u32,
    pub flags: u32,
}

impl PageFlipRequest {
    pub const FLAG_EVENT: u32 = 1 << 0;
    pub const FLAG_ASYNC: u32 = 1 << 1;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VblankEvent {
    pub crtc_id: u32,
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub fb_id: u32,
}

#[derive(Clone, Debug, Default)]
pub struct VblankState {
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub pending_events: alloc::vec::Vec<VblankEvent>,
    /// Host-side phase decorrelation for page-flip queue turns.  This value
    /// never changes the hardware vblank period or grants display authority.
    pub service_quantum_us: u32,
    scheduler: ChaosScheduler,
}

impl VblankState {
    pub const fn new() -> Self {
        Self {
            sequence: 0,
            timestamp_ns: 0,
            pending_events: alloc::vec::Vec::new(),
            service_quantum_us: 1,
            scheduler: ChaosScheduler::new(),
        }
    }

    /// Advance software vblank clock (≈16.67 ms for 60 Hz).
    pub fn tick(&mut self, crtc_id: u32, fb_id: u32, period_ns: u64) -> VblankEvent {
        self.service_quantum_us = self.scheduler.next_interval(0.01);
        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp_ns = self.timestamp_ns.wrapping_add(period_ns);
        let ev = VblankEvent {
            crtc_id,
            sequence: self.sequence,
            timestamp_ns: self.timestamp_ns,
            fb_id,
        };
        self.pending_events.push(ev);
        ev
    }

    pub fn pop_event(&mut self) -> Option<VblankEvent> {
        if self.pending_events.is_empty() {
            None
        } else {
            Some(self.pending_events.remove(0))
        }
    }
}

/// Queue a page flip onto an already-active CRTC (atomic rebind of FB).
pub fn page_flip(
    atom: &mut AtomicCommit,
    device: &mut DrmDevice,
    req: &PageFlipRequest,
) -> Result<CommitResult, FlipError> {
    if !device.gsp_online {
        return Err(FlipError::GspOffline);
    }
    let crtc = device
        .crtcs
        .iter()
        .find(|c| c.id == req.crtc_id)
        .ok_or(FlipError::InvalidCrtc)?;
    if !crtc.active {
        return Err(FlipError::NotActive);
    }
    if !device.framebuffers.iter().any(|f| f.id == req.fb_id) {
        return Err(FlipError::InvalidFramebuffer);
    }

    let mode = crtc.mode.unwrap_or_else(DisplayMode::fhd_60);
    let connector_id = device
        .connectors
        .iter()
        .find(|c| c.crtc_id == Some(req.crtc_id))
        .map(|c| c.id)
        .unwrap_or(1);
    let plane_id = device
        .planes
        .iter()
        .find(|p| p.crtc_id == Some(req.crtc_id) || p.possible_crtcs != 0)
        .map(|p| p.id)
        .unwrap_or(1);

    let atomic_req = AtomicRequest {
        connector_id,
        crtc_id: req.crtc_id,
        plane_id,
        fb_id: req.fb_id,
        mode,
        active: true,
    };
    let result = atom
        .commit(device, &atomic_req)
        .map_err(FlipError::Modeset)?;

    let period = 16_666_667u64; // ~60 Hz software vblank
    let _ev = device.vblank.tick(req.crtc_id, req.fb_id, period);
    let _ = req.flags;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::DrmDevice;
    use crate::framebuffer::{Framebuffer, PixelFormat};
    use crate::mode::DisplayMode;

    #[test]
    fn flip_requires_active_crtc() {
        let mut dev = DrmDevice::virtual_desktop(true);
        let fb = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).unwrap();
        dev.framebuffers.push(fb);
        let mut atom = AtomicCommit::new();
        let req = PageFlipRequest {
            crtc_id: 1,
            fb_id: 10,
            flags: PageFlipRequest::FLAG_EVENT,
        };
        assert_eq!(
            page_flip(&mut atom, &mut dev, &req),
            Err(FlipError::NotActive)
        );
    }

    #[test]
    fn flip_after_modeset_emits_vblank() {
        let mut dev = DrmDevice::virtual_desktop(true);
        let fb1 = Framebuffer::new(10, 1920, 1080, PixelFormat::Xrgb8888, 1).unwrap();
        let fb2 = Framebuffer::new(11, 1920, 1080, PixelFormat::Xrgb8888, 2).unwrap();
        dev.framebuffers.push(fb1);
        dev.framebuffers.push(fb2);
        let mut atom = AtomicCommit::new();
        atom.commit(
            &mut dev,
            &AtomicRequest {
                connector_id: 1,
                crtc_id: 1,
                plane_id: 1,
                fb_id: 10,
                mode: DisplayMode::fhd_60(),
                active: true,
            },
        )
        .unwrap();
        let r = page_flip(
            &mut atom,
            &mut dev,
            &PageFlipRequest {
                crtc_id: 1,
                fb_id: 11,
                flags: PageFlipRequest::FLAG_EVENT,
            },
        )
        .unwrap();
        assert!(r.active);
        assert_eq!(dev.crtcs[0].fb_id, Some(11));
        let ev = dev.vblank.pop_event().unwrap();
        assert_eq!(ev.fb_id, 11);
        assert_eq!(ev.sequence, 1);
        assert!((1..=50).contains(&dev.vblank.service_quantum_us));
    }
}
