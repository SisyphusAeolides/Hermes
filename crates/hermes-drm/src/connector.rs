//! DRM connectors (outputs).

use crate::mode::DisplayMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ConnectorType {
    Unknown = 0,
    Vga = 1,
    Dvi = 2,
    Hdmi = 3,
    DisplayPort = 4,
    Edp = 5,
    Lvds = 6,
    Virtual = 7,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ConnectorStatus {
    Disconnected = 0,
    Connected = 1,
    Unknown = 2,
}

#[derive(Clone, Debug)]
pub struct Connector {
    pub id: u32,
    pub connector_type: ConnectorType,
    pub status: ConnectorStatus,
    pub modes: alloc::vec::Vec<DisplayMode>,
    pub encoder_id: u32,
    pub crtc_id: Option<u32>,
    /// Property blob id for EDID (if attached on device PropertyStore).
    pub edid_blob_id: Option<u32>,
}

impl Connector {
    pub fn virtual_fhd(id: u32) -> Self {
        Self {
            id,
            connector_type: ConnectorType::Virtual,
            status: ConnectorStatus::Connected,
            modes: alloc::vec![DisplayMode::fhd_60(), DisplayMode::hd_60()],
            encoder_id: id,
            crtc_id: None,
            edid_blob_id: None,
        }
    }

    pub fn preferred_mode(&self) -> Option<DisplayMode> {
        self.modes.first().copied()
    }
}
