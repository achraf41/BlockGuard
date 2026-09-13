use std::collections::HashMap;

use blockguard_core::{
    Block, BlockHash, BlockHeight, BlockVersion, ChainId, PowTarget, TransactionVersion,
};

use blockguard_crypto::{block_hash, merkle_root};

use crate::{
    BlockMetadata, ChainError, ChainWork, GenesisConfig, candidate_is_better, next_chain_work,
};
use blockguard_consensus::{
    DIFFICULTY_ADJUSTMENT_INTERVAL, adjusted_target, target_work, validate_pow,
};
use blockguard_state::State;

#[derive(Debug, Clone)]
struct IndexedBlock {
    block: Block,
    metadata: BlockMetadata,
    state: State,
}

#[derive(Debug, Clone)]
pub struct Blockchain {
    chain_id: ChainId,
    pow_target: PowTarget,
    blocks: Vec<Block>,
    block_index: HashMap<BlockHash, IndexedBlock>,
    canonical_tip: BlockHash,
    state: State,
}

impl Blockchain {
    pub fn from_genesis_config(config: &GenesisConfig) -> Result<Self, ChainError> {
        Self::new(
            config.chain_id(),
            config.pow_target(),
            config.genesis_block(),
            config.initial_state(),
        )
    }

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

        if genesis.header().state_root() != initia_state.state_root() {
            return Err(ChainError::InvalidStateRoot);
        }

        if genesis.header().pow_target() != pow_target {
            return Err(ChainError::UnexpectedPowTarget);
        }

        let genesis_hash = block_hash(genesis.header());
        let genesis_metadata =
            BlockMetadata::new(BlockHash::ZERO, BlockHeight::ZERO, ChainWork::ZERO);
        let mut block_index = HashMap::new();

        block_index.insert(
            genesis_hash,
            IndexedBlock {
                block: genesis.clone(),
                metadata: genesis_metadata,
                state: initia_state.clone(),
            },
        );

        Ok(Self {
            chain_id,
            pow_target,
            blocks: vec![genesis],
            block_index,
            canonical_tip: genesis_hash,
            state: initia_state,
        })
    }

    pub const fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub const fn pow_target(&self) -> PowTarget {
        self.pow_target
    }

    pub fn next_pow_target(&self) -> Result<PowTarget, ChainError> {
        self.next_pow_target_after(self.canonical_tip)
    }

    pub fn next_pow_target_after(&self, parent_hash: BlockHash) -> Result<PowTarget, ChainError> {
        let parent = self
            .block_index
            .get(&parent_hash)
            .ok_or(ChainError::InvalidPreviousBlockHash)?;
        let next_height = parent
            .metadata
            .height()
            .checked_increment()
            .ok_or(ChainError::HeightOverflow)?;
        if next_height.value() % DIFFICULTY_ADJUSTMENT_INTERVAL != 0 {
            return Ok(parent.block.header().pow_target());
        }
        let ancestor_height = next_height.value() - DIFFICULTY_ADJUSTMENT_INTERVAL;
        let mut ancestor = parent;
        while ancestor.metadata.height().value() > ancestor_height {
            ancestor = self
                .block_index
                .get(&ancestor.metadata.parent())
                .expect("indexed ancestry is complete");
        }
        let actual = parent
            .block
            .header()
            .timestamp()
            .value()
            .saturating_sub(ancestor.block.header().timestamp().value());
        Ok(adjusted_target(
            parent.block.header().pow_target(),
            actual,
            self.pow_target,
        ))
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn tip(&self) -> &Block {
        &self
            .block_index
            .get(&self.canonical_tip)
            .expect("canonical tip is always indexed")
            .block
    }

    pub const fn canonical_tip_hash(&self) -> BlockHash {
        self.canonical_tip
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn indexed_block_count(&self) -> usize {
        self.block_index.len()
    }

    pub fn block(&self, hash: &BlockHash) -> Option<&Block> {
        self.block_index.get(hash).map(|entry| &entry.block)
    }

    pub fn metadata(&self, hash: &BlockHash) -> Option<&BlockMetadata> {
        self.block_index.get(hash).map(|entry| &entry.metadata)
    }

    pub fn state_at(&self, hash: &BlockHash) -> Option<&State> {
        self.block_index.get(hash).map(|entry| &entry.state)
    }

    pub fn indexed_blocks(
        &self,
    ) -> impl Iterator<Item = (BlockHash, &Block, &BlockMetadata, &State)> {
        self.block_index
            .iter()
            .map(|(hash, entry)| (*hash, &entry.block, &entry.metadata, &entry.state))
    }

    fn validate_candidate(
        &self,
        block: &Block,
        parent_hash: BlockHash,
        parent: &IndexedBlock,
    ) -> Result<(), ChainError> {
        if block.header().version() != BlockVersion::V1 {
            return Err(ChainError::UnsupportedBlockVersion);
        }

        if block.header().chain_id() != self.chain_id {
            return Err(ChainError::WrongChainId);
        }

        let expected_height = parent
            .metadata
            .height()
            .checked_increment()
            .ok_or(ChainError::HeightOverflow)?;

        if block.header().height() != expected_height {
            return Err(ChainError::InvalidHeight);
        }

        if block.header().timestamp() <= parent.block.header().timestamp() {
            return Err(ChainError::InvalidBlockTimestamp);
        }
        if block.header().pow_target() != self.next_pow_target_after(parent_hash)? {
            return Err(ChainError::UnexpectedPowTarget);
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

        if !validate_pow(block.header()) {
            return Err(ChainError::InvalidProofOfWork);
        }

        Ok(())
    }

    pub fn append_block(&mut self, block: Block) -> Result<BlockHash, ChainError> {
        let parent_hash = block.header().previous_block_hash();
        let parent = self
            .block_index
            .get(&parent_hash)
            .ok_or(ChainError::InvalidPreviousBlockHash)?;
        let parent_metadata = parent.metadata;
        let mut next_state = parent.state.clone();

        self.validate_candidate(&block, parent_hash, parent)?;

        let cumulative_work = next_chain_work(
            parent_metadata.cumulative_work(),
            target_work(block.header().pow_target()),
        )?;

        for tx in block.transactions() {
            next_state.apply_transaction(tx)?;
        }

        if block.header().state_root() != next_state.state_root() {
            return Err(ChainError::InvalidStateRoot);
        }

        let hash = block_hash(block.header());
        let metadata = BlockMetadata::new(parent_hash, block.header().height(), cumulative_work);
        let current_tip_metadata = self
            .block_index
            .get(&self.canonical_tip)
            .expect("canonical tip is always indexed")
            .metadata;
        let becomes_canonical = candidate_is_better(&current_tip_metadata, &metadata);

        self.block_index.insert(
            hash,
            IndexedBlock {
                block,
                metadata,
                state: next_state.clone(),
            },
        );

        if becomes_canonical {
            self.canonical_tip = hash;
            self.state = next_state;
            self.rebuild_canonical_blocks();
        }

        Ok(hash)
    }

    fn rebuild_canonical_blocks(&mut self) {
        let mut hashes = Vec::new();
        let mut hash = self.canonical_tip;

        loop {
            hashes.push(hash);
            let entry = self
                .block_index
                .get(&hash)
                .expect("canonical ancestors are always indexed");

            if entry.metadata.height() == BlockHeight::ZERO {
                break;
            }

            hash = entry.metadata.parent();
        }

        hashes.reverse();
        self.blocks = hashes
            .into_iter()
            .map(|hash| {
                self.block_index
                    .get(&hash)
                    .expect("canonical blocks are always indexed")
                    .block
                    .clone()
            })
            .collect();
    }
}
