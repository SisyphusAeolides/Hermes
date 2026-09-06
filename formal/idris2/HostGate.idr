module HostGate

%default total

||| Host Online requires IOMMU group, no foreign driver, and mapped BAR0.
public export
data DriverKind = Unbound | HermesNvidia | Foreign

public export
record HostFacts where
  constructor MkFacts
  hasIommu : Bool
  driver : DriverKind
  barDescribed : Bool
  barMapped : Bool

public export
isolationReady : HostFacts -> Bool
isolationReady f = hasIommu f && case driver f of
  Foreign => False
  _ => True

public export
mayClaimOnline : HostFacts -> Bool
mayClaimOnline f =
  isolationReady f && barDescribed f && barMapped f

public export
data OnlineAuthority : Type where
  MkAuth :
    (f : HostFacts) ->
    {auto ok : mayClaimOnline f = True} ->
    OnlineAuthority
