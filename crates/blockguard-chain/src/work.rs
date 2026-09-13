use crate::ChainError;
use blockguard_consensus::PowWork;
use primitive_types::U256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainWork([u8; 32]);

impl ChainWork {
    pub const ZERO: Self = Self([0; 32]);
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    pub fn from_u64(value: u64) -> Self {
        Self(U256::from(value).to_big_endian())
    }
    pub fn checked_add_work(self, rhs: PowWork) -> Option<Self> {
        U256::from_big_endian(&self.0)
            .checked_add(U256::from_big_endian(rhs.as_bytes()))
            .map(|value| Self(value.to_big_endian()))
    }
}

pub fn next_chain_work(parent: ChainWork, block_work: PowWork) -> Result<ChainWork, ChainError> {
    parent
        .checked_add_work(block_work)
        .ok_or(ChainError::ChainWorkOverflow)
}
