module Mailbox

%default total

||| Falcon mailbox RPC is only productive under GSP Online + non-zero response.
public export
data GspGate = GspOffline | GspOnline

public export
data MailboxReady = NotReady | Ready

public export
handshake : GspGate -> MailboxReady -> Bool
handshake GspOffline _ = False
handshake GspOnline NotReady = False
handshake GspOnline Ready = True

public export
data BootEvidence : Type where
  MkBoot :
    (gsp : GspGate) ->
    (ready : MailboxReady) ->
    {auto ok : handshake gsp ready = True} ->
    BootEvidence
