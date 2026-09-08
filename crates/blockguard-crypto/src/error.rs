use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    InvalidPrivatKey,
    InvalidPublicKey,
    InvalidSignatureEncoding,
    NonCanonicalSignature,
    SignatureVerificationFailed,
    SigningFailed,
    SenderPublicKeyMismatch,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrivatKey => {
                write!(f, "invalid secp256k1 private key")
            }

            Self::InvalidPublicKey => {
                write!(f, "invalid secp256k1 public key")
            }

            Self::InvalidSignatureEncoding => {
                write!(f, "invalid ECDSA signature encoding")
            }

            Self::NonCanonicalSignature => {
                write!(f, "ECDSA signature is not canonical low-S")
            }

            Self::SignatureVerificationFailed => {
                write!(f, "ECDSA signature verification failed")
            }

            Self::SigningFailed => {
                write!(f, "ECDSA signing failed")
            }

            Self::SenderPublicKeyMismatch => {
                write!(
                    f,
                    "transaction sender public key does not match signing key"
                )
            }
        }
    }
}

impl Error for CryptoError {}
