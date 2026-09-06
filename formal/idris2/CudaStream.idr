module CudaStream

%default total

||| Stream/event session is only inhabitible under GSP Online.
public export
data GspGate = GspOffline | GspOnline

public export
data Stream : Type where
  MkStream : (gsp : GspGate) -> {auto ok : gsp = GspOnline} -> Stream

public export
data Event : Type where
  MkEvent : (gsp : GspGate) -> {auto ok : gsp = GspOnline} -> Event

public export
data Launch : Type where
  MkLaunch :
    (s : Stream) ->
    (gridX : Nat) ->
    (blockX : Nat) ->
    Launch

public export
launchLegal : Nat -> Nat -> Bool
launchLegal Z _ = False
launchLegal _ Z = False
launchLegal _ _ = True
