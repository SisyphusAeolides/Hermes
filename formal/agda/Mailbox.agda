{-# OPTIONS --safe --without-K #-}

module Mailbox where

data GspGate : Set where
  gspOffline : GspGate
  gspOnline : GspGate

data MailboxReady : Set where
  notReady : MailboxReady
  ready : MailboxReady

data Bool : Set where
  false : Bool
  true : Bool

handshake : GspGate -> MailboxReady -> Bool
handshake gspOffline _ = false
handshake gspOnline notReady = false
handshake gspOnline ready = true
