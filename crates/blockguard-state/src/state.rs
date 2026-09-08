use std::collections::HashMap;

use blockguard_core::{Address, Amount, Nonce};

use crate::{Account, StateError};

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
}
