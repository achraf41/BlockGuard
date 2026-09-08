use crate::{transaction::UnsignedTransaction, types::SignatureBytes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedTransaction {
    payload: UnsignedTransaction,
    signature: SignatureBytes,
}

impl SignedTransaction {
    pub fn new(payload: UnsignedTransaction, signature: SignatureBytes) -> Self {
        Self { payload, signature }
    }

    pub const fn payload(&self) -> &UnsignedTransaction {
        &self.payload
    }

    pub const fn signature(&self) -> &SignatureBytes {
        &self.signature
    }
}
