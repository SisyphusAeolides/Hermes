{-# OPTIONS --safe --without-K #-}

module DropIn where

data Nat : Set where
  zero : Nat
  suc : Nat -> Nat

data Bool : Set where
  false : Bool
  true : Bool

data Surface : Set where
  nvidiaSmi : Surface
  nvml : Surface
  cudaDriver : Surface
  drmKms : Surface

data SessionPhase : Set where
  offline : SessionPhase
  online : SessionPhase

record Session : Set where
  constructor mkSession
  field
    phase : SessionPhase
    deviceCount : Nat

smiListsDevices : Session -> Bool
smiListsDevices (mkSession _ zero) = false
smiListsDevices (mkSession _ (suc _)) = true

telemetryLegal : Session -> Bool
telemetryLegal (mkSession online _) = true
telemetryLegal (mkSession offline _) = false

sessionSurfaces : Nat
sessionSurfaces = suc (suc (suc (suc zero)))

processListLegal : SessionPhase -> Bool
processListLegal online = true
processListLegal offline = false

dropInKinds : Nat
dropInKinds = suc (suc (suc (suc (suc zero))))

kmodNames : Nat
kmodNames = suc (suc (suc (suc (suc zero))))

-- userspace bins: nvidia-smi, nvidia-settings, nvidia-modprobe
userspaceBins : Nat
userspaceBins = suc (suc (suc zero))
