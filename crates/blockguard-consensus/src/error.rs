use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowError {
    NonceExhausted,
}

impl fmt::Display for PowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonceExhausted => {
                write!(f, "proof-of-work nonce space exhausted")
            }
        }
    }
}

impl Error for PowError {}
