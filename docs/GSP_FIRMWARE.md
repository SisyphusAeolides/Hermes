# GSP firmware reverse engineering (linux-firmware)

Source of layout truth:

- [NVIDIA/linux-firmware](https://github.com/NVIDIA/linux-firmware) (`nvidia-staging`)
- Upstream kernel tree: [kernel-firmware/linux-firmware](https://gitlab.com/kernel-firmware/linux-firmware)
- Local install: `/lib/firmware/nvidia/`
- Driver packages: `NVIDIA-Linux-x86_64-*.run` (origin of versioned GSP-RM images)

**Hermes never commits firmware blobs.** Only manifests (length + SHA-256), path layout, and ELF structural checks live in git.

## Two packaging layouts

### A. Driver-versioned OpenRM blobs (open kernel modules / hermes pins)

```text
/lib/firmware/nvidia/<MAJOR.MINOR.PATCH>/
  gsp_tu10x.bin      # GSP-RM ELF for Turing + GA100
  gsp_ga10x.bin      # GSP-RM ELF for GA10x and later
  ucodes_tu10x.bin   # "NVUCODES" container (boot helper blobs)
  ucodes_ga10x.bin
```

Observed on this host for **610.43.02**:

| File | Bytes | SHA-256 (full) | Format |
|---|---:|---|---|
| `gsp_tu10x.bin` | 29 352 832 | `c8fc1a92…dde17f3` | ELF64 RISC-V REL |
| `gsp_ga10x.bin` | 84 277 400 | `00da3fd9…6525ce` | ELF64 RISC-V REL |
| `ucodes_tu10x.bin` | 12 032 | `dcbdf512…5acb38` | magic `NVUCODES` |
| `ucodes_ga10x.bin` | 31 744 | `3a36ceb2…7a632c` | magic `NVUCODES` |

`.fwversion` inside the ELF strings as ASCII `610.43.02\0`.

Hermes also keeps a pin for **610.43.03** (from open-gpu-kernel-modules era staging): same lengths, different digests.

### B. Chip-tree linux-firmware (Nouveau / multi-version)

```text
/lib/firmware/nvidia/<chip>/
  gsp/
    gsp-<driver>.bin.xz           # same GSP-RM image family as gsp_*x.bin
    bootloader-<driver>.bin.xz
    booter_load-<driver>.bin.xz
    booter_unload-<driver>.bin.xz
    gen_bootloader-<driver>.bin.xz   # Turing/GA100 path
    fmc-<driver>.bin.xz              # Hopper/Blackwell path
    scrubber-<driver>.bin.xz         # some Ada+
  sec2/  acr/  gr/  nvdec/           # Falcon helpers (pre/post GSP)
```

**WHENCE aliasing (critical):**

| Logical GSP-RM line | Origin name | Chip paths that share the same image |
|---|---|---|
| tu10x | `gsp_tu10x.bin` from `NVIDIA-Linux-…-570.144.run` | `tu102`, `tu116`→tu102, `ga100`→tu102 |
| ga10x | `gsp_ga10x.bin` from same run | `ga102`, `ad102`→ga102, `gh100`→ga102, `gb100`/`gb202`→ga102 |

Chip directory symlinks (from WHENCE):

```text
tu104/gsp → tu102/gsp
tu106/gsp → tu102/gsp
tu117/gsp → tu116/gsp
ga103..ga107/gsp → ga102/gsp
ad103..ad107 → ad102
gb102 → gb100
gb203..gb207 → gb202
```

Driver versions observed in chip trees on this host: **535.113.01** and **570.144**.

## GSP-RM ELF structure (gsp_*.bin)

`file(1)`: `ELF 64-bit LSB relocatable, UCB RISC-V`.

Measured section layout (tu10x 610.43.02):

| Section | Role |
|---|---|
| `.fwimage` | Payload image (almost entire file) |
| `.note.gnu.build-id` | Build-id note |
| `.fwversion` | ASCII version string, e.g. `610.43.02` |
| `.fwsignature_tu10x` | Per-family signature blob (4 KiB) |
| `.fwsignature_tu11x` | TU11x signature |
| `.fwsignature_ga100` | GA100 signature (present on tu10x line) |
| ga10x image has its own signature section set | |

Hermes structural admission requires: ELF64 + RISC-V machine + `.fwimage` present + non-empty `.fwversion`. Signature sections are recorded but not cryptographically verified yet (verification needs NVIDIA’s public key path).

## Boot sequence roles (Turing)

Matches open-gpu-kernel-modules / Hermes bootstrap roles:

1. **SEC2 / generic bootloader** (`sec2/*`, `gen_bootloader-*`)
2. **Booter Load** (`booter_load-*`) — copies WPR metadata + images into FB, locks WPR2
3. **GSP bootloader** (`bootloader-*`) — starts RISC-V GSP
4. **GSP-RM** (`gsp-*.bin` / `gsp_tu10x.bin`) — main RM firmware
5. **Booter Unload** — teardown/recovery

Blackwell/Hopper add **FMC** (`fmc-*.bin`) instead of the older SEC2/Booter stack for some stages.

## Hermes mapping

| Device class | `FirmwareFamily` | linux-firmware GSP line | OpenRM file |
|---|---|---|---|
| Turing (`0x1E00–1FFF`, `0x2180–21FF`) | `Tu10x` | tu102/tu116 tree | `gsp_tu10x.bin` |
| GA100 (`0x2000–20FF`) | `Tu10x` | ga100 → tu102 gsp | `gsp_tu10x.bin` |
| Ampere non-GA100, Hopper, Ada, Blackwell | `Ga10x` | ga102/ad/gb/gh | `gsp_ga10x.bin` |

## Staging (operator)

```sh
# Prefer host linux-firmware / driver install
sh scripts/stage-linux-firmware-gsp.sh /lib/firmware/nvidia target/hermes-gsp/staged

# Or pin a single OpenRM blob
sh scripts/stage-gsp-rm.sh /lib/firmware/nvidia/610.43.02/gsp_tu10x.bin target/hermes-gsp/tu10x
```

Blobs stay under `target/` (gitignored). Admission uses allow-listed digests only.
