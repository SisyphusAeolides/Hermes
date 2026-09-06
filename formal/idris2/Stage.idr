module Stage

%default total

||| Full-image DMA staging: every published byte contributes to the digest.
public export
data StageChunk = MkChunk Nat

public export
bytesStaged : List StageChunk -> Nat
bytesStaged [] = Z
bytesStaged (MkChunk n :: xs) = n + bytesStaged xs

public export
data GspGate = GspOffline | GspOnline

||| Online requires staged digest match + mailbox ready + WPR locked.
public export
data Evidence = MkEvidence Bool Bool Bool

public export
canIgnite : GspGate -> Evidence -> Bool
canIgnite GspOffline _ = False
canIgnite GspOnline (MkEvidence digestOk mailboxOk wprOk) =
  digestOk && mailboxOk && wprOk

public export
data OnlineSession : Type where
  MkOnline :
    (g : GspGate) ->
    (e : Evidence) ->
    {auto ok : canIgnite g e = True} ->
    OnlineSession
