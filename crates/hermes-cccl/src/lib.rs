//! CCCL reverse engineering for Hermes CUDA compatibility.
//!
//! Source: https://github.com/NVIDIA/cccl (Thrust, CUB, libcudacxx)
//! Regenerated tables: `scripts/reverse-engineer-cccl.py`
//!
//! Host-side Thrust algorithm subset runs on CPU today; device dispatch is
//! wired through `hermes-cuda` once GSP Online + context exist.

#![no_std]

extern crate alloc;

pub mod cub_modules;
pub mod host_thrust;
pub mod thrust_algorithms;

pub use cub_modules::{cub_layers, cub_module_count, CubModule, CUB_MODULES};
pub use host_thrust::{
    hermes_copy, hermes_count, hermes_equal, hermes_fill, hermes_find, hermes_for_each,
    hermes_reduce, hermes_replace, hermes_scan_inclusive, hermes_sequence, hermes_sort,
    hermes_transform, hermes_unique,
};
pub use thrust_algorithms::{
    is_public_thrust_header, thrust_header_count, CCCL_VERSION, HERMES_HOST_IMPLEMENTED,
    THRUST_PUBLIC_HEADERS,
};
