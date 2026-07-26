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
