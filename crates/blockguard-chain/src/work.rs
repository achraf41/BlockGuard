use crate::ChainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainWork(u64);

impl ChainWork {
    pub const ZERO: Self = Self(0);

    pub const ONE: Self = Self(1);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

pub fn next_chain_work(parent_work: ChainWork) -> Result<ChainWork, ChainError> {
    parent_work
        .checked_add(ChainWork::ONE)
        .ok_or(ChainError::ChainWorkOverflow)
}
