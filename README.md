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
| **Austral** | Linear ownership of domains, firmware seals, rings, sessions |
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
formal/
  idris2/           HermesAuthority.idr
  agda/             HermesWire.agda
  austral/          HermesResources, HermesRings, HermesFailClosed
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
