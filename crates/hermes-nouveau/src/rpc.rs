//! GSP RPC reply policy reverse-engineered from Nouveau `nvkm_gsp` docs.
//!
//! Reference: include/nvkm/subdev/gsp.h — reply policies NOWAIT/NOSEQ/RECV/POLL.

use core::fmt;

/// When sending a GSP RPC command, reply handling modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum RpcReplyPolicy {
    /// Return immediately after the command is issued.
    NoWait = 0,
    /// Like NoWait but do not emit an RPC sequence number.
    NoSeq = 1,
    /// Wait and receive the entire GSP RPC message.
    Recv = 2,
    /// Wait for a specific reply and discard it before returning.
    Poll = 3,
}

impl RpcReplyPolicy {
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::NoWait),
            1 => Some(Self::NoSeq),
            2 => Some(Self::Recv),
            3 => Some(Self::Poll),
            _ => None,
        }
    }

    pub const fn waits(self) -> bool {
        matches!(self, Self::Recv | Self::Poll)
    }

    pub const fn emits_sequence(self) -> bool {
        !matches!(self, Self::NoSeq)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RpcHeader {
    pub function: u32,
    pub sequence: u32,
    pub payload_bytes: u32,
    pub policy: RpcReplyPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RpcError {
    SequenceMismatch,
    Timeout,
    Fault,
    BufferTooSmall,
    PolicyDenied,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SequenceMismatch => write!(f, "rpc sequence mismatch"),
            Self::Timeout => write!(f, "rpc timeout"),
            Self::Fault => write!(f, "rpc fault"),
            Self::BufferTooSmall => write!(f, "rpc buffer too small"),
            Self::PolicyDenied => write!(f, "rpc policy denied"),
        }
    }
}

/// Validate a reply against the request under the selected policy.
pub fn validate_reply(request: &RpcHeader, reply_fn: u32, reply_seq: u32) -> Result<(), RpcError> {
    if !request.policy.waits() {
        return Ok(());
    }
    if reply_fn != request.function {
        return Err(RpcError::Fault);
    }
    if request.policy.emits_sequence() && reply_seq != request.sequence {
        return Err(RpcError::SequenceMismatch);
    }
    Ok(())
}

/// Hermes edge: refuse to mark session Online on RPC fault (Nouveau may leave
/// partial device state; Hermes quarantines).
pub fn hermes_rpc_fault_is_terminal() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recv_requires_matching_sequence() {
        let req = RpcHeader {
            function: 0x10,
            sequence: 7,
            payload_bytes: 32,
            policy: RpcReplyPolicy::Recv,
        };
        assert!(validate_reply(&req, 0x10, 7).is_ok());
        assert_eq!(
            validate_reply(&req, 0x10, 8),
            Err(RpcError::SequenceMismatch)
        );
    }

    #[test]
    fn nowait_skips_sequence_check() {
        let req = RpcHeader {
            function: 1,
            sequence: 0,
            payload_bytes: 0,
            policy: RpcReplyPolicy::NoWait,
        };
        assert!(validate_reply(&req, 99, 99).is_ok());
    }
}
