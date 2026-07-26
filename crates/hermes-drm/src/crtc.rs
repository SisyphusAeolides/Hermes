//! CRTCs (scanout pipes).

use crate::mode::DisplayMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Crtc {
    pub id: u32,
    pub primary_plane_id: u32,
    pub active: bool,
    pub mode: Option<DisplayMode>,
    pub fb_id: Option<u32>,
    pub x: u32,
    pub y: u32,
}

impl Crtc {
    pub const fn new(id: u32, primary_plane_id: u32) -> Self {
        Self {
            id,
            primary_plane_id,
            active: false,
            mode: None,
            fb_id: None,
            x: 0,
            y: 0,
        }
    }
}
