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
- `AtomicCommit::commit` — fail-closed atomic apply
- `AtomicCommit::disable_crtc` — blank a pipe

This is **not** a full in-kernel DRM driver yet. It is the clean-room modeset
foundation that will back `nvidia-drm` / a future DRM character device once
GSP Online is real on silicon.

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
- Austral: `formal/austral/DrmKms.aui` / `.aum`

## Smoke

```sh
cargo run -p hermes-ctl -- drm-smoke offline
cargo run -p hermes-ctl -- drm-smoke online
cargo run -p hermes-ctl -- drm-smoke dual
cargo run -p hermes-ctl -- mesa-smoke offline
cargo run -p hermes-ctl -- mesa-smoke online
```

## Honesty

Full Nouveau/NVK/Mesa parity (shader compilers, real scanout, GEM/TTM BOs,
atomic page-flip with vblank) is multi-year. Hermes ships the gated foundation
and named drop-in surfaces first; never claims Online without the manifold.
