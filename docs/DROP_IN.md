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
| `nvidia-smi` / NVML | `hermes-ctl` + `hermes-nvml` | Discovers host Turing+ via sysfs; Online telemetry after session promote |

### nvidia-smi

```sh
cargo run -p hermes-ctl --bin nvidia-smi -- -L
cargo run -p hermes-ctl --bin nvidia-smi -- --hermes-sim-online
cargo run -p hermes-ctl --bin hermes-ctl -- smi-smoke host
cargo run -p hermes-ctl --bin hermes-ctl -- smi-smoke online
```

Host discover binds Offline NVML slots from PCI. `--hermes-sim-online` promotes
the first GPU with a complete-evidence Online manifold so power/temp/util query
paths run through the real NVML ABI.

See `docs/DRM_MESA.md` and `docs/CCCL_CUDA.md`.
