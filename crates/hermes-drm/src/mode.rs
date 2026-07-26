//! Display mode timing (DRM modeinfo subset).

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayMode {
    pub clock_khz: u32,
    pub hdisplay: u16,
    pub hsync_start: u16,
    pub hsync_end: u16,
    pub htotal: u16,
    pub vdisplay: u16,
    pub vsync_start: u16,
    pub vsync_end: u16,
    pub vtotal: u16,
    pub flags: u32,
}

impl DisplayMode {
    pub const FLAG_PHSYNC: u32 = 1 << 0;
    pub const FLAG_NHSYNC: u32 = 1 << 1;
    pub const FLAG_PVSYNC: u32 = 1 << 2;
    pub const FLAG_NVSYNC: u32 = 1 << 3;
    pub const FLAG_INTERLACE: u32 = 1 << 4;

    /// Common 1920×1080 @ ~60 Hz modeline (simplified).
    pub const fn fhd_60() -> Self {
        Self {
            clock_khz: 148_500,
            hdisplay: 1920,
            hsync_start: 2008,
            hsync_end: 2052,
            htotal: 2200,
            vdisplay: 1080,
            vsync_start: 1084,
            vsync_end: 1089,
            vtotal: 1125,
            flags: Self::FLAG_PHSYNC | Self::FLAG_PVSYNC,
        }
    }

    pub const fn hd_60() -> Self {
        Self {
            clock_khz: 74_250,
            hdisplay: 1280,
            hsync_start: 1390,
            hsync_end: 1430,
            htotal: 1650,
            vdisplay: 720,
            vsync_start: 725,
            vsync_end: 730,
            vtotal: 750,
            flags: Self::FLAG_PHSYNC | Self::FLAG_PVSYNC,
        }
    }

    pub const fn valid(self) -> bool {
        self.hdisplay > 0
            && self.vdisplay > 0
            && self.htotal >= self.hsync_end
            && self.hsync_end >= self.hsync_start
            && self.hsync_start >= self.hdisplay
            && self.vtotal >= self.vsync_end
            && self.vsync_end >= self.vsync_start
            && self.vsync_start >= self.vdisplay
            && self.clock_khz > 0
    }

    pub fn refresh_hz_approx(self) -> u32 {
        if self.htotal == 0 || self.vtotal == 0 {
            return 0;
        }
        let denom = (self.htotal as u64) * (self.vtotal as u64);
        ((self.clock_khz as u64) * 1000 / denom) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fhd_mode_valid() {
        let m = DisplayMode::fhd_60();
        assert!(m.valid());
        assert!(m.refresh_hz_approx() >= 59 && m.refresh_hz_approx() <= 61);
    }
}
