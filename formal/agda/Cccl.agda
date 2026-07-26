{-# OPTIONS --safe --without-K #-}

module Cccl where

data Nat : Set where
  zero : Nat
  suc : Nat -> Nat

data Component : Set where
  thrust : Component
  cub : Component
  libcu : Component

thrustHeaders : Nat
thrustHeaders = suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (suc (zero))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))

data Gate : Set where
  gspOnline : Gate
  driverReady : Gate
  contextBound : Gate

data CudaSession : Set where
  session : Gate -> Gate -> Gate -> CudaSession
