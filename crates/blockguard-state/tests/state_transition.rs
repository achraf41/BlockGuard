use blockguard_core::{
    Address, Amount, ChainId, Nonce, SignedTransaction, TransactionVersion, UnsignedTransaction,
};

use blockguard_crypto::{KeyPair, derive_address, sign_transaction};

use blockguard_state::{Account, State, StateError};

fn alice_key() -> KeyPair {
    KeyPair::from_private_key_bytes([1u8; 32]).expect("Alice test key must be valid")
}

fn bob_key() -> KeyPair {
    KeyPair::from_private_key_bytes([2u8; 32]).expect("Bob test key must be valid")
}

fn signed_transfer(
    sender: &KeyPair,
    recipient: Address,
    amount: u64,
    nonce: u64,
) -> SignedTransaction {
    let unsigned = UnsignedTransaction::new(
        TransactionVersion::V1,
        ChainId::new(1),
        sender.public_key(),
        recipient,
        Amount::new(amount),
        Nonce::new(nonce),
    );

    sign_transaction(sender, unsigned).expect("test transaction must sign successfully")
}
#[test]
fn valid_transfer_updates_balances_and_sender_nonce() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));

    state.set_account(bob_address, Account::new(Amount::new(200), Nonce::ZERO));

    let tx = signed_transfer(&alice, bob_address, 300, 0);

    state
        .apply_transaction(&tx)
        .expect("valid transaction must succeed");

    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(700),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(500),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::new(1),);

    assert_eq!(state.nonce(&bob_address).unwrap(), Nonce::ZERO,);
}
#[test]
fn transfer_creates_recipient_account_when_missing() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));

    assert!(state.account(&bob_address).is_none());

    let tx = signed_transfer(&alice, bob_address, 250, 0);

    state.apply_transaction(&tx).expect("transfer must succeed");

    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(750),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(250),);

    assert_eq!(state.nonce(&bob_address).unwrap(), Nonce::ZERO,);
}

#[test]
fn insufficient_balance_is_rejected_without_changing_state() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(100), Nonce::ZERO));

    state.set_account(bob_address, Account::new(Amount::new(50), Nonce::ZERO));

    let tx = signed_transfer(&alice, bob_address, 200, 0);

    let result = state.apply_transaction(&tx);

    assert_eq!(result, Err(StateError::InsufficientBalance),);

    // Alice must still have 100.
    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(100),);

    // Bob must still have 50.
    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(50),);

    // Failed transaction must NOT consume the nonce.
    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::ZERO,);
}

#[test]
fn wrong_nonce_is_rejected_without_changing_state() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(
        alice_address,
        Account::new(Amount::new(1000), Nonce::new(2)),
    );

    state.set_account(bob_address, Account::new(Amount::new(100), Nonce::ZERO));

    // State expects Alice nonce = 2,
    // but transaction uses nonce = 1.
    let tx = signed_transfer(&alice, bob_address, 200, 1);

    let result = state.apply_transaction(&tx);

    assert_eq!(result, Err(StateError::InvalidNonce),);

    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(1000),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(100),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::new(2),);
}
#[test]
fn tampered_transaction_is_rejected_without_changing_state() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));

    state.set_account(bob_address, Account::new(Amount::new(100), Nonce::ZERO));

    // Alice genuinely signs a transfer of 100.
    let original = signed_transfer(&alice, bob_address, 100, 0);

    // Attacker creates a different payload:
    // 100 -> 900
    let tampered_payload = UnsignedTransaction::new(
        original.payload().version(),
        original.payload().chain_id(),
        *original.payload().sender_public_key(),
        *original.payload().recipient(),
        Amount::new(900),
        original.payload().nonce(),
    );

    // Attacker keeps Alice's original signature.
    let tampered_transaction = SignedTransaction::new(tampered_payload, *original.signature());

    let result = state.apply_transaction(&tampered_transaction);

    assert_eq!(result, Err(StateError::InvalidSignature),);

    // Nothing must change.
    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(1000),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(100),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::ZERO,);
}

#[test]
fn recipient_balance_overflow_is_rejected_without_changing_state() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(100), Nonce::ZERO));

    state.set_account(
        bob_address,
        Account::new(Amount::new(u64::MAX), Nonce::ZERO),
    );

    let tx = signed_transfer(&alice, bob_address, 1, 0);

    let result = state.apply_transaction(&tx);

    assert_eq!(result, Err(StateError::BalanceOverflow),);

    // Atomicity: sender must not lose the 1 coin.
    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(100),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(u64::MAX),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::ZERO,);
}

#[test]
fn nonce_overflow_is_rejected_without_changing_state() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(
        alice_address,
        Account::new(Amount::new(100), Nonce::new(u64::MAX)),
    );

    state.set_account(bob_address, Account::new(Amount::ZERO, Nonce::ZERO));

    let tx = signed_transfer(&alice, bob_address, 10, u64::MAX);

    let result = state.apply_transaction(&tx);

    assert_eq!(result, Err(StateError::NonceOverflow),);

    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(100),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::ZERO,);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::new(u64::MAX),);
}
#[test]
fn self_transfer_preserves_balance_and_increments_nonce() {
    let alice = alice_key();

    let alice_address = derive_address(&alice.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(500), Nonce::ZERO));

    let tx = signed_transfer(&alice, alice_address, 100, 0);

    state
        .apply_transaction(&tx)
        .expect("self transfer must succeed");

    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(500),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::new(1),);
}
#[test]
fn replayed_transaction_is_rejected() {
    let alice = alice_key();
    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));

    state.set_account(bob_address, Account::new(Amount::ZERO, Nonce::ZERO));

    let tx = signed_transfer(&alice, bob_address, 100, 0);

    // First execution.
    state
        .apply_transaction(&tx)
        .expect("first execution must succeed");

    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(900),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(100),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::new(1),);

    // Attacker broadcasts exactly the same
    // signed transaction again.
    let replay_result = state.apply_transaction(&tx);

    assert_eq!(replay_result, Err(StateError::InvalidNonce),);

    // Nothing changes after the replay attempt.
    assert_eq!(state.balance(&alice_address).unwrap(), Amount::new(900),);

    assert_eq!(state.balance(&bob_address).unwrap(), Amount::new(100),);

    assert_eq!(state.nonce(&alice_address).unwrap(), Nonce::new(1),);
}
