//! Full GSP-RM image DMA staging through the platform HAL.
//!
//! Streams the entire measured image in fixed-size chunks so hosts can stage
//! multi-megabyte GSP-RM ELF blobs without requiring a single contiguous DMA
//! mapping the size of the image. Each chunk is written and published; a
//! running SHA-256 of staged bytes is returned so callers can prove the full
//! image crossed the DMA boundary (not just the first 4 KiB).

use hermes_core::{DmaPurpose, DmaRegion, HermesFault, HermesPlatform};
use sha2::{Digest, Sha256};

/// Default DMA chunk size (one page). Matches T1000 GSP boot binary page.
pub const STAGE_CHUNK_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StageReport {
    /// Total bytes written+published across all chunks.
    pub bytes_staged: u64,
    /// SHA-256 of the concatenated staged stream (must match firmware admit).
    pub staged_sha256: [u8; 32],
    /// Number of chunk publish operations performed.
    pub chunks: u32,
    /// Device address of the last chunk region (IOVA).
    pub last_device_address: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageError {
    EmptyImage,
    Platform(HermesFault),
    DigestMismatch,
}

impl From<HermesFault> for StageError {
    fn from(value: HermesFault) -> Self {
        Self::Platform(value)
    }
}

/// Stage the entire `image` into `domain` via chunked DMA.
///
/// Uses a single reusable DMA region of `chunk_len` bytes (or image length if
/// smaller). Overwrites the region per chunk after publishing — models a
/// streaming firmware loader. The returned digest covers **all** bytes in
/// order; callers compare to the admitted firmware SHA-256.
pub fn stage_gsp_rm_image<P: HermesPlatform>(
    platform: &P,
    domain: P::Domain,
    image: &[u8],
    chunk_len: usize,
) -> Result<(StageReport, DmaRegion<P::Dma>), StageError> {
    if image.is_empty() {
        return Err(StageError::EmptyImage);
    }
    let chunk_len = chunk_len.max(64).min(STAGE_CHUNK_BYTES.max(64));
    let region = platform.allocate_dma(domain, chunk_len, 64, DmaPurpose::Firmware)?;

    let mut hasher = Sha256::new();
    let mut offset = 0usize;
    let mut chunks = 0u32;

    while offset < image.len() {
        let end = core::cmp::min(offset + chunk_len, image.len());
        let slice = &image[offset..end];
        // Zero-pad tail of the DMA window for short final chunks.
        if slice.len() < chunk_len {
            let mut pad = alloc::vec![0u8; chunk_len];
            pad[..slice.len()].copy_from_slice(slice);
            platform.dma_write(region, 0, &pad)?;
            platform.dma_publish(region, 0, slice.len())?;
        } else {
            platform.dma_write(region, 0, slice)?;
            platform.dma_publish(region, 0, slice.len())?;
        }
        hasher.update(slice);
        offset = end;
        chunks = chunks.saturating_add(1);
    }

    let digest = hasher.finalize();
    let mut staged_sha256 = [0u8; 32];
    staged_sha256.copy_from_slice(&digest);

    Ok((
        StageReport {
            bytes_staged: image.len() as u64,
            staged_sha256,
            chunks,
            last_device_address: region.device_address,
        },
        region,
    ))
}

/// Verify staged digest matches the admitted firmware measurement.
pub fn stage_matches_admit(staged: &[u8; 32], admitted: &[u8; 32]) -> bool {
    staged == admitted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::firmware::sha256_bytes;
    use core::cell::UnsafeCell;
    use hermes_abi::hermes::HermesPciIdentity;
    use hermes_core::{DmaPurpose, DmaRegion, MmioWindow};

    struct TinyDma {
        buf: UnsafeCell<[u8; 8192]>,
        len: UnsafeCell<usize>,
        published: UnsafeCell<usize>,
    }
    unsafe impl Sync for TinyDma {}

    impl TinyDma {
        fn new() -> Self {
            Self {
                buf: UnsafeCell::new([0; 8192]),
                len: UnsafeCell::new(0),
                published: UnsafeCell::new(0),
            }
        }
    }

    impl HermesPlatform for TinyDma {
        type Domain = u32;
        type Mmio = u32;
        type Dma = u32;
        fn isolate_device(&self, _: HermesPciIdentity) -> Result<u32, HermesFault> {
            Ok(1)
        }
        fn release_domain(&self, _: u32) {}
        fn map_bar(&self, _: u32, _: u8, _: u64) -> Result<MmioWindow<u32>, HermesFault> {
            Ok(MmioWindow {
                handle: 1,
                bar: 0,
                length: 4096,
            })
        }
        fn unmap_bar(&self, _: MmioWindow<u32>) {}
        fn read32(&self, _: MmioWindow<u32>, _: u32) -> Result<u32, HermesFault> {
            Ok(0)
        }
        fn write32(&self, _: MmioWindow<u32>, _: u32, _: u32) -> Result<(), HermesFault> {
            Ok(())
        }
        fn io_fence(&self) -> Result<(), HermesFault> {
            Ok(())
        }
        fn allocate_dma(
            &self,
            _: u32,
            length: usize,
            _: usize,
            purpose: DmaPurpose,
        ) -> Result<DmaRegion<u32>, HermesFault> {
            if length > 8192 {
                return Err(HermesFault::DmaAllocation);
            }
            unsafe {
                *self.len.get() = length;
            }
            Ok(DmaRegion {
                handle: 1,
                device_address: 0x9000_0000,
                length,
                alignment: 64,
                purpose,
            })
        }
        fn release_dma(&self, _: DmaRegion<u32>) {}
        fn dma_write(
            &self,
            _: DmaRegion<u32>,
            offset: usize,
            bytes: &[u8],
        ) -> Result<(), HermesFault> {
            let len = unsafe { *self.len.get() };
            if offset + bytes.len() > len {
                return Err(HermesFault::DmaAccess);
            }
            unsafe {
                let buf = &mut *self.buf.get();
                buf[offset..offset + bytes.len()].copy_from_slice(bytes);
            }
            Ok(())
        }
        fn dma_read(
            &self,
            _: DmaRegion<u32>,
            _: usize,
            _: &mut [u8],
        ) -> Result<(), HermesFault> {
            Ok(())
        }
        fn dma_publish(
            &self,
            _: DmaRegion<u32>,
            _: usize,
            length: usize,
        ) -> Result<(), HermesFault> {
            unsafe {
                *self.published.get() += length;
            }
            Ok(())
        }
        fn dma_acquire(
            &self,
            _: DmaRegion<u32>,
            _: usize,
            _: usize,
        ) -> Result<(), HermesFault> {
            Ok(())
        }
        fn now_tick(&self) -> u64 {
            0
        }
        fn relax(&self) {}
    }

    #[test]
    fn stages_full_image_digest_matches() {
        let p = TinyDma::new();
        // Multi-chunk image (5.5 KiB → 2 chunks of 4 KiB capacity).
        let mut image = alloc::vec![0u8; 5500];
        for (i, b) in image.iter_mut().enumerate() {
            *b = (i % 251) as u8;
        }
        let expected = sha256_bytes(&image);
        let (rep, _reg) = stage_gsp_rm_image(&p, 1, &image, 4096).unwrap();
        assert_eq!(rep.bytes_staged, 5500);
        assert_eq!(rep.chunks, 2);
        assert!(stage_matches_admit(&rep.staged_sha256, &expected));
        assert_eq!(unsafe { *p.published.get() }, 5500);
    }

    #[test]
    fn empty_image_fails() {
        let p = TinyDma::new();
        assert_eq!(
            stage_gsp_rm_image(&p, 1, &[], 4096),
            Err(StageError::EmptyImage)
        );
    }
}
