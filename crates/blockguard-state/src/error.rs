use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    AccountNotFound,
    InsufficientBalance,
    InvalidNonce,
    BalanceOverflow,
    NonceOverflow,
    InvalidSignature,
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccountNotFound => {
                write!(f, "account not found")
            }
            Self::InsufficientBalance => {
                write!(f, "insufficient balance")
            }
            Self::InvalidNonce => {
                write!(f, "invalid transaction nonce")
            }
            Self::BalanceOverflow => {
                write!(f, "balance overflow")
            }
            Self::NonceOverflow => {
                write!(f, "nonce overflow")
            }
            Self::InvalidSignature => {
                write!(f, "transaction signature is invalid")
            }
        }
    }
}

impl Error for StateError {}
