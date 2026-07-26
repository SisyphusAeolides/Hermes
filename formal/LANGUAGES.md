# Hermes formal languages

Hermes is specified and checked in four languages. Earlier Sisyphus-OS work
carried only Rust, Idris2, and Agda. **Austral is first-class here**: it owns
the linear resource discipline that the GSP path cannot express with free
types alone.

| Language | Role in Hermes |
|---|---|
| **Rust** | Executable runtime: wire ABI, platform HAL, GSP personality, Linux drop-in surfaces, settings/NVML shims |
| **Idris2** | Total phase lattice and online certificates (`HermesAuthority.idr`) — no jump to Online without every gate |
| **Agda** | Safe feature-lattice and ring geometry (`HermesWire.agda`) under `--safe --without-K` |
| **Austral** | Linear capabilities for IOMMU domains, firmware seals, MMIO/DMA, WPR locks, armed rings, and live sessions |

## Why Austral

Idris and Agda prove *when* Online is legal. Austral proves *who still owns
the hardware resources* after each step:

- An `IommuDomain` is `Linear`. Leak or double-release is a type error.
- `mapBar` / `allocateDma` / `lockWpr` consume the domain and return a linear
  handle that must eventually restore it.
- `OnlineSession` is only constructed by `ignite`, which consumes the domain
  and WPR lock together with free mailbox/ready/feature evidence.
- Command and event rings are linear; a slot token is borrowed and returned
  through `takeSlot` / `retireSlot`.
- A live GPU is linear; fault/contain/release paths consume it exactly once
  (`HermesFailClosed`).

There is no Austral constructor for an online GPU that skipped isolation.

## Layout

```
formal/
  idris2/HermesAuthority.idr
  agda/HermesWire.agda
  austral/
    HermesResources.aui/.aum
    HermesRings.aui/.aum
    HermesFailClosed.aui/.aum
    HermesWpr.aui/.aum
    HermesBootstrap.aui/.aum
    HermesFirmware.aui/.aum
    README.md
  toolchains.lock
  LANGUAGES.md
```

**Note:** `tokei` does not recognize `.aui`/`.aum`. Austral is still present;
use `sh scripts/loc.sh` for a count that includes it.

Austral modules use the interface/body split (`.aui` / `.aum`) required by the
language. Typecheck with:

```sh
sh scripts/check-formal.sh
# requires austral when run with --strict
sh scripts/check-formal.sh --strict
```
