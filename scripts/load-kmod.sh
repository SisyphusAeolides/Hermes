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

# Unload reverse order (kernel names use underscores).
for m in nvidia_peermem nvidia_drm nvidia_uvm nvidia_modeset nvidia; do
  if [ -d "/sys/module/$m" ]; then
    rmmod "$m" 2>/dev/null || true
  fi
done

SIM_PARAM=
if [ "${HERMES_SIM_PROMOTE:-0}" = "1" ]; then
  SIM_PARAM="allow_sim_promote=1"
  echo "note: loading with allow_sim_promote=1 (integration Online only)"
fi

# shellcheck disable=SC2086
insmod ./nvidia.ko $SIM_PARAM
# Companions soft-depend on nvidia; load if built.
for m in nvidia-modeset nvidia-uvm nvidia-drm nvidia-peermem; do
  if [ -f "./$m.ko" ]; then
    insmod "./$m.ko" 2>/dev/null || echo "warn: insmod $m failed (may need deps)"
  fi
done

# Ensure world-readable for drop-in userspace (class devnode also sets 0666 on create).
for n in /dev/nvidiactl /dev/nvidia0 /dev/nvidia-drm /dev/nvidia-uvm \
         /dev/nvidia-uvm-tools /dev/nvidia-modeset /dev/nvidia-peermem; do
  if [ -e "$n" ]; then
    chmod 666 "$n" 2>/dev/null || true
  fi
done

echo "loaded modules:"
lsmod | grep -E '^nvidia' || true
echo "nodes:"
ls -l /dev/nvidiactl /dev/nvidia0 /dev/nvidia-drm /dev/nvidia-uvm \
  /dev/nvidia-uvm-tools /dev/nvidia-modeset /dev/nvidia-peermem 2>/dev/null || true

# Prove ioctl STATUS on nvidiactl + companions (fail-closed unless HERMES_SIM_PROMOTE=1).
HERMES_SIM_PROMOTE="${HERMES_SIM_PROMOTE:-0}" python3 - <<'PY'
import array, fcntl, os, sys
IOC_READ = 2
req = (IOC_READ << 30) | (0x48 << 8) | 0x10 | (16 << 16)
# _IO('H', 0x11) / 0x12
req_sim = (0x48 << 8) | 0x11
req_demote = (0x48 << 8) | 0x12
req_comp = (IOC_READ << 30) | (0x48 << 8) | 0x20 | (16 << 16)
path = "/dev/nvidiactl"
if not os.path.exists(path):
    print("FAIL: missing", path)
    sys.exit(1)
fd = os.open(path, os.O_RDWR)
a = array.array("I", [0, 0, 0, 0])
fcntl.ioctl(fd, req, a, True)
online, phase, version, mask = a
print(f"nvidiactl ioctl: online={online} phase={phase} ver={version} mask=0x{mask:x}")
if version < 2:
    print("FAIL: unexpected status version")
    sys.exit(1)
if online not in (0, 1):
    print("FAIL: invalid online flag")
    sys.exit(1)
if online == 1 and phase != 5:
    print("FAIL: online set with non-ONLINE phase")
    sys.exit(1)

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

# Companion STATUS (0x20) when nodes exist.
for node, bit in (
    ("/dev/nvidia-modeset", HERMES_MOD_MODESET),
    ("/dev/nvidia-uvm", HERMES_MOD_UVM),
    ("/dev/nvidia-uvm-tools", HERMES_MOD_UVM),
    ("/dev/nvidia-peermem", HERMES_MOD_PEERMEM),
):
    if not os.path.exists(node):
        print(f"skip companion {node}")
        continue
    cfd = os.open(node, os.O_RDWR)
    b = array.array("I", [0, 0, 0, 0])
    try:
        fcntl.ioctl(cfd, req_comp, b, True)
        print(f"{node} STATUS: online={b[0]} phase={b[1]} mask=0x{b[3]:x}")
        if (b[3] & bit) == 0:
            print(f"FAIL: {node} mask missing self bit")
            sys.exit(1)
    except OSError as e:
        print(f"FAIL: {node} STATUS: {e}")
        sys.exit(1)
    finally:
        os.close(cfd)
print("companion STATUS: PASS")

# Optional SIM_PROMOTE / DEMOTE when HERMES_SIM_PROMOTE=1
if os.environ.get("HERMES_SIM_PROMOTE", "0") == "1":
    try:
        fcntl.ioctl(fd, req_sim)
    except OSError as e:
        print(f"FAIL: SIM_PROMOTE: {e}")
        sys.exit(1)
    a = array.array("I", [0, 0, 0, 0])
    fcntl.ioctl(fd, req, a, True)
    print(f"after SIM_PROMOTE: online={a[0]} phase={a[1]}")
    if a[0] != 1 or a[1] != 5:
        print("FAIL: SIM_PROMOTE did not reach ONLINE")
        sys.exit(1)
    # Companion must see Online
    for node in ("/dev/nvidia-uvm", "/dev/nvidia-modeset"):
        if not os.path.exists(node):
            continue
        cfd = os.open(node, os.O_RDWR)
        b = array.array("I", [0, 0, 0, 0])
        fcntl.ioctl(cfd, req_comp, b, True)
        os.close(cfd)
        if b[0] != 1:
            print(f"FAIL: {node} still offline after SIM_PROMOTE")
            sys.exit(1)
    print("SIM_PROMOTE Online + companion visibility: PASS")
    fcntl.ioctl(fd, req_demote)
    a = array.array("I", [0, 0, 0, 0])
    fcntl.ioctl(fd, req, a, True)
    print(f"after DEMOTE: online={a[0]} phase={a[1]}")
    if a[0] != 0:
        print("FAIL: DEMOTE did not Offline")
        sys.exit(1)
    print("DEMOTE Offline: PASS")

os.close(fd)

drm = "/dev/nvidia-drm"
if os.path.exists(drm):
    req_d = (IOC_READ << 30) | (0x48 << 8) | 0x01 | (20 << 16)
    dfd = os.open(drm, os.O_RDWR)
    b = array.array("I", [0, 0, 0, 0, 0])
    try:
        fcntl.ioctl(dfd, req_d, b, True)
        print(f"nvidia-drm ioctl: online={b[0]} connectors={b[1]} crtcs={b[2]} active={b[3]} ver={b[4]}")
        print("nvidia-drm ioctl: PASS")
    except OSError as e:
        print(f"nvidia-drm ioctl: {e}")
    finally:
        os.close(dfd)
PY

echo "load-kmod: done"
