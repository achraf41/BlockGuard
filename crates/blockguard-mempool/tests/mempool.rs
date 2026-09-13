use blockguard_core::{
    Address, Amount, ChainId, Nonce, SignatureBytes, SignedTransaction, TransactionVersion,
    UnsignedTransaction,
};
use blockguard_crypto::{KeyPair, derive_address, sign_transaction, transaction_id};
use blockguard_mempool::{Mempool, MempoolError};
use blockguard_state::{Account, State};

fn key(byte: u8) -> KeyPair {
    KeyPair::from_private_key_bytes([byte; 32]).unwrap()
}
fn address(key: &KeyPair) -> Address {
    derive_address(&key.public_key())
}
fn tx(sender: &KeyPair, recipient: Address, amount: u64, nonce: u64) -> SignedTransaction {
    sign_transaction(
        sender,
        UnsignedTransaction::new(
            TransactionVersion::V1,
            ChainId::new(7),
            sender.public_key(),
            recipient,
            Amount::new(amount),
            Nonce::new(nonce),
        ),
    )
    .unwrap()
}
fn state() -> State {
    let mut state = State::new();
    state.set_account(
        address(&key(1)),
        Account::new(Amount::new(100), Nonce::ZERO),
    );
    state.set_account(
        address(&key(2)),
        Account::new(Amount::new(100), Nonce::ZERO),
    );
    state
}

#[test]
fn valid_transaction_is_admitted_and_queryable() {
    let transaction = tx(&key(1), address(&key(2)), 10, 0);
    let id = transaction_id(&transaction);
    let mut pool = Mempool::new(ChainId::new(7));
    assert_eq!(pool.admit(transaction.clone(), &state()).unwrap(), id);
    assert_eq!(pool.len(), 1);
    assert!(pool.contains(&id));
    assert_eq!(pool.get(&id), Some(&transaction));
}

#[test]
fn duplicate_transaction_id_is_rejected() {
    let transaction = tx(&key(1), address(&key(2)), 10, 0);
    let mut pool = Mempool::new(ChainId::new(7));
    pool.admit(transaction.clone(), &state()).unwrap();
    assert_eq!(
        pool.admit(transaction, &state()),
        Err(MempoolError::DuplicateTransaction)
    );
}

#[test]
fn invalid_signature_is_rejected() {
    let valid = tx(&key(1), address(&key(2)), 10, 0);
    let invalid =
        SignedTransaction::new(valid.payload().clone(), SignatureBytes::from_bytes([0; 64]));
    assert_eq!(
        Mempool::new(ChainId::new(7)).admit(invalid, &state()),
        Err(MempoolError::InvalidSignature)
    );
}

#[test]
fn stale_nonce_is_rejected() {
    let alice = key(1);
    let mut confirmed = state();
    confirmed.set_account(
        address(&alice),
        Account::new(Amount::new(100), Nonce::new(1)),
    );
    assert_eq!(
        Mempool::new(ChainId::new(7)).admit(tx(&alice, address(&key(2)), 1, 0), &confirmed),
        Err(MempoolError::StaleNonce)
    );
}

#[test]
fn conflicting_sender_nonce_is_rejected() {
    let alice = key(1);
    let mut pool = Mempool::new(ChainId::new(7));
    pool.admit(tx(&alice, address(&key(2)), 1, 0), &state())
        .unwrap();
    assert_eq!(
        pool.admit(tx(&alice, Address::from_bytes([9; 20]), 2, 0), &state()),
        Err(MempoolError::ConflictingNonce)
    );
}

#[test]
fn sequential_future_nonces_can_coexist() {
    let alice = key(1);
    let mut pool = Mempool::new(ChainId::new(7));
    for nonce in 0..3 {
        pool.admit(tx(&alice, address(&key(2)), 10, nonce), &state())
            .unwrap();
    }
    assert_eq!(pool.len(), 3);
    assert_eq!(
        pool.candidates(&state(), 10)
            .iter()
            .map(|t| t.payload().nonce())
            .collect::<Vec<_>>(),
        vec![Nonce::new(0), Nonce::new(1), Nonce::new(2)]
    );
}

#[test]
fn candidate_order_is_deterministic() {
    let first = tx(&key(1), Address::from_bytes([8; 20]), 1, 0);
    let second = tx(&key(2), Address::from_bytes([9; 20]), 1, 0);
    let expected_first = if transaction_id(&first).as_bytes() < transaction_id(&second).as_bytes() {
        transaction_id(&first)
    } else {
        transaction_id(&second)
    };
    let mut a = Mempool::new(ChainId::new(7));
    let mut b = Mempool::new(ChainId::new(7));
    a.admit(first.clone(), &state()).unwrap();
    a.admit(second.clone(), &state()).unwrap();
    b.admit(second, &state()).unwrap();
    b.admit(first, &state()).unwrap();
    let ids = |pool: &Mempool| {
        pool.candidates(&state(), 10)
            .iter()
            .map(transaction_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&a), ids(&b));
    assert_eq!(ids(&a)[0], expected_first);
}

#[test]
fn revalidation_removes_confirmed_transaction() {
    let transaction = tx(&key(1), address(&key(2)), 10, 0);
    let id = transaction_id(&transaction);
    let mut pool = Mempool::new(ChainId::new(7));
    let mut confirmed = state();
    pool.admit(transaction.clone(), &confirmed).unwrap();
    confirmed.apply_transaction(&transaction).unwrap();
    pool.revalidate(&confirmed);
    assert!(!pool.contains(&id));
}
