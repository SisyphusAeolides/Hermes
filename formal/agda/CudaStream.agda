{-# OPTIONS --safe --without-K #-}

module CudaStream where

data Nat : Set where
  zero : Nat
  suc : Nat -> Nat

data GspGate : Set where
  gspOffline : GspGate
  gspOnline : GspGate

data Stream : Set where
  stream : GspGate -> Stream

data Event : Set where
  event : GspGate -> Event

data Launch : Set where
  launch : Stream -> Nat -> Nat -> Launch
