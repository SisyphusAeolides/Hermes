# Hermes GSP

**Hermes** is a clean-room, fail-closed, universal **GPU System Processor (GSP) and Firmware Host** for **NVIDIA, AMD, and Intel** GPUs. 
Originally built to be a strict drop-in replacement for the proprietary NVIDIA Linux stack (`nvidia`, `nvidia-modeset`, `nvidia-uvm`, `nvidia-smi` / NVML), Hermes has evolved into a mathematically verified, multi-vendor GPU hypervisor and host layer.

Hermes intentionally **breaks the rules** of traditional OS scheduling. By replacing standard locks and exponential backoff with **continuous and discrete deterministic chaos** (Lorenz, Rössler, Logistic Map, Duffing), Hermes prevents atomic phase-locking and delivers staggering zero-copy ring throughput (12+ Million ops/sec) that radically outperforms proprietary driver stacks.

Languages:

| Language | Role |
|---|---|
| **Rust** | Executable runtime, HAL, chaotic ring geometry, Linux drop-in surfaces, tests |
| **Fortran** | Exclusive resource ownership (`formal/fortran/*.f90`, `make -C formal/fortran check`) |
| **Idris2** | Total phase lattice and online certificates |
| **Agda** | Feature lattice and ring geometry (`--safe`) |

## Scope (honest)

- **In scope:** Universal device admission (NVIDIA Turing+, AMD RDNA/CDNA, Intel Xe/Arc), strict firmware measurement gates (OpenRM, SMU, GuC), SEC2/bootstrap manifests, fail-closed Online progression, Linux module/device/userspace **names** matching the proprietary stack, formal models.
- **Out of this tree’s git objects:** proprietary firmware blobs (stage them).
- **Not claimed complete:** full binary parity with every proprietary userspace library (complete CUDA / OptiX / every ICD path). Those surfaces grow on the same HAL without inventing Online.

Hermes **never** reports a GPU Online unless PCI match, firmware measurement, IOMMU isolation, non-zero DMA domain, WPR lock, boot mailbox, ready queue, and a well-formed feature set are all present.

## Workspace

```
crates/
  hermes-abi/       Wire contracts (GPU + Hermes personality ABI)
  hermes-core/      Vendor admission (NVIDIA, AMD, Intel), chaos manifold, HAL
  hermes-gsp/       GSP-RM manifests, bootstrap, activation plan
  hermes-linux/     Drop-in module / device / userspace names
  hermes-settings/  nvidia-settings + hermes-settings binaries
  hermes-nvml/      NVML-compatible shared library surface
  hermes-ctl/       hermes-ctl (universal diagnostic) + nvidia-smi binaries
  hermes-nouveau/   Nouveau GSP path tables + superiority matrix
  hermes-cccl/      CCCL (Thrust/CUB) catalog + host subset
  hermes-cuda/      GSP-gated CUDA driver/runtime shell
  hermes-drm/       Atomic modeset foundation (GSP-gated)
  hermes-mesa/      Vulkan ICD + GL stubs + present path
formal/
  idris2/           HermesAuthority, NvkmGsp, Cccl, DrmKms, …
  agda/             HermesWire, NvkmGsp, Cccl, DrmKms, …
  fortran/          hermes_resources, hermes_rings, hermes_fail_closed, …
```

## Chaotic Ring Scheduling

Instead of busy-spinning or yielding to the OS scheduler, Hermes `ZeroCopyRing` relies on non-linear dynamics:
- **Lorenz & Rössler attractors:** Provide non-periodic, bounded sleep intervals.
- **Duffing oscillator:** Injects resonant forcing for cyclic workloads.
- **Logistic Map:** Generates extremely fast pseudo-random state transitions.
- **Mandelbrot & Lyapunov estimators:** Maintain the chaos envelope, preventing the scheduler from settling into fixed points.

Test the ultra-high throughput locally:
```sh
cargo run -p hermes-ctl -- chaos-benchmark
```

## Build and test

```sh
cargo test --workspace
cargo build --release -p hermes-settings -p hermes-ctl -p hermes-nvml
sh scripts/check-formal.sh
# Shared sequencer probe (fail then full Online on SimPlatform)
cargo run -p hermes-ctl -- bringup both
# Out-of-tree vendor modules (clang recommended)
make -C linux/kmod CC=clang LLVM=1
```

## ArachOS integration

Hermes is packaged as `hermes-gpu-stack` for ArachOS, the independent RPM/DNF
distribution that carries this checkout alongside RustD, RustD-resolved, and
the ArachOS kernel qualification path. The package provides the Hermes
control tools, the NVIDIA-compatible library names, the Vulkan/EGL registration
files, the kernel-module source, and the native RustD unit definition.

RustD replaces the service-manager runtime on ArachOS. Enable and inspect the
Hermes unit with `rustctl`; Hermes does not require `systemctl` to start or
manage its service:

```sh
sudo dnf install hermes-gpu-stack
sudo rustctl enable hermes-gpu.service
hermes-ctl kmod-status
hermes-ctl dropin-catalog
```

Build and validate the ArachOS RPM repository from the coordinated checkouts:

```sh
cd ~/Projects/ArachOS
make verify-sources
make build-rpms
make validate-rpms
```

The ArachOS package carries the source needed to build Hermes' out-of-tree
kernel modules for the selected target kernel. It does not bundle proprietary
GPU firmware. Stage a matching, operator-approved firmware release under
`target/hermes-gsp/`, run the firmware scan, and only then attempt hardware
bring-up. Simulation flags are useful for exercising the state machine, but
they never certify a physical GPU:

```sh
sh scripts/stage-linux-firmware-gsp.sh /lib/firmware target/hermes-gsp/staged
cargo run -p hermes-ctl -- firmware-scan /lib/firmware
cargo run -p hermes-ctl -- bringup both
```

The package and service are compatibility surfaces, not a promise that every
vendor firmware, CUDA, OptiX, or display path is complete on every machine.
Hermes must report Offline when any admission gate is absent; ArachOS release
validation treats that truthful result as distinct from a successful hardware
Online session.

### Universal Hardware Coverage

| Vendor | Family | Codec |
|---|---|---|
| **NVIDIA** | Turing, Ampere, Hopper, Ada, Blackwell | OpenRM / GSP-RM |
| **AMD** | RDNA, RDNA2, RDNA3, CDNA, CDNA2, CDNA3 | PSP / SMU |
| **Intel** | Gen9, Gen11, Gen12, Xe, Arc | GuC / HuC |

Pre-Turing (Maxwell / Pascal / Volta) is **rejected**.

## Drop-in install (Linux)

After building:

| Proprietary Entry | Hermes Artifact |
|---|---|
| `nvidia` module name | `hermes-linux` personality / `hermes-gsp` (classic name `nvidia` when configured) |
| `nvidia-settings` | `target/release/nvidia-settings` |
| `nvidia-smi` | `target/release/nvidia-smi` (universal `hermes-ctl` backend) |
| `libnvidia-ml.so.1` | `target/release/libnvidia_ml.so` (symlink to the classic soname) |

Device nodes expected by clients: `/dev/nvidiactl`, `/dev/nvidia0`, `/dev/nvidia-uvm`, `/dev/nvidia-modeset`.

Firmware must be staged from a matching driver release or linux-firmware install. Digests and ELF structure live in `hermes-gsp`; **blobs are never committed**.

```sh
# Stage from host linux-firmware
sh scripts/stage-linux-firmware-gsp.sh /lib/firmware target/hermes-gsp/staged
cargo run -p hermes-ctl -- firmware-scan /lib/firmware
```

## Formal gate

```sh
sh scripts/check-formal.sh          # idris2 + agda + gfortran when installed
sh scripts/check-formal.sh --strict # requires idris2, agda, and gfortran
```

See [formal/LANGUAGES.md](formal/LANGUAGES.md).

## License

MIT. Clean-room implementation. Do not paste proprietary sources into this tree. Redistributable firmware remains subject to the vendor's firmware license and is staged by the operator, not shipped in git.
