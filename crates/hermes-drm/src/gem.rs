//! GEM-like buffer objects and DRM dumb-buffer creation.
//!
//! Host-side model until a real GEM/TTM backend exists. Handles are stable
//! ids; mapping returns a host slice for software scanout / present.

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GemError {
    GspOffline,
    InvalidSize,
    InvalidHandle,
    InvalidName,
    Busy,
    OutOfMemory,
}

/// Software PRIME / dma-buf-shaped export token (not a real kernel fd).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimeExport {
    pub handle: u32,
    pub size: u64,
    pub token: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumbCreateRequest {
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumbCreateResult {
    pub handle: u32,
    pub pitch: u32,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct GemObject {
    pub handle: u32,
    pub size: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u32,
    /// Host backing store (software BO).
    pub data: Vec<u8>,
    pub name: Option<u32>,
    pub refcount: u32,
}

impl GemObject {
    pub fn create_dumb(
        handle: u32,
        req: &DumbCreateRequest,
    ) -> Result<Self, GemError> {
        if req.width == 0 || req.height == 0 || req.bpp == 0 {
            return Err(GemError::InvalidSize);
        }
        let bpp_bytes = req.bpp.div_ceil(8);
        let pitch = req
            .width
            .checked_mul(bpp_bytes)
            .ok_or(GemError::InvalidSize)?;
        // Align pitch to 64 bytes (common DRM dumb convention).
        let pitch = pitch.div_ceil(64) * 64;
        let size = (pitch as u64)
            .checked_mul(req.height as u64)
            .ok_or(GemError::InvalidSize)?;
        if size > 512 * 1024 * 1024 {
            return Err(GemError::OutOfMemory);
        }
        Ok(Self {
            handle,
            size,
            pitch,
            width: req.width,
            height: req.height,
            bpp: req.bpp,
            data: alloc::vec![0u8; size as usize],
            name: None,
            refcount: 1,
        })
    }

    pub fn fill_solid_xrgb8888(&mut self, color: u32) -> Result<(), GemError> {
        if self.bpp != 32 {
            return Err(GemError::InvalidSize);
        }
        let bytes = color.to_le_bytes();
        for chunk in self.data.chunks_exact_mut(4) {
            chunk.copy_from_slice(&bytes);
        }
        Ok(())
    }

    pub fn map(&self) -> &[u8] {
        &self.data
    }

    pub fn map_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[derive(Clone, Debug, Default)]
pub struct GemManager {
    next_handle: u32,
    next_name: u32,
    next_prime_token: u64,
    objects: Vec<GemObject>,
    /// PRIME token → source handle at export time.
    prime_tokens: Vec<(u64, u32)>,
}

impl GemManager {
    pub const fn new() -> Self {
        Self {
            next_handle: 1,
            next_name: 1,
            next_prime_token: 1,
            objects: Vec::new(),
            prime_tokens: Vec::new(),
        }
    }

    fn alloc_handle(&mut self) -> u32 {
        let h = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1).max(1);
        h
    }

    pub fn create_dumb(
        &mut self,
        gsp_online: bool,
        req: &DumbCreateRequest,
    ) -> Result<DumbCreateResult, GemError> {
        if !gsp_online {
            return Err(GemError::GspOffline);
        }
        let handle = self.alloc_handle();
        let obj = GemObject::create_dumb(handle, req)?;
        let result = DumbCreateResult {
            handle: obj.handle,
            pitch: obj.pitch,
            size: obj.size,
        };
        self.objects.push(obj);
        Ok(result)
    }

    pub fn destroy(&mut self, handle: u32) -> Result<(), GemError> {
        let before = self.objects.len();
        self.objects.retain(|o| o.handle != handle);
        self.prime_tokens.retain(|(_, h)| *h != handle);
        if self.objects.len() == before {
            Err(GemError::InvalidHandle)
        } else {
            Ok(())
        }
    }

    /// DRM_IOCTL_GEM_FLINK — publish a global name for a BO.
    pub fn flink(&mut self, gsp_online: bool, handle: u32) -> Result<u32, GemError> {
        if !gsp_online {
            return Err(GemError::GspOffline);
        }
        if let Some(n) = self
            .objects
            .iter()
            .find(|o| o.handle == handle)
            .and_then(|o| o.name)
        {
            return Ok(n);
        }
        if !self.objects.iter().any(|o| o.handle == handle) {
            return Err(GemError::InvalidHandle);
        }
        let n = self.next_name;
        self.next_name = self.next_name.wrapping_add(1).max(1);
        if let Some(obj) = self.objects.iter_mut().find(|o| o.handle == handle) {
            obj.name = Some(n);
        }
        Ok(n)
    }

    /// DRM_IOCTL_GEM_OPEN — open BO by global name; returns a new handle (data clone).
    pub fn open_name(&mut self, gsp_online: bool, name: u32) -> Result<u32, GemError> {
        if !gsp_online {
            return Err(GemError::GspOffline);
        }
        let src = self
            .objects
            .iter()
            .find(|o| o.name == Some(name))
            .ok_or(GemError::InvalidName)?
            .clone();
        let handle = self.alloc_handle();
        self.objects.push(GemObject {
            handle,
            size: src.size,
            pitch: src.pitch,
            width: src.width,
            height: src.height,
            bpp: src.bpp,
            data: src.data,
            name: Some(name),
            refcount: 1,
        });
        Ok(handle)
    }

    /// Software PRIME export: opaque token for later import.
    pub fn prime_export(&mut self, gsp_online: bool, handle: u32) -> Result<PrimeExport, GemError> {
        if !gsp_online {
            return Err(GemError::GspOffline);
        }
        let size = self
            .objects
            .iter()
            .find(|o| o.handle == handle)
            .map(|o| o.size)
            .ok_or(GemError::InvalidHandle)?;
        let token = self.next_prime_token;
        self.next_prime_token = self.next_prime_token.wrapping_add(1).max(1);
        self.prime_tokens.push((token, handle));
        Ok(PrimeExport {
            handle,
            size,
            token,
        })
    }

    /// Import a prior PRIME token as a new handle (cloned backing).
    pub fn prime_import(&mut self, gsp_online: bool, token: u64) -> Result<u32, GemError> {
        if !gsp_online {
            return Err(GemError::GspOffline);
        }
        let src_handle = self
            .prime_tokens
            .iter()
            .find(|(t, _)| *t == token)
            .map(|(_, h)| *h)
            .ok_or(GemError::InvalidHandle)?;
        let src = self
            .objects
            .iter()
            .find(|o| o.handle == src_handle)
            .ok_or(GemError::InvalidHandle)?
            .clone();
        let handle = self.alloc_handle();
        self.objects.push(GemObject {
            handle,
            size: src.size,
            pitch: src.pitch,
            width: src.width,
            height: src.height,
            bpp: src.bpp,
            data: src.data,
            name: None,
            refcount: 1,
        });
        Ok(handle)
    }

    pub fn get(&self, handle: u32) -> Option<&GemObject> {
        self.objects.iter().find(|o| o.handle == handle)
    }

    pub fn get_mut(&mut self, handle: u32) -> Option<&mut GemObject> {
        self.objects.iter_mut().find(|o| o.handle == handle)
    }

    pub fn count(&self) -> usize {
        self.objects.len()
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.prime_tokens.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dumb_create_aligns_pitch() {
        let mut m = GemManager::new();
        let r = m
            .create_dumb(
                true,
                &DumbCreateRequest {
                    width: 1920,
                    height: 1080,
                    bpp: 32,
                },
            )
            .unwrap();
        assert_eq!(r.handle, 1);
        assert_eq!(r.pitch % 64, 0);
        assert!(r.size >= 1920 * 1080 * 4);
        assert_eq!(m.count(), 1);
    }

    #[test]
    fn dumb_requires_gsp() {
        let mut m = GemManager::new();
        assert_eq!(
            m.create_dumb(
                false,
                &DumbCreateRequest {
                    width: 64,
                    height: 64,
                    bpp: 32,
                }
            ),
            Err(GemError::GspOffline)
        );
    }

    #[test]
    fn fill_and_destroy() {
        let mut m = GemManager::new();
        let r = m
            .create_dumb(
                true,
                &DumbCreateRequest {
                    width: 16,
                    height: 16,
                    bpp: 32,
                },
            )
            .unwrap();
        m.get_mut(r.handle)
            .unwrap()
            .fill_solid_xrgb8888(0x00ff_0000)
            .unwrap();
        assert_eq!(m.get(r.handle).unwrap().data[0], 0x00);
        assert_eq!(m.get(r.handle).unwrap().data[2], 0xff);
        m.destroy(r.handle).unwrap();
        assert_eq!(m.count(), 0);
    }

    #[test]
    fn flink_open_and_prime() {
        let mut m = GemManager::new();
        let r = m
            .create_dumb(
                true,
                &DumbCreateRequest {
                    width: 32,
                    height: 32,
                    bpp: 32,
                },
            )
            .unwrap();
        m.get_mut(r.handle)
            .unwrap()
            .fill_solid_xrgb8888(0x0000_00ff)
            .unwrap();
        let name = m.flink(true, r.handle).unwrap();
        let h2 = m.open_name(true, name).unwrap();
        assert_ne!(h2, r.handle);
        assert_eq!(m.get(h2).unwrap().data[0], 0xff);
        let exp = m.prime_export(true, r.handle).unwrap();
        assert_eq!(exp.size, r.size);
        let h3 = m.prime_import(true, exp.token).unwrap();
        assert_eq!(m.get(h3).unwrap().data[0], 0xff);
        assert_eq!(m.flink(false, r.handle), Err(GemError::GspOffline));
        assert_eq!(m.open_name(true, 9999), Err(GemError::InvalidName));
    }
}
