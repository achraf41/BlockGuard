use crate::types::{Address, Amount, ChainId, Nonce, PublicKeyBytes, TransactionVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsignedTransaction {
    version: TransactionVersion,
    chain_id: ChainId,
    sender_public_key: PublicKeyBytes,
    recipient: Address,
    amount: Amount,
    nonce: Nonce,
}

impl UnsignedTransaction {
    pub fn new(
        version: TransactionVersion,
        chain_id: ChainId,
        sender_public_key: PublicKeyBytes,
        recipient: Address,
        amount: Amount,
        nonce: Nonce,
    ) -> Self {
        Self {
            version,
            chain_id,
            sender_public_key,
            recipient,
            amount,
            nonce,
        }
    }

    pub const fn version(&self) -> TransactionVersion {
        self.version
    }

    pub const fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub const fn sender_public_key(&self) -> &PublicKeyBytes {
        &self.sender_public_key
    }

    pub const fn recipient(&self) -> &Address {
        &self.recipient
    }

    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn nonce(&self) -> Nonce {
        self.nonce
    }
}
