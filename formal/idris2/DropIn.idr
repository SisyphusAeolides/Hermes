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

||| Process table only legal when session Online.
public export
processListLegal : SessionPhase -> Bool
processListLegal Online = True
processListLegal Offline = False

||| Advertised open-stack drop-in kinds Hermes covers.
public export
dropInKinds : Nat
dropInKinds = 5  -- kmod, device, bin, lib, surface

public export
kmodNames : Nat
kmodNames = 5  -- nvidia, modeset, uvm, drm, peermem

||| Userspace bins: nvidia-smi, nvidia-settings, nvidia-modprobe.
public export
userspaceBins : Nat
userspaceBins = 3
