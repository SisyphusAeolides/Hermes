//! DRM framebuffer objects.

use crate::gem::GemObject;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PixelFormat {
    Xrgb8888 = 0x3432_5258, // DRM_FORMAT_XRGB8888 fourcc-ish tag
    Argb8888 = 0x3432_5241,
    Xbgr8888 = 0x3432_5242,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Xrgb8888 | Self::Argb8888 | Self::Xbgr8888 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Framebuffer {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub format: PixelFormat,
    /// GEM handle backing this FB (or opaque host BO id).
    pub bo_handle: u64,
}

impl Framebuffer {
    pub fn new(
        id: u32,
        width: u32,
        height: u32,
        format: PixelFormat,
        bo_handle: u64,
    ) -> Result<Self, FramebufferError> {
        if width == 0 || height == 0 {
            return Err(FramebufferError::InvalidSize);
        }
        let pitch = width
            .checked_mul(format.bytes_per_pixel())
            .ok_or(FramebufferError::InvalidSize)?;
        Ok(Self {
            id,
            width,
            height,
            pitch,
            format,
            bo_handle,
        })
    }

    /// Bind a GEM dumb buffer as a framebuffer.
    pub fn from_gem(
        id: u32,
        gem: &GemObject,
        format: PixelFormat,
    ) -> Result<Self, FramebufferError> {
        if gem.width == 0 || gem.height == 0 {
            return Err(FramebufferError::InvalidSize);
        }
        if gem.bpp < format.bytes_per_pixel() * 8 {
            return Err(FramebufferError::FormatMismatch);
        }
        Ok(Self {
            id,
            width: gem.width,
            height: gem.height,
            pitch: gem.pitch,
            format,
            bo_handle: gem.handle as u64,
        })
    }

    pub fn size_bytes(self) -> u64 {
        (self.pitch as u64) * (self.height as u64)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FramebufferError {
    InvalidSize,
    FormatMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fb_pitch_and_size() {
        let fb = Framebuffer::new(1, 1920, 1080, PixelFormat::Xrgb8888, 0x1000).unwrap();
        assert_eq!(fb.pitch, 1920 * 4);
        assert_eq!(fb.size_bytes(), 1920 * 1080 * 4);
    }
}
