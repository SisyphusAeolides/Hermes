module NvkmGsp

%default total

||| Nouveau GSP firmware package style (from NVKM_GSP_FIRMWARE_* macros).
public export
data FirmwareStyle = BooterStyle | FmcStyle

||| Hermes improves on Nouveau by forbidding Online without measured admission.
public export
data HermesGate
  = FirmwareMeasured
  | IommuIsolated
  | WprLocked
  | MailboxOk
  | ReadyQueue

public export
booterRoles : Nat
booterRoles = 4

public export
fmcRoles : Nat
fmcRoles = 3

public export
rolesFor : FirmwareStyle -> Nat
rolesFor BooterStyle = booterRoles
rolesFor FmcStyle = fmcRoles

||| Inventory sizes extracted from Nouveau sources at generation time.
public export
nouveauBooterDeclarations : Nat
nouveauBooterDeclarations = 32

public export
nouveauFmcDeclarations : Nat
nouveauFmcDeclarations = 8

||| Online requires every gate evidence bit (Hermes > Nouveau running flag).
public export
data Online : Type where
  MkOnline :
    (firmware : HermesGate) ->
    (iommu : HermesGate) ->
    (wpr : HermesGate) ->
    (mailbox : HermesGate) ->
    (ready : HermesGate) ->
    {auto fOk : firmware = FirmwareMeasured} ->
    {auto iOk : iommu = IommuIsolated} ->
    {auto wOk : wpr = WprLocked} ->
    {auto mOk : mailbox = MailboxOk} ->
    {auto rOk : ready = ReadyQueue} ->
    Online
