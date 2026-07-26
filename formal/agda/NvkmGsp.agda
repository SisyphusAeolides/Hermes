{-# OPTIONS --safe --without-K #-}

module NvkmGsp where

data Empty : Set where

Not : Set -> Set
Not A = A -> Empty

data Nat : Set where
  zero : Nat
  suc : Nat -> Nat

data FirmwareStyle : Set where
  booter : FirmwareStyle
  fmc : FirmwareStyle

data Role : FirmwareStyle -> Set where
  booterLoad : Role booter
  booterUnload : Role booter
  bootloaderB : Role booter
  gspB : Role booter
  fmcR : Role fmc
  bootloaderF : Role fmc
  gspF : Role fmc

-- Hermes forbids claiming GSP online with a missing role.
data Complete : FirmwareStyle -> Set where
  completeBooter :
    Role booter -> Role booter -> Role booter -> Role booter -> Complete booter
  completeFmc :
    Role fmc -> Role fmc -> Role fmc -> Complete fmc

-- Inventory size from generator (13 fwif rows).
fwifRows : Nat
fwifRows =
  suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc zero))))))))))))

data Gate : Set where
  measured : Gate
  iommu : Gate
  wpr : Gate
  mailbox : Gate
  ready : Gate

data Online : Set where
  online : Gate -> Gate -> Gate -> Gate -> Gate -> Online
