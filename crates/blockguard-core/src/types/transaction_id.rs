use super::Hash256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(Hash256);

impl TransactionId {
    pub const fn from_hash(hash: Hash256) -> Self {
        Self(hash)
    }

    pub const fn as_hash(&self) -> &Hash256 {
        &self.0
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}
