use blockguard_core::{Amount, Nonce};
use blockguard_crypto::{derive_address, verify_transaction_signature};

use crate::{Account, State, StateError};

impl State {
    pub fn apply_transaction(
        &mut self,
        tx: &blockguard_core::SignedTransaction,
    ) -> Result<(), StateError> {
        verify_transaction_signature(tx).map_err(|_| StateError::InvalidSignature)?;

        let sender_public_key = tx.payload().sender_public_key();
        let sender_address = derive_address(sender_public_key);
        let recipient_address = *tx.payload().recipient();
        let amount = tx.payload().amount();
        let expected_nonce = self.nonce(&sender_address)?;

        if tx.payload().nonce() != expected_nonce {
            return Err(StateError::InvalidNonce);
        }
        let sender_balance = self.balance(&sender_address)?;
        if sender_balance < amount {
            return Err(StateError::InsufficientBalance);
        }

        let new_sender_balance = sender_balance
            .checked_sub(amount)
            .ok_or(StateError::InsufficientBalance)?;

        let recipient_balance = self
            .account(&recipient_address)
            .map(Account::balance)
            .unwrap_or(Amount::ZERO);

        let new_recipient_balance = recipient_balance
            .checked_add(amount)
            .ok_or(StateError::BalanceOverflow)?;

        let new_sender_nonce = expected_nonce
            .checked_increment()
            .ok_or(StateError::NonceOverflow)?;

        let recipient_nonce = self
            .account(&recipient_address)
            .map(Account::nonce)
            .unwrap_or(Nonce::ZERO);

        if sender_address == recipient_address {
            self.set_account(
                sender_address,
                Account::new(sender_balance, new_sender_nonce),
            );
            return Ok(());
        }

        self.set_account(
            sender_address,
            Account::new(new_sender_balance, new_sender_nonce),
        );
        self.set_account(
            recipient_address,
            Account::new(new_recipient_balance, recipient_nonce),
        );

        Ok(())
    }
}
