# Formal models

Hermes formal authority is four languages:

| Dir | Language | Obligation |
|---|---|---|
| `idris2/` | Idris2 | Total phase lattice and online certificates |
| `agda/` | Agda | Feature lattice and non-empty ring geometry |
| `fortran/` | Fortran | Exclusive ownership of domains, firmware, rings, sessions |

See [LANGUAGES.md](LANGUAGES.md) for the division of labor.

Pinned versions live in `toolchains.lock`. Check with:

```sh
sh scripts/check-formal.sh
sh scripts/check-formal.sh --strict   # requires idris2, agda, and gfortran
```
