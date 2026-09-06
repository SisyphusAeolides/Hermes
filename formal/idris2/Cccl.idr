module Cccl

%default total

||| CCCL component lattice (open CUDA C++ libraries).
public export
data CcclComponent = Thrust | Cub | Libcudacxx

public export
thrustPublicHeaders : Nat
thrustPublicHeaders = 64

public export
cubModules : Nat
cubModules = 86

||| Hermes CUDA Online requires GSP Online first.
public export
data CudaGate = GspOnline | DriverReady | ContextBound | ModuleLoaded

public export
data CudaSession : Type where
  MkCudaSession :
    (gsp : CudaGate) ->
    (drv : CudaGate) ->
    (ctx : CudaGate) ->
    {auto gOk : gsp = GspOnline} ->
    {auto dOk : drv = DriverReady} ->
    {auto cOk : ctx = ContextBound} ->
    CudaSession
