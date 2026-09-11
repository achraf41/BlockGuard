use blockguard_core::{
    Block, BlockHash, BlockHeight, BlockVersion, ChainId, PowTarget, TransactionVersion,
};

use blockguard_crypto::{block_hash, merkle_root};

use crate::ChainError;
use blockguard_consensus::validate_pow;
use blockguard_state::State;

#[derive(Debug, Clone)]
pub struct Blockchain {
    chain_id: ChainId,
    pow_target: PowTarget,
    blocks: Vec<Block>,
    state: State,
}

impl Blockchain {
    pub fn new(
        chain_id: ChainId,
        pow_target: PowTarget,
        genesis: Block,
        initia_state: State,
    ) -> Result<Self, ChainError> {
        if genesis.header().version() != BlockVersion::V1 {
            return Err(ChainError::UnsupportedBlockVersion);
        }

        if genesis.header().chain_id() != chain_id {
            return Err(ChainError::WrongChainId);
        }

        if genesis.header().height() != BlockHeight::ZERO {
            return Err(ChainError::InvalidGenesisHeight);
        }

        if genesis.header().previous_block_hash() != BlockHash::ZERO {
            return Err(ChainError::InvalidGenesisParent);
        }

        if !genesis.transactions().is_empty() {
            return Err(ChainError::GenesisTransactionsNotAllowed);
        }
        let expected_root = merkle_root(genesis.transactions());

        if genesis.header().transaction_root() != expected_root {
            return Err(ChainError::InvalidMerkleRoot);
        }

        Ok(Self {
            chain_id,
            pow_target,
            blocks: vec![genesis],
            state: initia_state,
        })
    }

    pub const fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub const fn pow_target(&self) -> PowTarget {
        self.pow_target
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn tip(&self) -> &Block {
        self.blocks
            .last()
            .expect("blockchain always contains genesis")
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    fn validate_candidate(&self, block: &Block) -> Result<(), ChainError> {
        if block.header().version() != BlockVersion::V1 {
            return Err(ChainError::UnsupportedBlockVersion);
        }

        if block.header().chain_id() != self.chain_id {
            return Err(ChainError::WrongChainId);
        }

        let expected_height = self
            .tip()
            .header()
            .height()
            .checked_increment()
            .ok_or(ChainError::HeightOverflow)?;

        if block.header().height() != expected_height {
            return Err(ChainError::InvalidHeight);
        }

        let expected_previous_hash = block_hash(self.tip().header());

        if block.header().previous_block_hash() != expected_previous_hash {
            return Err(ChainError::InvalidPreviousBlockHash);
        }

        let expected_merkle_root = merkle_root(block.transactions());

        if block.header().transaction_root() != expected_merkle_root {
            return Err(ChainError::InvalidMerkleRoot);
        }

        for tx in block.transactions() {
            if tx.payload().chain_id() != self.chain_id {
                return Err(ChainError::WrongChainId);
            }

            if tx.payload().version() != TransactionVersion::V1 {
                return Err(ChainError::UnsupportedTransactionVersion);
            }
        }

        if !validate_pow(block.header(), &self.pow_target) {
            return Err(ChainError::InvalidProofOfWork);
        }

        Ok(())
    }

    pub fn append_block(&mut self, block: Block) -> Result<BlockHash, ChainError> {
        self.validate_candidate(&block)?;

        let mut next_state = self.state.clone();

        for tx in block.transactions() {
            next_state.apply_transaction(tx)?;
        }

        let hash = block_hash(block.header());

        self.state = next_state;

        self.blocks.push(block);

        Ok(hash)
    }
}
