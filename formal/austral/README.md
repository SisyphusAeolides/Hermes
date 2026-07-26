# Austral formal models (Hermes GSP)

These files **are** part of Hermes. They use Austral’s interface/body split:

| Extension | Role |
|---|---|
| `.aui` | Module interface (public linear types and functions) |
| `.aum` | Module body (implementations) |

## Why `tokei` does not list Austral

[tokei](https://github.com/XAMPPRocky/tokei) has **no built-in language** for
`.aui` / `.aum`, so those files are skipped as “unknown extension” unless you
map them. They still exist, are git-tracked, and are typechecked by:

```sh
sh scripts/check-formal.sh          # austral if installed
sh scripts/loc.sh                   # includes Austral line counts
```

## Modules

| Module | Linear concern |
|---|---|
| `HermesResources` | IOMMU domain, MMIO, DMA, WPR lock, OnlineSession |
| `HermesRings` | Command/event rings and slot tokens |
| `HermesFailClosed` | Live GPU fault/contain/release |
| `HermesWpr` | WPR2 plan + SEC2 Booter mailbox |
| `HermesBootstrap` | Five-file Turing bootstrap bundle |
| `HermesFirmware` | GSP-RM measure/hash/ELF seal |

Runtime code remains Rust; Austral owns **resource linearity** proofs.
