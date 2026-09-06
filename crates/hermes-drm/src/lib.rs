//! DRM/KMS atomic modeset foundation (open-source, Nouveau-shaped).
//!
//! Display commits require Hermes GSP Online. The in-tree modeset state
//! machine is exported through the `nvidia-drm` compatibility surface.

#![no_std]

extern crate alloc;

pub mod atomic;
pub mod connector;
pub mod crtc;
pub mod device;
pub mod edid;
pub mod framebuffer;
pub mod gem;
pub mod mode;
pub mod pageflip;
pub mod plane;
pub mod property;

pub use atomic::{AtomicCommit, AtomicRequest, CommitError, CommitResult};
pub use connector::{Connector, ConnectorStatus, ConnectorType};
pub use crtc::Crtc;
pub use device::{DrmDevice, DrmError};
pub use edid::{build_base_edid, edid_checksum_ok, edid_preferred_size};
pub use framebuffer::{Framebuffer, FramebufferError, PixelFormat};
pub use gem::{DumbCreateRequest, DumbCreateResult, GemError, GemManager, GemObject, PrimeExport};
pub use mode::DisplayMode;
pub use pageflip::{page_flip, FlipError, PageFlipRequest, VblankEvent, VblankState};
pub use plane::{Plane, PlaneType};
pub use property::{PropType, Property, PropertyStore};
