{-# OPTIONS --safe --without-K #-}

module Stage where

data Nat : Set where
  zero : Nat
  suc : Nat -> Nat

data Bool : Set where
  false : Bool
  true : Bool

_&&_ : Bool -> Bool -> Bool
false && _ = false
true && b = b

data GspGate : Set where
  gspOffline : GspGate
  gspOnline : GspGate

data Evidence : Set where
  evidence : Bool -> Bool -> Bool -> Evidence

canIgnite : GspGate -> Evidence -> Bool
canIgnite gspOffline _ = false
canIgnite gspOnline (evidence d m w) = (d && m) && w
