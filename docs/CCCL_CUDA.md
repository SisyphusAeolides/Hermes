# CCCL → Hermes CUDA compatibility

## Source

[NVIDIA/cccl](https://github.com/NVIDIA/cccl) — **CUDA Core Compute Libraries**
(Apache-2.0 / open source):

| Library | Role |
|---|---|
| **Thrust** | High-level parallel algorithms (`copy`, `sort`, `reduce`, …) |
| **CUB** | CUDA cooperative primitives (block/warp/device) |
| **libcudacxx** | CUDA C++ standard library (`cuda::std::…`) |

Inventoried version (generator default): see `cccl-version.json` (currently **3.5.0**).

## What CCCL is *not*

CCCL is **not** the proprietary CUDA driver or runtime (`libcuda.so`, `libcudart.so`
full surface, NVCC, PTX JIT). Hermes still needs:

1. GSP Online (`hermes-gsp` / Nouveau / OpenRM path)
2. Driver/runtime shell (`hermes-cuda`) — **this crate family**
3. CCCL algorithm catalog + host subset (`hermes-cccl`)
4. Later: device codegen / PTX path for kernel launch

## Generator

```sh
python3 scripts/reverse-engineer-cccl.py \
  --cccl /path/to/cccl \
  --out generated/cccl-re
```

Produces Thrust/CUB tables, formal models, and inventory JSON.

## Hermes layout

| Crate | Purpose |
|---|---|
| `hermes-cccl` | Thrust header catalog, CUB modules, **host** algorithm subset |
| `hermes-cuda` | `cu*` / `cuda*` ABI shell, **fails closed without GSP Online** |

## Fail-closed CUDA

```text
cuInit / cudaGetDeviceCount
        │
        ▼
  GSP Online?  ──no──►  CUDA_ERROR_HERMES_GSP_OFFLINE
        │ yes
        ▼
  driver init → context → alloc/memcpy/module/stream/event/launch
```

Driver surface currently includes (GSP-gated): `cuInit`, device query/attrs,
`cuCtx*` / primary context retain-release, `cuMem*` / `cuMemGetInfo_v2`,
`cuMemcpy*`, `cuMemset*`, `cuModule*` / `cuLaunchKernel`, `cuStream*`,
`cuEvent*`, plus runtime aliases (`cudaMalloc`, `cudaMemGetInfo`,
`cudaSetDevice`, `cudaStream*`, …).

Host Thrust algorithms can run on CPU without a device; they still do not
claim a GPU.

```sh
cargo run -p hermes-ctl --bin hermes-ctl -- cuda-smoke deep
```

## Formal models

- `formal/idris2/Cccl.idr`, `formal/idris2/CudaStream.idr`
- `formal/agda/Cccl.agda`
- `formal/fortran/hermes_cccl.f90` (exclusive driver/context/buffer handles)
