#!/usr/bin/env sh
# Load Hermes out-of-tree kmods (classic nvidia* names) and prove chardev ioctl.
# Requires root (sudo). Never invents Online — incomplete host evidence stays Offline.
set -eu

ROOT="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
KMOD="$ROOT/linux/kmod"
need_root() {
  if [ "$(id -u)" -ne 0 ]; then
    if command -v sudo >/dev/null 2>&1; then
      exec sudo -E "$0" "$@"
    fi
    echo "error: must run as root to insmod" >&2
    exit 1
  fi
}

need_root "$@"

cd "$KMOD"
if [ ! -f nvidia.ko ]; then
  echo "building modules..."
  make -C "$KMOD" modules
fi

# Unload reverse order if present (ignore errors).
for m in nvidia-peermem nvidia-drm nvidia-uvm nvidia-modeset nvidia; do
  if [ -d "/sys/module/$m" ]; then
    rmmod "$m" 2>/dev/null || true
  fi
done

insmod ./nvidia.ko
# Companions soft-depend on nvidia; load if built.
for m in nvidia-modeset nvidia-uvm nvidia-drm nvidia-peermem; do
  if [ -f "./$m.ko" ]; then
    insmod "./$m.ko" 2>/dev/null || echo "warn: insmod $m failed (may need deps)"
  fi
done

# Ensure world-readable for drop-in userspace (class devnode also sets 0666 on create).
for n in /dev/nvidiactl /dev/nvidia0 /dev/nvidia-drm; do
  if [ -e "$n" ]; then
    chmod 666 "$n" 2>/dev/null || true
  fi
done

echo "loaded modules:"
lsmod | grep -E '^nvidia' || true
echo "nodes:"
ls -l /dev/nvidiactl /dev/nvidia0 /dev/nvidia-drm 2>/dev/null || true

# Prove ioctl STATUS on nvidiactl (fail-closed Online expected without full evidence).
python3 - <<'PY'
import array, fcntl, os, sys
IOC_READ = 2
req = (IOC_READ << 30) | (0x48 << 8) | 0x10 | (16 << 16)
path = "/dev/nvidiactl"
if not os.path.exists(path):
    print("FAIL: missing", path)
    sys.exit(1)
fd = os.open(path, os.O_RDWR)
a = array.array("I", [0, 0, 0, 0])
fcntl.ioctl(fd, req, a, True)
os.close(fd)
online, phase, version, mask = a
print(f"nvidiactl ioctl: online={online} phase={phase} ver={version} mask=0x{mask:x}")
if version < 2:
    print("FAIL: unexpected status version")
    sys.exit(1)
if online not in (0, 1):
    print("FAIL: invalid online flag")
    sys.exit(1)
# Fail-closed: incomplete silicon must not claim Online after bare insmod.
if online == 1 and phase != 5:
    print("FAIL: online set with non-ONLINE phase")
    sys.exit(1)
# Companion soft-deps ORed into mask (kernel names use underscores).
import os
def live(name):
    return os.path.isdir(f"/sys/module/{name}")
HERMES_MOD_NVIDIA = 1 << 0
HERMES_MOD_MODESET = 1 << 1
HERMES_MOD_UVM = 1 << 2
HERMES_MOD_DRM = 1 << 3
HERMES_MOD_PEERMEM = 1 << 4
expect = HERMES_MOD_NVIDIA
if live("nvidia_modeset"):
    expect |= HERMES_MOD_MODESET
if live("nvidia_uvm"):
    expect |= HERMES_MOD_UVM
if live("nvidia_drm"):
    expect |= HERMES_MOD_DRM
if live("nvidia_peermem"):
    expect |= HERMES_MOD_PEERMEM
if mask != expect:
    print(f"FAIL: module_mask 0x{mask:x} != expected 0x{expect:x}")
    sys.exit(1)
print(f"companion mask OR ok (0x{mask:x})")
print("nvidiactl ioctl: PASS (real chardev path)")
# Optional DRM status if node present.
drm = "/dev/nvidia-drm"
if os.path.exists(drm):
    req_d = (IOC_READ << 30) | (0x48 << 8) | 0x01 | (20 << 16)
    fd = os.open(drm, os.O_RDWR)
    b = array.array("I", [0, 0, 0, 0, 0])
    try:
        fcntl.ioctl(fd, req_d, b, True)
        print(f"nvidia-drm ioctl: online={b[0]} connectors={b[1]} crtcs={b[2]} active={b[3]} ver={b[4]}")
        print("nvidia-drm ioctl: PASS")
    except OSError as e:
        print(f"nvidia-drm ioctl: {e} (still OK if offline gates return ENODEV on other cmds)")
    finally:
        os.close(fd)
PY

echo "load-kmod: done"
