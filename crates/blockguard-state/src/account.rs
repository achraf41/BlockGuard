use blockguard_core::{Amount, Nonce};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    balance: Amount,
    nonce: Nonce,
}

impl Account {
    pub const fn new(balance: Amount, nonce: Nonce) -> Self {
        Self {
            balance: balance,
            nonce: nonce,
        }
    }

    pub const fn empty() -> Self {
        Self {
            balance: Amount::ZERO,
            nonce: Nonce::ZERO,
        }
    }

    pub const fn balance(&self) -> Amount {
        self.balance
    }
    pub const fn nonce(&self) -> Nonce {
        self.nonce
    }
}
