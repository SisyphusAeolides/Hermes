# Formal models

Hermes formal authority is four languages, not three.

| Dir | Language | Obligation |
|---|---|---|
| `idris2/` | Idris2 | Total phase lattice and online certificates |
| `agda/` | Agda | Feature lattice and non-empty ring geometry |
| `austral/` | Austral | Linear ownership of domains, firmware, rings, sessions |

See [LANGUAGES.md](LANGUAGES.md) for the division of labor and why Austral
was added beyond the historical Rust + Idris + Agda set.

Pinned versions live in `toolchains.lock`. Check with:

```sh
sh scripts/check-formal.sh
sh scripts/check-formal.sh --strict   # requires idris2, agda, and austral
```
