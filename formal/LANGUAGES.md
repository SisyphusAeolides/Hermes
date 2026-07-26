# Hermes formal languages

Hermes is specified and checked in four languages: **Rust**, **Fortran**,
**Idris2**, and **Agda**.

| Language | Role in Hermes |
|---|---|
| **Rust** | Executable runtime: wire ABI, platform HAL, GSP personality, Linux drop-in surfaces, settings/NVML shims |
| **Idris2** | Total phase lattice and online certificates (`HermesAuthority.idr`) — no jump to Online without every gate |
| **Agda** | Safe feature-lattice and ring geometry (`HermesWire.agda`) under `--safe --without-K` |
| **Fortran** | Exclusive resource / ownership discipline for IOMMU domains, firmware seals, MMIO/DMA, WPR locks, armed rings, and live sessions |

## Why Fortran

Idris and Agda prove *when* Online is legal. Fortran proves *who still owns
the hardware resources* after each step, using exclusive handles:

- A domain / WPR / Online session handle is live or dead; transfer kills the source.
- Double-consume is `error stop` (fail-closed at the formal gate).
- `ignite` requires simultaneous consumption of domain + WPR + mailbox/ready/features evidence.
- Command and event rings are exclusive; a slot token is borrowed and returned through `take_slot` / `retire_slot`.
- A live GPU handle is exclusive; fault/contain/release paths consume it exactly once.

There is no Fortran path that constructs Online after skipping isolation.

## Layout

```
formal/
  idris2/HermesAuthority.idr …
  agda/HermesWire.agda …
  fortran/
    hermes_resources.f90
    hermes_rings.f90
    hermes_fail_closed.f90
    hermes_wpr.f90
    hermes_bootstrap.f90
    hermes_firmware.f90
    hermes_check.f90   # executable gate
    Makefile
    README.md
  toolchains.lock
  LANGUAGES.md
```

Typecheck / run the formal gate with:

```sh
sh scripts/check-formal.sh
# requires idris2, agda, and gfortran when run with --strict
sh scripts/check-formal.sh --strict
make -C formal/fortran check
```
