use blockguard_core::{SignatureBytes, SignedTransaction, UnsignedTransaction};

use k256::ecdsa::{
    Signature, VerifyingKey,
    signature::hazmat::{PrehashSigner, PrehashVerifier},
};

use crate::{CryptoError, KeyPair, transaction_signing_digest};

pub fn sign_transaction(
    key_pair: &KeyPair,
    tx: UnsignedTransaction,
) -> Result<SignedTransaction, CryptoError> {
    let expected_public_key = key_pair.public_key();

    if tx.sender_public_key() != &expected_public_key {
        return Err(CryptoError::SenderPublicKeyMismatch);
    }

    let digest = transaction_signing_digest(&tx);

    let signature: Signature = key_pair
        .signing_key()
        .sign_prehash(digest.as_bytes())
        .map_err(|_| CryptoError::SigningFailed)?;

    let signature = signature.normalize_s();

    let encoded_signature = signature.to_bytes();
    let mut signature_bytes = [0u8; 64];

    signature_bytes.copy_from_slice(encoded_signature.as_ref());

    Ok(SignedTransaction::new(
        tx,
        SignatureBytes::from_bytes(signature_bytes),
    ))
}

pub fn verify_transaction_signature(tx: &SignedTransaction) -> Result<(), CryptoError> {
    let public_key_bytes = tx.payload().sender_public_key().as_bytes();

    let verfying_key = VerifyingKey::from_sec1_bytes(public_key_bytes)
        .map_err(|_| CryptoError::InvalidPublicKey)?;

    let signature = Signature::from_slice(tx.signature().as_bytes())
        .map_err(|_| CryptoError::InvalidSignatureEncoding)?;

    if signature.normalize_s() != signature {
        return Err(CryptoError::NonCanonicalSignature);
    }

    let digest = transaction_signing_digest(tx.payload());

    verfying_key
        .verify_prehash(digest.as_bytes(), &signature)
        .map_err(|_| CryptoError::SignatureVerificationFailed)
}
