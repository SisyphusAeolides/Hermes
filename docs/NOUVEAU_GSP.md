# Nouveau GSP reverse engineering → Hermes

## Source tree

```text
drivers/gpu/drm/nouveau/
  nvkm/subdev/gsp/          # MIT GSP core (Red Hat / Nouveau)
    tu102.c tu116.c ga100.c ga102.c ad102.c gh100.c gb100.c gb202.c
    rm/r535/  rm/r570/      # RM API generations
  include/nvkm/subdev/gsp.h
```

Regenerate tables:

```sh
python3 scripts/reverse-engineer-nouveau.py \
  --nouveau /path/to/linux/drivers/gpu/drm/nouveau \
  --out generated/nouveau-re
# copy firmware_manifest.rs into crates/hermes-nouveau/src/
```

## Firmware binding (Nouveau)

| Style | Roles | Chips |
|---|---|---|
| **BOOTER** | `booter_load`, `booter_unload`, `bootloader`, `gsp` | TU10x, GA100, GA10x, AD10x |
| **FMC** | `fmc`, `bootloader`, `gsp` | GH100, GB10x, GB20x |

Paths: `nvidia/<chip>/gsp/<role>-<version>.bin`  
Versions in current mainline: **535.113.01**, **570.144**.

## Hermes superiority

See `generated/nouveau-re/superiority.md` and `hermes_nouveau::superiority`.

Hermes reuses Nouveau’s path/version knowledge but seals Online only after the
shared `run_bringup` manifold (firmware hash + ELF + IOMMU + WPR + mailbox + ready).

## Crate

`crates/hermes-nouveau` — NVKM layering, GSP load plans, RPC policy, capability matrix.

## Display path (next layer)

Nouveau couples GSP to DRM/KMS and Mesa (NVK / Nouveau GL). Hermes mirrors that
stack with:

- `crates/hermes-drm` — GSP-gated atomic modeset foundation
- `crates/hermes-mesa` — Vulkan ICD + GL stubs + present via atomic commit

See `docs/DRM_MESA.md`. Capability matrix marks `DrmKmsDisplay` and
`MesaUserspace` as present on both sides; Hermes still owns exclusive edges for
measured firmware, IOMMU, WPR/mailbox certificates, and formal models.
