//! DRM/KMS atomic modeset foundation (clean-room, Nouveau-shaped).
//!
//! Display commits require Hermes GSP Online. This is not a full kernel DRM
//! driver yet — it is the in-tree modeset state machine Hermes will export
//! through `nvidia-drm` / a future DRM character device.

#![no_std]

extern crate alloc;

pub mod atomic;
pub mod connector;
pub mod crtc;
pub mod device;
pub mod framebuffer;
pub mod gem;
pub mod mode;
pub mod pageflip;
pub mod plane;

pub use atomic::{AtomicCommit, AtomicRequest, CommitError, CommitResult};
pub use connector::{Connector, ConnectorStatus, ConnectorType};
pub use crtc::Crtc;
pub use device::{DrmDevice, DrmError};
pub use framebuffer::{Framebuffer, FramebufferError, PixelFormat};
pub use gem::{DumbCreateRequest, DumbCreateResult, GemError, GemManager, GemObject};
pub use mode::DisplayMode;
pub use pageflip::{page_flip, FlipError, PageFlipRequest, VblankEvent, VblankState};
pub use plane::{Plane, PlaneType};
