//! Stable ABI surface for Hermes GSP and portable GPU compatibility.
//!
//! These contracts are intentionally `no_std` so Boulder, Linux personalities,
//! and host unit tests share one definition of the wire layout.

#![no_std]

pub mod gpu;
pub mod hermes;

pub use gpu::*;
pub use hermes::*;
