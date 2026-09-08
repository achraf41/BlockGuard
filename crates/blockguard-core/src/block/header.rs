use crate::types::{BlockHash, BlockHeight, BlockTimestamp, BlockVersion, ChainId, MerkleRoot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    version: BlockVersion,
    chain_id: ChainId,
    height: BlockHeight,
    previous_block_hash: BlockHash,
    transaction_root: MerkleRoot,
    timestamp: BlockTimestamp,
}

impl BlockHeader {
    pub fn new(
        version: BlockVersion,
        chain_id: ChainId,
        height: BlockHeight,
        previous_block_hash: BlockHash,
        transaction_root: MerkleRoot,
        timestamp: BlockTimestamp,
    ) -> Self {
        Self {
            version,
            chain_id,
            height,
            previous_block_hash,
            transaction_root,
            timestamp,
        }
    }

    pub const fn version(&self) -> BlockVersion {
        self.version
    }

    pub const fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub const fn height(&self) -> BlockHeight {
        self.height
    }

    pub const fn previous_block_hash(&self) -> BlockHash {
        self.previous_block_hash
    }

    pub const fn transaction_root(&self) -> MerkleRoot {
        self.transaction_root
    }

    pub const fn timestamp(&self) -> BlockTimestamp {
        self.timestamp
    }
}
