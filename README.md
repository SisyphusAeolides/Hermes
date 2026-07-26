# Hermes GSP

**Hermes** is a clean-room, fail-closed **GPU System Processor (GSP)** host for
**NVIDIA Turing and newer** GPUs. It is written to be a **drop-in replacement
surface** for the open NVIDIA Linux stack entry points (`nvidia`,
`nvidia-modeset`, `nvidia-uvm`, `nvidia-drm`, `nvidia-peermem`,
`nvidia-settings`, `nvidia-smi` / NVML), with universal host portability through
a kernel HAL.

Languages:

| Language | Role |
|---|---|
| **Rust** | Executable runtime, HAL, Linux drop-in surfaces, tests |
| **Austral** | Linear ownership (`formal/austral/*.aui` + `*.aum` — stock **tokei ignores these**; run `sh scripts/loc.sh`) |
| **Idris2** | Total phase lattice and online certificates |
| **Agda** | Feature lattice and ring geometry (`--safe`) |

Upstream reverse-engineering reference (not vendored):
[NVIDIA/open-gpu-kernel-modules](https://github.com/NVIDIA/open-gpu-kernel-modules).

## Scope (honest)

- **In scope:** Turing → Blackwell device admission, measured GSP-RM firmware
  gates, SEC2/bootstrap manifests, fail-closed Online progression, Linux
  module/device/userspace **names** matching the NVIDIA stack, formal models.
- **Out of this tree’s git objects:** proprietary firmware blobs (stage them).
- **Not claimed complete:** full binary parity with every proprietary userspace
  library (complete CUDA / OptiX / every ICD path). Those surfaces grow on the
  same HAL without inventing Online.

Hermes **never** reports a GPU Online unless PCI match, firmware measurement,
IOMMU isolation, non-zero DMA domain, WPR lock, boot mailbox, ready queue, and
a well-formed feature set are all present.

## Workspace

```
crates/
  hermes-abi/       Wire contracts (GPU + Hermes personality ABI)
  hermes-core/      Device family, manifold, platform HAL, admission
  hermes-gsp/       GSP-RM manifests, bootstrap, activation plan
  hermes-linux/     Drop-in module / device / userspace names
  hermes-settings/  nvidia-settings + hermes-settings binaries
  hermes-nvml/      NVML-compatible shared library surface
  hermes-ctl/       hermes-ctl + nvidia-smi binaries
  hermes-nouveau/   Nouveau GSP path tables + superiority matrix
  hermes-cccl/      CCCL (Thrust/CUB) catalog + host subset
  hermes-cuda/      GSP-gated CUDA driver/runtime shell
  hermes-drm/       Atomic modeset foundation (GSP-gated)
  hermes-mesa/      Vulkan ICD + GL stubs + present path
formal/
  idris2/           HermesAuthority, NvkmGsp, Cccl, DrmKms
  agda/             HermesWire, NvkmGsp, Cccl, DrmKms
  austral/          HermesResources, HermesRings, HermesFailClosed, DrmKms, …
```

## Build and test

```sh
cargo test --workspace
cargo build --release -p hermes-settings -p hermes-ctl -p hermes-nvml
sh scripts/check-formal.sh
# Shared sequencer probe (fail then full Online on SimPlatform)
cargo run -p hermes-ctl -- bringup both
# Out-of-tree NVIDIA-named modules
make -C linux/kmod
```

### Linux kernel modules

See [`linux/kmod/README.md`](linux/kmod/README.md). Modules export classic names
`nvidia`, `nvidia-modeset`, `nvidia-uvm`, `nvidia-drm`, `nvidia-peermem` and call
the shared fail-closed bring-up (`hermes_run_bringup` / `hermes_gsp::run_bringup`).
Online is never advertised without firmware + IOMMU + WPR + mailbox + ready.

### Nouveau reverse engineering

```sh
python3 scripts/reverse-engineer-nouveau.py \
  --nouveau /path/to/linux/drivers/gpu/drm/nouveau \
  --out generated/nouveau-re
cargo test -p hermes-nouveau
cargo run -p hermes-ctl -- nouveau-compare
cargo run -p hermes-ctl -- nouveau-plan tu102 570.144
```

See [`docs/NOUVEAU_GSP.md`](docs/NOUVEAU_GSP.md). Hermes re-hosts Nouveau GSP
firmware tables under a **stricter Online** policy (measured digests + manifold).

### CCCL / CUDA compatibility

```sh
python3 scripts/reverse-engineer-cccl.py --cccl /path/to/cccl --out generated/cccl-re
cargo test -p hermes-cccl -p hermes-cuda
cargo run -p hermes-ctl -- cccl
cargo run -p hermes-ctl -- cuda-smoke offline
cargo run -p hermes-ctl -- cuda-smoke online
```

See [`docs/CCCL_CUDA.md`](docs/CCCL_CUDA.md). CCCL (Thrust/CUB/libcu++) is the
open CUDA **C++ library** layer; `hermes-cuda` is the driver/runtime shell and
**rejects all device calls while GSP is offline**.

### DRM/KMS and Mesa

```sh
cargo test -p hermes-drm -p hermes-mesa
cargo run -p hermes-ctl --bin hermes-ctl -- drm-smoke gem
cargo run -p hermes-ctl --bin hermes-ctl -- mesa-smoke gem
cargo run -p hermes-ctl --bin hermes-ctl -- cuda-smoke deep
cargo run -p hermes-ctl --bin hermes-ctl -- stack-smoke
make -C linux/kmod host-test
sh scripts/stage-dropin.sh
```

See [`docs/DRM_MESA.md`](docs/DRM_MESA.md). Atomic modeset, dumb GEM, page-flip,
Vulkan ICD, and CUDA streams only succeed when GSP is Online; Offline is fail-closed.

### Live silicon probe and full-image stage

```sh
cargo run -p hermes-ctl --bin hermes-ctl -- silicon-probe /lib/firmware
cargo run -p hermes-ctl --bin hermes-ctl -- mailbox-smoke
cargo run -p hermes-ctl --bin hermes-ctl -- silicon-bringup fail-mailbox
cargo run -p hermes-ctl --bin hermes-ctl -- silicon-bringup sim
cargo run -p hermes-ctl --bin hermes-ctl -- silicon-bringup live-fw
```

`run_bringup` now **stages the entire GSP-RM image** (chunked DMA + staged
SHA-256 must match admit), optionally drives Falcon mailbox and WPR/SEC2 paths,
and ANDs live observations into evidence (never invents Online). `live-fw` loads
real `/lib/firmware/nvidia/610.43.02/gsp_tu10x.bin` through the shared sequencer
on SimPlatform. Host `silicon-probe` still reports `online_claimed: false` when
IOMMU is missing or Nouveau owns the device.

## Drop-in install (Linux)

After building:

| NVIDIA entry | Hermes artifact |
|---|---|
| `nvidia` module name | `hermes-linux` personality / `hermes-gsp` (classic name `nvidia` when configured) |
| `nvidia-settings` | `target/release/nvidia-settings` |
| `nvidia-smi` | `target/release/nvidia-smi` |
| `libnvidia-ml.so.1` | `target/release/libnvidia_ml.so` (symlink to the classic soname) |

Device nodes expected by clients: `/dev/nvidiactl`, `/dev/nvidia0`,
`/dev/nvidia-uvm`, `/dev/nvidia-modeset`.

Firmware must be staged from a matching driver release or linux-firmware install.
Digests and ELF structure live in `hermes-gsp`; **blobs are never committed**.

See [`docs/GSP_FIRMWARE.md`](docs/GSP_FIRMWARE.md) for the reverse-engineered
layout of [NVIDIA/linux-firmware](https://github.com/NVIDIA/linux-firmware).

```sh
# Stage from host linux-firmware / OpenRM install
sh scripts/stage-linux-firmware-gsp.sh /lib/firmware target/hermes-gsp/staged
cargo run -p hermes-ctl -- firmware-scan /lib/firmware
```

## Turing+ coverage

| Family | PCI device-ID bands (coarse) | GSP line |
|---|---|---|
| Turing | `0x1E00–0x1FFF`, `0x2180–0x21FF` | tu10x |
| Ampere | `0x2000–0x20FF`, `0x2200–0x22FF`, `0x2400–0x25FF` | ga10x (GA100→tu10x) |
| Hopper | `0x2300–0x23FF` | ga10x |
| Ada | `0x2600–0x28FF` | ga10x |
| Blackwell | `0x2900–0x2FFF` | ga10x |

Pre-Turing (Maxwell / Pascal / Volta) is **rejected**.

## Formal gate

```sh
sh scripts/check-formal.sh          # idris2 + agda; austral if installed
sh scripts/check-formal.sh --strict # requires all three
```

See [formal/LANGUAGES.md](formal/LANGUAGES.md).

## License

MIT. Clean-room implementation. Do not paste proprietary NVIDIA sources into
this tree. Redistributable GSP firmware remains subject to NVIDIA’s firmware
license and is staged by the operator, not shipped in git.
