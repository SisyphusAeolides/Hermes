# Drop-in compatibility

Hermes targets the same operator-facing names as
[open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules):

| Component | Role |
|---|---|
| `nvidia` | Core RM / GSP host |
| `nvidia-modeset` | Modeset |
| `nvidia-uvm` | Unified virtual memory |
| `nvidia-drm` | DRM/KMS |
| `nvidia-peermem` | Peer memory |
| `nvidia-settings` | GUI/CLI control |
| `nvidia-smi` / NVML | Management queries |
| `/dev/nvidia*` | Character devices |

Hermes provides personalities and userspace binaries under those names. Binding
a name does not mark the GPU Online; the manifold gates still apply.

## Display / compute userspace (in progress)

| Surface | Hermes crate | Notes |
|---|---|---|
| DRM/KMS atomic | `hermes-drm` | Modeset foundation; backs future `nvidia-drm` |
| Mesa / Vulkan ICD | `hermes-mesa` | `libhermes_mesa.so` ICD name; GSP-gated |
| CUDA driver API | `hermes-cuda` | `cu*` / `cuda*` shell; GSP-gated |
| CCCL catalog | `hermes-cccl` | Thrust/CUB inventory + host subset |

See `docs/DRM_MESA.md` and `docs/CCCL_CUDA.md`.
