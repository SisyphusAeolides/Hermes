# Hermes out-of-tree kernel modules

NVIDIA-compatible module names for the open-gpu-kernel-modules set:

| Module | Role |
|---|---|
| `nvidia.ko` | Hermes GSP host — runs shared evidence-driven bring-up |
| `nvidia-modeset.ko` | Modeset companion |
| `nvidia-uvm.ko` | UVM companion |
| `nvidia-drm.ko` | DRM char device + ioctl surface (`/dev/nvidia-drm`) |
| `nvidia-peermem.ko` | Peer-memory companion |

## Build

```sh
cd linux/kmod
make
make host-test   # userspace unit tests (bringup + drm logic)
```

Requires `/lib/modules/$(uname -r)/build` kernel headers for `make`.

## Behavior

- `nvidia` registers a PCI driver for NVIDIA display-class devices.
- During PCI probe it loads the staged `nvidia/<version>/gsp_{tu10x,ga10x}.bin`,
  hashes the complete image with the kernel SHA-256 API, and submits that
  measurement to the same embedded pin list used by the userspace tool.
- Bring-up calls `hermes_run_bringup()` (same gate order as `hermes_gsp::run_bringup`).
- **Online is published** only after firmware measurement + IOMMU domain + WPR + mailbox + ready are all observed.
- `hermes_gsp_is_online()` / `hermes_gsp_phase()` exported for companion modules.
- `nvidia` creates **`/dev/nvidiactl`** and **`/dev/nvidia0`** (status ioctl/read; evidence-gated Online).
  STATUS `module_mask` ORs live companions (`nvidia_modeset` / `uvm` / `drm` / `peermem`).
- `nvidia-drm` misc **`/dev/nvidia-drm`**: `STATUS`, `DUMB_CREATE`, `ATOMIC`, `DISABLE_CRTC`,
  `GET_EDID`, `GET_PROP` — see `include/hermes_drm_uapi.h`.
- `nvidia-modeset` → **`/dev/nvidia-modeset`** (STATUS always; other ioctls need Online).
- `nvidia-uvm` → **`/dev/nvidia-uvm`** + **`/dev/nvidia-uvm-tools`** (STATUS + Online gate).
- `nvidia-peermem` → **`/dev/nvidia-peermem`** (STATUS + `hermes_peermem_register_ok`).
- The modules can load while a host session is being established; until staged GSP firmware and the remaining hardware evidence are observed, status is truthfully **offline**.

The firmware directory defaults to `610.57.04` and can be selected at module
load time when an older pinned release is staged:

```sh
sudo insmod ./nvidia.ko firmware_version=610.43.03
```

The version selects a path only; admission still requires the exact embedded
length and SHA-256 pin. An unpinned or altered image is rejected and the
session remains offline.

### Integration Online (optional)

```sh
# Complete-evidence Online for host tests — NOT silicon measurement:
HERMES_SIM_PROMOTE=1 sudo -E sh scripts/load-kmod.sh
# or:
sudo insmod ./nvidia.ko allow_sim_promote=1
# then HERMES_CTL_IOCTL_SIM_PROMOTE / DEMOTE on /dev/nvidiactl
```

Default `allow_sim_promote=0` denies SIM_PROMOTE (`-EPERM`).

## Load (operator)

```sh
sudo sh scripts/load-kmod.sh
# or manually:
sudo insmod ./nvidia.ko
sudo insmod ./nvidia-modeset.ko
sudo insmod ./nvidia-uvm.ko
sudo insmod ./nvidia-drm.ko
sudo insmod ./nvidia-peermem.ko
dmesg | grep hermes
ls -l /dev/nvidia*
```

Do not assert Online without measured firmware (except the explicit simulation promote above).
