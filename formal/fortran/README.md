# Fortran formal models (Hermes GSP)

Hermes is built with **Rust**, **Fortran**, **Idris2**, and **Agda**.

Fortran owns the **exclusive resource / ownership discipline**: IOMMU domains,
WPR locks, rings, firmware seals, mailbox sessions, and GPU lifecycle handles
are modeled as handles with a live flag. Transfer subroutines require
`live=.true.`, kill the source, and return a new live handle. Double-consume is
`error stop`.

| Module | Concern |
|---|---|
| `hermes_resources` | PCI → firmware → domain → BAR/DMA/WPR → Online session |
| `hermes_rings` | Command/event rings and slot tokens |
| `hermes_lifecycle` | Live GPU fault / contain / release lifecycle |
| `hermes_wpr` | WPR2 plan + SEC2 Booter mailbox |
| `hermes_bootstrap` | Five-file Turing bootstrap bundle |
| `hermes_firmware` | GSP-RM measure / hash / ELF seal |
| `hermes_mailbox` | Falcon HELLO session |
| `hermes_host_gate` | Host facts → Online authority |
| `hermes_dropin` | smi / catalog session phase |
| `hermes_drm_kms` | Atomic modeset session |
| `hermes_cccl` | CUDA driver/context/buffer |
| `hermes_nvkm_gsp` | Nouveau-shaped firmware bundle |
| `hermes_check` | Executable gate program |

## Build / check

```sh
# From repo root
sh scripts/check-formal.sh
# or directly:
make -C formal/fortran check
```

Requires **gfortran** (GCC Fortran, free-form F2018).
