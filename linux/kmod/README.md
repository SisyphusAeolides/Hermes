# Hermes out-of-tree kernel modules

NVIDIA-compatible module names for the open-gpu-kernel-modules set:

| Module | Role |
|---|---|
| `nvidia.ko` | Hermes GSP host — runs shared fail-closed bring-up |
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
- Bring-up calls `hermes_run_bringup()` (same gate order as `hermes_gsp::run_bringup`).
- **Online is never claimed** unless firmware measured + IOMMU domain + WPR + mailbox + ready are all true.
- `hermes_gsp_is_online()` / `hermes_gsp_phase()` exported for companion modules.
- `nvidia` creates **`/dev/nvidiactl`** and **`/dev/nvidia0`** (status ioctl/read; fail-closed Online).
  STATUS `module_mask` ORs live companions (`nvidia_modeset` / `uvm` / `drm` / `peermem`).
- `nvidia-drm` misc **`/dev/nvidia-drm`**: `STATUS`, `DUMB_CREATE`, `ATOMIC`, `DISABLE_CRTC`,
  `GET_EDID`, `GET_PROP` — see `include/hermes_drm_uapi.h`.
- `nvidia-modeset` → **`/dev/nvidia-modeset`** (STATUS always; other ioctls need Online).
- `nvidia-uvm` → **`/dev/nvidia-uvm`** + **`/dev/nvidia-uvm-tools`** (STATUS + Online gate).
- `nvidia-peermem` → **`/dev/nvidia-peermem`** (STATUS + `hermes_peermem_register_ok`).
- On a normal host without staged GSP firmware / IOMMU session, modules load and stay **offline**.

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

Do not force Online without measured firmware (except explicit sim promote above).
