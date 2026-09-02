module DrmKms

%default total

||| DRM object kinds used by Hermes atomic modeset.
public export
data DrmObject = Connector | Crtc | Plane | Framebuffer

||| Scanout is only legal when GSP is Online (Hermes evidence gate).
public export
data GspGate = GspOffline | GspOnline

public export
data CommitOutcome = Rejected | Applied

||| Atomic commit policy: Offline never applies.
public export
commit : GspGate -> CommitOutcome
commit GspOffline = Rejected
commit GspOnline = Applied

public export
data ModesetSession : Type where
  MkModeset :
    (gsp : GspGate) ->
    (activeCrtcs : Nat) ->
    {auto online : gsp = GspOnline} ->
    ModesetSession

||| Virtual desktop topology size (single head).
public export
virtualDesktopObjects : Nat
virtualDesktopObjects = 3  -- connector + crtc + primary plane

||| Dual-head topology object count (connectors+crtcs+planes).
public export
dualHeadObjects : Nat
dualHeadObjects = 6

||| GEM dumb buffers also require Online.
public export
data GemOp = DumbCreate | Map | Destroy

public export
gemAllowed : GspGate -> GemOp -> Bool
gemAllowed GspOffline _ = False
gemAllowed GspOnline _ = True

public export
data PageFlip : Type where
  MkFlip :
    (gsp : GspGate) ->
    (fb : Nat) ->
    {auto online : gsp = GspOnline} ->
    PageFlip

public export
flipNeedsFb : Nat -> Bool
flipNeedsFb Z = False
flipNeedsFb _ = True
