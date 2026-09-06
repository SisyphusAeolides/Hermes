{-# OPTIONS --safe --without-K #-}

module DrmKms where

data Nat : Set where
  zero : Nat
  suc : Nat -> Nat

data DrmObject : Set where
  connector : DrmObject
  crtc : DrmObject
  plane : DrmObject
  framebuffer : DrmObject

data GspGate : Set where
  gspOffline : GspGate
  gspOnline : GspGate

data CommitOutcome : Set where
  rejected : CommitOutcome
  applied : CommitOutcome

commit : GspGate -> CommitOutcome
commit gspOffline = rejected
commit gspOnline = applied

data ModesetSession : Set where
  modeset : GspGate -> Nat -> ModesetSession

virtualDesktopObjects : Nat
virtualDesktopObjects = suc (suc (suc zero))

dualHeadObjects : Nat
dualHeadObjects = suc (suc (suc (suc (suc (suc zero)))))
