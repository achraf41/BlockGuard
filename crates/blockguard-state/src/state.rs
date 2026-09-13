use std::collections::HashMap;

use blockguard_core::{Address, Amount, Nonce, StateRoot};
use blockguard_crypto::sha256;

use crate::{Account, StateError};

const STATE_DOMAIN: &[u8] = b"BLOCKGUARD_STATE_V1";
const ACCOUNT_COUNT_LENGTH: usize = 8;
const ENCODED_ACCOUNT_LENGTH: usize = Address::LENGTH + 8 + 8;

#[derive(Debug, Clone, Default)]
pub struct State {
    accounts: HashMap<Address, Account>,
}

impl State {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn set_account(&mut self, address: Address, account: Account) {
        self.accounts.insert(address, account);
    }

    pub fn create_account(&mut self, address: Address) {
        self.accounts.entry(address).or_insert_with(Account::empty);
    }

    pub fn account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }

    pub fn balance(&self, address: &Address) -> Result<Amount, StateError> {
        self.accounts
            .get(address)
            .map(Account::balance)
            .ok_or(StateError::AccountNotFound)
    }

    pub fn nonce(&self, address: &Address) -> Result<Nonce, StateError> {
        self.accounts
            .get(address)
            .map(Account::nonce)
            .ok_or(StateError::AccountNotFound)
    }

    pub fn accounts(&self) -> impl Iterator<Item = (&Address, &Account)> {
        self.accounts.iter()
    }

    pub fn state_root(&self) -> StateRoot {
        let mut accounts: Vec<_> = self.accounts.iter().collect();

        accounts.sort_unstable_by_key(|(address, _)| address.as_bytes());

        let mut encoded = Vec::with_capacity(
            STATE_DOMAIN.len() + ACCOUNT_COUNT_LENGTH + accounts.len() * ENCODED_ACCOUNT_LENGTH,
        );

        encoded.extend_from_slice(STATE_DOMAIN);
        encoded.extend_from_slice(&(accounts.len() as u64).to_be_bytes());

        for (address, account) in accounts {
            encoded.extend_from_slice(address.as_bytes());
            encoded.extend_from_slice(&account.balance().value().to_be_bytes());
            encoded.extend_from_slice(&account.nonce().as_u64().to_be_bytes());
        }

        StateRoot::from_hash(sha256(&encoded))
    }
}
