use std::{collections::HashMap, error::Error, fmt};

use blockguard_core::{
    Address, ChainId, Nonce, SignedTransaction, TransactionId, TransactionVersion,
};
use blockguard_crypto::{derive_address, transaction_id, verify_transaction_signature};
use blockguard_state::{State, StateError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MempoolError {
    InvalidSignature,
    WrongChainId,
    UnsupportedTransactionVersion,
    DuplicateTransaction,
    SenderAccountNotFound,
    StaleNonce,
    ConflictingNonce,
    NonceGap,
    ImpossibleSpend(StateError),
}

impl fmt::Display for MempoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => write!(f, "invalid transaction signature"),
            Self::WrongChainId => write!(f, "wrong chain id"),
            Self::UnsupportedTransactionVersion => write!(f, "unsupported transaction version"),
            Self::DuplicateTransaction => write!(f, "duplicate transaction ID"),
            Self::SenderAccountNotFound => write!(f, "sender account does not exist"),
            Self::StaleNonce => write!(f, "transaction nonce is stale"),
            Self::ConflictingNonce => write!(f, "sender nonce already has a transaction"),
            Self::NonceGap => write!(f, "future transaction has a nonce gap"),
            Self::ImpossibleSpend(error) => write!(f, "transaction cannot execute: {error}"),
        }
    }
}

impl Error for MempoolError {}

#[derive(Debug, Clone)]
pub struct Mempool {
    chain_id: ChainId,
    transactions: HashMap<TransactionId, SignedTransaction>,
    sender_nonces: HashMap<([u8; Address::LENGTH], u64), TransactionId>,
}

impl Mempool {
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            transactions: HashMap::new(),
            sender_nonces: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.transactions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
    pub fn contains(&self, id: &TransactionId) -> bool {
        self.transactions.contains_key(id)
    }
    pub fn get(&self, id: &TransactionId) -> Option<&SignedTransaction> {
        self.transactions.get(id)
    }

    pub fn admit(
        &mut self,
        tx: SignedTransaction,
        state: &State,
    ) -> Result<TransactionId, MempoolError> {
        self.validate_static(&tx)?;
        let id = transaction_id(&tx);
        if self.contains(&id) {
            return Err(MempoolError::DuplicateTransaction);
        }

        let sender = derive_address(tx.payload().sender_public_key());
        let confirmed_nonce = state
            .nonce(&sender)
            .map_err(|_| MempoolError::SenderAccountNotFound)?;
        if tx.payload().nonce() < confirmed_nonce {
            return Err(MempoolError::StaleNonce);
        }
        let key = sender_nonce_key(sender, tx.payload().nonce());
        if self.sender_nonces.contains_key(&key) {
            return Err(MempoolError::ConflictingNonce);
        }

        // Validate the complete sender sequence against confirmed state. Future
        // incoming transfers are intentionally not treated as spendable in V1.
        let mut sequence: Vec<&SignedTransaction> = self
            .transactions
            .values()
            .filter(|queued| derive_address(queued.payload().sender_public_key()) == sender)
            .collect();
        sequence.push(&tx);
        sequence.sort_unstable_by_key(|queued| queued.payload().nonce().as_u64());
        let mut trial = state.clone();
        let mut expected = confirmed_nonce;
        for queued in sequence {
            if queued.payload().nonce() != expected {
                return Err(MempoolError::NonceGap);
            }
            trial
                .apply_transaction(queued)
                .map_err(MempoolError::ImpossibleSpend)?;
            expected = expected
                .checked_increment()
                .ok_or_else(|| MempoolError::ImpossibleSpend(StateError::NonceOverflow))?;
        }

        self.sender_nonces.insert(key, id);
        self.transactions.insert(id, tx);
        Ok(id)
    }

    /// Selects executable transactions by repeatedly choosing the lowest TxID
    /// among all transactions executable against the evolving trial state.
    pub fn candidates(&self, state: &State, max_count: usize) -> Vec<SignedTransaction> {
        let mut trial = state.clone();
        let mut remaining: Vec<_> = self.transactions.iter().collect();
        let mut selected = Vec::new();
        while selected.len() < max_count {
            let choice = remaining
                .iter()
                .enumerate()
                .filter(|(_, (_, tx))| {
                    let sender = derive_address(tx.payload().sender_public_key());
                    trial.nonce(&sender).ok() == Some(tx.payload().nonce())
                })
                .min_by_key(|(_, (id, _))| *id.as_bytes())
                .map(|(index, _)| index);
            let Some(index) = choice else { break };
            let (_, tx) = remaining.swap_remove(index);
            let mut next = trial.clone();
            if next.apply_transaction(tx).is_ok() {
                trial = next;
                selected.push(tx.clone());
            }
        }
        selected
    }

    /// Rebuilds the pool against a canonical state. Stale, conflicting,
    /// non-executable, and no-longer-affordable transactions are discarded.
    pub fn revalidate(&mut self, state: &State) {
        let mut pending: Vec<_> = self.transactions.drain().map(|(_, tx)| tx).collect();
        pending.sort_unstable_by_key(|tx| *transaction_id(tx).as_bytes());
        self.sender_nonces.clear();

        loop {
            let before = pending.len();
            let mut deferred = Vec::new();
            for tx in pending {
                match self.admit(tx.clone(), state) {
                    Ok(_) => {}
                    Err(MempoolError::NonceGap) => deferred.push(tx),
                    Err(_) => {}
                }
            }
            if deferred.is_empty() || deferred.len() == before {
                break;
            }
            pending = deferred;
        }
    }

    fn validate_static(&self, tx: &SignedTransaction) -> Result<(), MempoolError> {
        if tx.payload().chain_id() != self.chain_id {
            return Err(MempoolError::WrongChainId);
        }
        if tx.payload().version() != TransactionVersion::V1 {
            return Err(MempoolError::UnsupportedTransactionVersion);
        }
        verify_transaction_signature(tx).map_err(|_| MempoolError::InvalidSignature)
    }
}

fn sender_nonce_key(sender: Address, nonce: Nonce) -> ([u8; Address::LENGTH], u64) {
    (*sender.as_bytes(), nonce.as_u64())
}
