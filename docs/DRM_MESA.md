# DRM/KMS and Mesa surfaces

## Crates

| Crate | Role |
|---|---|
| `hermes-drm` | Atomic modeset state machine (connector / CRTC / plane / FB) |
| `hermes-mesa` | Vulkan ICD-shaped loader + minimal GL + present path |

Both are **GSP-gated**: Offline never applies modeset or advertises a GPU.

## hermes-drm

- `DrmDevice::virtual_desktop` — single-head virtual topology
- `DrmDevice::virtual_dual_head` — dual CRTC / plane / connector
- `AtomicCommit::commit` — evidence-gated atomic apply
- `AtomicCommit::disable_crtc` — blank a pipe
- `GemManager` / `create_dumb` — GEM-like dumb buffers (pitch 64-aligned)
- `flink` / `open_name` / PRIME export-import tokens
- `add_fb_from_gem` — FB from dumb handle
- `page_flip` + software `VblankState` — flip with event sequence
- Synthetic EDID blobs + property store on Online virtual connectors
  (`drm-smoke edid`)

## Kernel `nvidia-drm.ko`

- Misc char device `/dev/nvidia-drm`
- Ioctls: `STATUS`, `DUMB_CREATE`, `ATOMIC`, `DISABLE_CRTC` (`hermes_drm_uapi.h`)
- Gates every call on `hermes_gsp_is_online()` exported by `nvidia.ko`
- Host-testable logic: `make -C linux/kmod host-test`

This is **not** full DRM subsystem registration yet (no `drm_device` / KMS
connector sysfs). It is the evidence-gated ioctl + userspace state machine pair.

## hermes-mesa

| Entry | Behavior Offline | Behavior Online |
|---|---|---|
| `vkCreateInstance` | `VK_ERROR_INCOMPATIBLE_DRIVER` | instance token |
| `vkEnumeratePhysicalDevices` | count 0 | one virtual device |
| `glGetError` | `GL_INVALID_OPERATION` | `GL_NO_ERROR` |
| `hermes_present_solid_frame` | error | atomic present sequence |

ICD library name (for future `nvidia_icd.json` / Mesa loader):
`libhermes_mesa.so`.

## Formal models

- Idris2: `formal/idris2/DrmKms.idr`
- Agda: `formal/agda/DrmKms.agda`
- Fortran: `formal/fortran/hermes_drm_kms.f90`

## Smoke

```sh
cargo run -p hermes-ctl --bin hermes-ctl -- drm-smoke offline
cargo run -p hermes-ctl --bin hermes-ctl -- drm-smoke online
cargo run -p hermes-ctl --bin hermes-ctl -- drm-smoke dual
cargo run -p hermes-ctl --bin hermes-ctl -- drm-smoke gem
cargo run -p hermes-ctl --bin hermes-ctl -- mesa-smoke offline
cargo run -p hermes-ctl --bin hermes-ctl -- mesa-smoke online
cargo run -p hermes-ctl --bin hermes-ctl -- mesa-smoke gem
cargo run -p hermes-ctl --bin hermes-ctl -- stack-smoke
cargo run -p hermes-ctl --bin hermes-ctl -- icd-json
sh scripts/stage-dropin.sh
```

## Honesty

Full Nouveau/NVK/Mesa parity (shader compilers, real scanout, GEM/TTM BOs,
atomic page-flip with vblank) is multi-year. Hermes ships the gated foundation
and named drop-in surfaces first; never claims Online without the manifold.
