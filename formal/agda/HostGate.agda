{-# OPTIONS --safe --without-K #-}

module HostGate where

data Bool : Set where
  false : Bool
  true : Bool

_&&_ : Bool -> Bool -> Bool
false && _ = false
true && b = b

data DriverKind : Set where
  unbound : DriverKind
  hermesNvidia : DriverKind
  foreign : DriverKind

record HostFacts : Set where
  field
    hasIommu : Bool
    driver : DriverKind
    barDescribed : Bool
    barMapped : Bool

notForeign : DriverKind -> Bool
notForeign foreign = false
notForeign _ = true

isolationReady : HostFacts -> Bool
isolationReady f = (HostFacts.hasIommu f) && notForeign (HostFacts.driver f)

mayClaimOnline : HostFacts -> Bool
mayClaimOnline f =
  (isolationReady f && HostFacts.barDescribed f) && HostFacts.barMapped f
