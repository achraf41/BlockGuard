use blockguard_core::{
    Hash256, SignedTransaction, TransactionId, UnsignedTransaction,
    encode_signed_transaction_for_id, encode_unsigned_transaction_for_signing,
};

use crate::sha256;

pub fn transaction_signing_digest(tx: &UnsignedTransaction) -> Hash256 {
    let encoded = encode_unsigned_transaction_for_signing(tx);

    sha256(&encoded)
}

pub fn transaction_id(tx: &SignedTransaction) -> TransactionId {
    let encoded = encode_signed_transaction_for_id(tx);

    let hash = sha256(&encoded);

    TransactionId::from_hash(hash)
}
