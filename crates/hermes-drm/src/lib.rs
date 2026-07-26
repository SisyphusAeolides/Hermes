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
pub mod framebuffer;
pub mod mode;
pub mod plane;
pub mod device;

pub use atomic::{AtomicCommit, AtomicRequest, CommitError, CommitResult};
pub use connector::{Connector, ConnectorStatus, ConnectorType};
pub use crtc::Crtc;
pub use device::{DrmDevice, DrmError};
pub use framebuffer::{Framebuffer, PixelFormat};
pub use mode::DisplayMode;
pub use plane::{Plane, PlaneType};
