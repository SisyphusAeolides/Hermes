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
- `nvidia-drm` registers misc device **`/dev/nvidia-drm`** with GSP-gated ioctls
  (`STATUS`, `DUMB_CREATE`, `ATOMIC`, `DISABLE_CRTC`) — see `include/hermes_drm_uapi.h`.
- `nvidia-modeset` logs Online state and refuses authority while Offline.
- On a normal host without staged GSP firmware / IOMMU session, modules load and stay **offline** by design.

## Load (operator)

Unload proprietary stack first if present, then:

```sh
sudo insmod ./nvidia.ko
sudo insmod ./nvidia-modeset.ko
sudo insmod ./nvidia-drm.ko
# ...
dmesg | grep hermes
ls -l /dev/nvidia-drm
```

Do not force Online without measured firmware.
