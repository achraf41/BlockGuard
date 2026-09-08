use blockguard_core::{
    Address, Amount, ChainId, Nonce, SignedTransaction, TransactionVersion, UnsignedTransaction,
};

use blockguard_crypto::{
    KeyPair, derive_address, sign_transaction, transaction_id, verify_transaction_signature,
};

fn alice_key() -> KeyPair {
    KeyPair::from_private_key_bytes([1u8; 32]).expect("test private key must be valid")
}

fn bob_key() -> KeyPair {
    KeyPair::from_private_key_bytes([2u8; 32]).expect("test private key must be valid")
}

fn create_transaction(alice: &KeyPair, recipient: Address, amount: u64) -> UnsignedTransaction {
    UnsignedTransaction::new(
        TransactionVersion::V1,
        ChainId::new(1),
        alice.public_key(),
        recipient,
        Amount::new(amount),
        Nonce::ZERO,
    )
}

#[test]
fn generated_public_key_is_compressed() {
    let key_pair = KeyPair::generate();

    let public_key = key_pair.public_key();

    assert_eq!(public_key.as_bytes().len(), 33);

    assert!(public_key.as_bytes()[0] == 0x02 || public_key.as_bytes()[0] == 0x03);
}

#[test]
fn address_derivation_is_deterministic() {
    let alice = alice_key();

    let public_key = alice.public_key();

    let address1 = derive_address(&public_key);

    let address2 = derive_address(&public_key);

    assert_eq!(address1, address2);
}

#[test]
fn valid_transaction_signature_verifies() {
    let alice = alice_key();
    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let unsigned = create_transaction(&alice, bob_address, 500);

    let signed = sign_transaction(&alice, unsigned).expect("signing must succeed");

    assert!(verify_transaction_signature(&signed).is_ok());
}

#[test]
fn modified_transaction_fails_signature_verification() {
    let alice = alice_key();
    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let unsigned = create_transaction(&alice, bob_address, 500);

    let signed = sign_transaction(&alice, unsigned).expect("signing must succeed");

    /*
     * Attacker changes:
     *
     * amount = 500
     *
     * to:
     *
     * amount = 50_000
     *
     * but keeps Alice's original signature.
     */
    let modified_payload = UnsignedTransaction::new(
        signed.payload().version(),
        signed.payload().chain_id(),
        *signed.payload().sender_public_key(),
        *signed.payload().recipient(),
        Amount::new(50_000),
        signed.payload().nonce(),
    );

    let forged = SignedTransaction::new(modified_payload, *signed.signature());

    assert!(verify_transaction_signature(&forged).is_err());
}

#[test]
fn signing_with_wrong_key_is_rejected() {
    let alice = alice_key();
    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let transaction = create_transaction(&alice, bob_address, 500);

    /*
     * Transaction says:
     *
     * sender public key = Alice
     *
     * but Bob attempts to sign it.
     */
    assert!(sign_transaction(&bob, transaction,).is_err());
}

#[test]
fn transaction_id_is_deterministic() {
    let alice = alice_key();
    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let transaction = create_transaction(&alice, bob_address, 500);

    let signed = sign_transaction(&alice, transaction).expect("signing must succeed");
    let signed2 = signed.clone();
    let id1 = transaction_id(&signed);

    let id2 = transaction_id(&signed2);

    assert_eq!(id1, id2);
}
