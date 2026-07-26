# Hermes out-of-tree kernel modules

NVIDIA-compatible module names for the open-gpu-kernel-modules set:

| Module | Role |
|---|---|
| `nvidia.ko` | Hermes GSP host — runs shared fail-closed bring-up |
| `nvidia-modeset.ko` | Modeset companion |
| `nvidia-uvm.ko` | UVM companion |
| `nvidia-drm.ko` | DRM companion |
| `nvidia-peermem.ko` | Peer-memory companion |

## Build

```sh
cd linux/kmod
make
```

Requires `/lib/modules/$(uname -r)/build` kernel headers.

## Behavior

- `nvidia` registers a PCI driver for NVIDIA display-class devices.
- Bring-up calls `hermes_run_bringup()` (same gate order as `hermes_gsp::run_bringup`).
- **Online is never claimed** unless firmware measured + IOMMU domain + WPR + mailbox + ready are all true.
- On a normal host without staged GSP firmware / IOMMU session, the module loads and stays **offline** by design.

## Load (operator)

Unload proprietary stack first if present, then:

```sh
sudo insmod ./nvidia.ko
sudo insmod ./nvidia-modeset.ko
# ...
dmesg | grep hermes
```

Do not force Online without measured firmware.
