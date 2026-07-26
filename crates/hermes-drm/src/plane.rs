//! DRM planes (primary / overlay / cursor).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PlaneType {
    Primary = 0,
    Overlay = 1,
    Cursor = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plane {
    pub id: u32,
    pub plane_type: PlaneType,
    pub possible_crtcs: u32,
    pub crtc_id: Option<u32>,
    pub fb_id: Option<u32>,
    pub src_x: u32,
    pub src_y: u32,
    pub src_w: u32,
    pub src_h: u32,
    pub crtc_x: i32,
    pub crtc_y: i32,
    pub crtc_w: u32,
    pub crtc_h: u32,
}

impl Plane {
    pub const fn primary(id: u32, crtc_mask: u32) -> Self {
        Self {
            id,
            plane_type: PlaneType::Primary,
            possible_crtcs: crtc_mask,
            crtc_id: None,
            fb_id: None,
            src_x: 0,
            src_y: 0,
            src_w: 0,
            src_h: 0,
            crtc_x: 0,
            crtc_y: 0,
            crtc_w: 0,
            crtc_h: 0,
        }
    }
}
