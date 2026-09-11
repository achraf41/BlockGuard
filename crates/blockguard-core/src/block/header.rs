use crate::types::{
    BlockHash, BlockHeight, BlockTimestamp, BlockVersion, ChainId, MerkleRoot, PowNonce,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    version: BlockVersion,
    chain_id: ChainId,
    height: BlockHeight,
    previous_block_hash: BlockHash,
    transaction_root: MerkleRoot,
    timestamp: BlockTimestamp,
    pow_nonce: PowNonce,
}

impl BlockHeader {
    pub fn new(
        version: BlockVersion,
        chain_id: ChainId,
        height: BlockHeight,
        previous_block_hash: BlockHash,
        transaction_root: MerkleRoot,
        timestamp: BlockTimestamp,
        pow_nonce: PowNonce,
    ) -> Self {
        Self {
            version,
            chain_id,
            height,
            previous_block_hash,
            transaction_root,
            timestamp,
            pow_nonce,
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

    pub const fn pow_nonce(&self) -> PowNonce {
        self.pow_nonce
    }

    pub fn set_pow_nonce(&mut self, pow_nonce: PowNonce) {
        self.pow_nonce = pow_nonce
    }
}
