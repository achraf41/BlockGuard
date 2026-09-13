use blockguard_core::{Address, Amount, Nonce};
use blockguard_state::{Account, State};

fn address(byte: u8) -> Address {
    Address::from_bytes([byte; Address::LENGTH])
}

#[test]
fn empty_state_root_is_deterministic() {
    let state = State::new();

    assert_eq!(state.state_root(), state.state_root());
    assert_eq!(state.state_root(), State::new().state_root());
}

#[test]
fn insertion_order_does_not_change_state_root() {
    let first_address = address(1);
    let second_address = address(2);
    let first_account = Account::new(Amount::new(100), Nonce::new(3));
    let second_account = Account::new(Amount::new(200), Nonce::new(4));
    let mut first = State::new();
    let mut second = State::new();

    first.set_account(first_address, first_account.clone());
    first.set_account(second_address, second_account.clone());
    second.set_account(second_address, second_account);
    second.set_account(first_address, first_account);

    assert_eq!(first.state_root(), second.state_root());
}

#[test]
fn changing_balance_changes_state_root() {
    let account_address = address(1);
    let mut first = State::new();
    let mut second = State::new();

    first.set_account(
        account_address,
        Account::new(Amount::new(100), Nonce::new(3)),
    );
    second.set_account(
        account_address,
        Account::new(Amount::new(101), Nonce::new(3)),
    );

    assert_ne!(first.state_root(), second.state_root());
}

#[test]
fn changing_nonce_changes_state_root() {
    let account_address = address(1);
    let mut first = State::new();
    let mut second = State::new();

    first.set_account(
        account_address,
        Account::new(Amount::new(100), Nonce::new(3)),
    );
    second.set_account(
        account_address,
        Account::new(Amount::new(100), Nonce::new(4)),
    );

    assert_ne!(first.state_root(), second.state_root());
}

#[test]
fn changing_address_or_account_set_changes_state_root() {
    let mut original = State::new();
    let mut changed_address = State::new();
    let mut additional_account = State::new();
    let account = Account::new(Amount::new(100), Nonce::new(3));

    original.set_account(address(1), account.clone());
    changed_address.set_account(address(2), account.clone());
    additional_account.set_account(address(1), account);
    additional_account.set_account(address(2), Account::empty());

    assert_ne!(original.state_root(), changed_address.state_root());
    assert_ne!(original.state_root(), additional_account.state_root());
}
