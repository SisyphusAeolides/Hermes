# Drop-in compatibility

Hermes targets the same operator-facing names as
[open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules).

Full catalog (source of truth in `hermes_linux::DROP_IN_CATALOG`):

```sh
cargo run -p hermes-ctl --bin hermes-ctl -- dropin-catalog
cargo run -p hermes-ctl --bin hermes-ctl -- dropin-parity
cargo run -p hermes-ctl --bin hermes-ctl -- dropin-complete
```

`dropin-parity` reports named-surface coverage against `DROP_IN_PARITY_TARGET`
(currently **30** catalog entries = 100% of the advertised open-stack name set).

| Component | Role |
|---|---|
| `nvidia` | Core RM / GSP host |
| `nvidia-modeset` | Modeset |
| `nvidia-uvm` | Unified virtual memory |
| `nvidia-drm` | DRM/KMS |
| `nvidia-peermem` | Peer memory |
| `nvidia-settings` | GUI/CLI control |
| `nvidia-smi` / NVML | Management queries |
| `nvidia-modprobe` | Module/device helper (fail-closed status + load) |
| `nvidia-persistenced` | Persistence mode helper via NVML |
| `libcuda.so.1` / `libcudart.so.12` | CUDA driver + runtime sonames |
| `libGLX_nvidia` / `libEGL_nvidia` | Mesa GL/EGL sonames |
| `/dev/nvidia*` | Character devices (incl. uvm-tools, caps, drm) |

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
cargo run -p hermes-ctl --bin nvidia-smi -- --hermes-sim-online \
  --query-gpu=name,brand,fan.speed,temperature.gpu,power.draw,memory.total
cargo run -p hermes-ctl --bin hermes-ctl -- smi-smoke host
cargo run -p hermes-ctl --bin hermes-ctl -- smi-smoke online
```

Host discover binds Offline NVML slots from PCI. `--hermes-sim-online` promotes
the first GPU with a complete-evidence Online manifold so power/temp/fan/util
query paths run through the real NVML ABI. Summary table shows fan, power cap,
brand, and SM version when Online. CSV:

```sh
nvidia-smi --hermes-sim-online --query-gpu=name,brand,fan.speed \
  --format=csv,noheader
```

### kmod / chardev probe

```sh
cargo run -p hermes-ctl --bin hermes-ctl -- chardev-smoke
cargo run -p hermes-ctl --bin hermes-ctl -- kmod-status
# Root: load modules + prove real ioctl
sudo sh scripts/load-kmod.sh
cargo run -p hermes-ctl --bin hermes-ctl -- kmod-load-smoke
```

Reports `/sys/module/nvidia*` and `/dev/nvidia*` honestly. When the Hermes
`nvidia.ko` is loaded, ioctl/read of `/dev/nvidiactl` returns `HermesCtlStatus`
(never invents Online without kernel phase Online). `load-kmod.sh` insmods
`linux/kmod/*.ko` and runs a Python ioctl probe on the live chardev.

```sh
# Integration Online (complete-evidence sim — not silicon measure):
HERMES_SIM_PROMOTE=1 sudo -E sh scripts/load-kmod.sh
```

Companions (`nvidia-modeset` / `uvm` / `uvm-tools` / `peermem`) expose STATUS
ioctls. `module_mask` ORs all live companions. `allow_sim_promote=1` enables
`SIM_PROMOTE` / `DEMOTE` on `/dev/nvidiactl` for Turing+ host GPUs.

```sh
# Rust end-to-end: SIM_PROMOTE → companion Online → live EDID → DEMOTE
cargo run -p hermes-ctl --bin hermes-ctl -- kmod-online-smoke
```

### DRM EDID via kmod

With `nvidia-drm.ko` loaded, `HERMES_DRM_IOCTL_GET_EDID` / `GET_PROP` return a
checksummed synthetic base EDID and property shell (Online-gated in logic;
bare load stays Offline → `-ENODEV`).

### nvidia-settings

```sh
cargo run -p hermes-settings --bin nvidia-settings -- --status
cargo run -p hermes-settings --bin nvidia-settings -- --query gpus
HERMES_SETTINGS_SIM_ONLINE=1 cargo run -p hermes-settings --bin nvidia-settings -- --query gpus
```

Settings discovers the same host GPUs via NVML and prints phase/memory from the
session (not a fixed empty list).

### Unified session promote

```sh
cargo run -p hermes-ctl --bin hermes-ctl -- session-promote
cargo run -p hermes-ctl --bin hermes-ctl -- stack-smoke
cargo run -p hermes-ctl --bin nvidia-smi -- --hermes-sim-online
```

`session-promote` discovers host GPUs, runs complete-evidence Online, registers a
compute process, binds CUDA + Mesa, and leaves process rows visible to smi.

### nvidia-modprobe

```sh
cargo run -p hermes-ctl --bin nvidia-modprobe -- --status
cargo run -p hermes-ctl --bin nvidia-modprobe -- -u --verbose
```

Reports module/device presence honestly. Load attempts use `modprobe`/`insmod`
and never claim GSP Online. Device node creation refuses forged majors when the
kernel has not registered an `nvidia` char device.

### nvidia-persistenced

```sh
cargo run -p hermes-ctl --bin nvidia-persistenced -- --verbose
cargo run -p hermes-ctl --bin nvidia-persistenced -- --no-persistence-mode
```

Sets NVML persistence mode on discovered host GPUs. Does not invent Online.

### Query clocks / architecture

```sh
nvidia-smi --hermes-sim-online \
  --query-gpu=name,architecture,clocks.current.graphics,clocks.current.memory \
  --format=csv
```

### Stage prefix

```sh
sh scripts/stage-dropin.sh
# → staging/dropin/{bin,lib,share/...} with soname links + ICD/EGL vendor JSON
```

See `docs/DRM_MESA.md` and `docs/CCCL_CUDA.md`.
