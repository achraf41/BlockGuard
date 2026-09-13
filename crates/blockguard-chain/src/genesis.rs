use std::{error::Error, fmt};

use blockguard_core::{
    Address, Amount, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion,
    ChainId, Nonce, PowNonce, PowTarget, StateRoot,
};
use blockguard_crypto::{block_hash, merkle_root};
use blockguard_state::{Account, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisAllocation {
    address: Address,
    balance: Amount,
    nonce: Nonce,
}

impl GenesisAllocation {
    pub const fn new(address: Address, balance: Amount, nonce: Nonce) -> Self {
        Self {
            address,
            balance,
            nonce,
        }
    }

    pub const fn address(&self) -> Address {
        self.address
    }

    pub const fn balance(&self) -> Amount {
        self.balance
    }

    pub const fn nonce(&self) -> Nonce {
        self.nonce
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisConfigError {
    DuplicateAllocation(Address),
}

impl fmt::Display for GenesisConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateAllocation(_) => write!(f, "duplicate genesis allocation"),
        }
    }
}

impl Error for GenesisConfigError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenesisConfig {
    chain_id: ChainId,
    timestamp: BlockTimestamp,
    pow_target: PowTarget,
    allocations: Vec<GenesisAllocation>,
}

impl GenesisConfig {
    pub fn new(
        chain_id: ChainId,
        timestamp: BlockTimestamp,
        pow_target: PowTarget,
        mut allocations: Vec<GenesisAllocation>,
    ) -> Result<Self, GenesisConfigError> {
        allocations.sort_unstable_by_key(|allocation| *allocation.address().as_bytes());
        for pair in allocations.windows(2) {
            if pair[0].address() == pair[1].address() {
                return Err(GenesisConfigError::DuplicateAllocation(pair[0].address()));
            }
        }
        Ok(Self {
            chain_id,
            timestamp,
            pow_target,
            allocations,
        })
    }

    pub const fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub const fn timestamp(&self) -> BlockTimestamp {
        self.timestamp
    }

    pub const fn pow_target(&self) -> PowTarget {
        self.pow_target
    }

    pub fn allocations(&self) -> &[GenesisAllocation] {
        &self.allocations
    }

    pub fn initial_state(&self) -> State {
        let mut state = State::new();
        for allocation in &self.allocations {
            state.set_account(
                allocation.address(),
                Account::new(allocation.balance(), allocation.nonce()),
            );
        }
        state
    }

    pub fn state_root(&self) -> StateRoot {
        self.initial_state().state_root()
    }

    pub fn genesis_block(&self) -> Block {
        let transactions = Vec::new();
        let header = BlockHeader::new(
            BlockVersion::V1,
            self.chain_id,
            BlockHeight::ZERO,
            BlockHash::ZERO,
            merkle_root(&transactions),
            self.state_root(),
            self.timestamp,
            PowNonce::ZERO,
        );
        Block::new(header, transactions)
    }

    pub fn genesis_hash(&self) -> BlockHash {
        block_hash(self.genesis_block().header())
    }
}
