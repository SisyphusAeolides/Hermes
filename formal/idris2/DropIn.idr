module DropIn

%default total

||| Proprietary-named surfaces read Hermes session state.
public export
data Surface = NvidiaSmi | Nvml | CudaDriver | DrmKms | Settings

public export
data SessionPhase = Offline | Online

public export
record Session where
  constructor MkSession
  phase : SessionPhase
  deviceCount : Nat

public export
smiListsDevices : Session -> Bool
smiListsDevices s = case deviceCount s of
  Z => False
  _ => True

public export
telemetryLegal : Session -> Bool
telemetryLegal s = case phase s of
  Online => True
  Offline => False

||| CUDA and smi share one Online promote.
public export
sessionSurfaces : Nat
sessionSurfaces = 4  -- smi, nvml, cuda, settings
