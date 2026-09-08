use crate::transaction::SignedTransaction;

use super::BlockHeader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    header: BlockHeader,
    transactions: Vec<SignedTransaction>,
}

impl Block {
    pub fn new(header: BlockHeader, transactions: Vec<SignedTransaction>) -> Self {
        Self {
            header,
            transactions,
        }
    }

    pub const fn header(&self) -> &BlockHeader {
        &self.header
    }

    pub fn transactions(&self) -> &[SignedTransaction] {
        &self.transactions
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}
