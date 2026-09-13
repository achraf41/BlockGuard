use std::{
    collections::HashSet,
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use blockguard_chain::{Blockchain, ChainError, GenesisConfig};
use blockguard_consensus::{PowError, mine_header};
use blockguard_core::{
    Block, BlockHash, BlockHeader, BlockTimestamp, BlockVersion, PowNonce, SignedTransaction,
    TransactionId,
};
use blockguard_crypto::{merkle_root, transaction_id};
use blockguard_mempool::{Mempool, MempoolError};
use blockguard_storage::StorageError;

mod network;
pub use network::{NetworkNode, NetworkNodeError};

#[derive(Debug)]
pub enum NodeError {
    Chain(ChainError),
    Mempool(MempoolError),
    Mining(PowError),
    Storage(StorageError),
    StorageNotConfigured,
    HeightOverflow,
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chain(e) => write!(f, "chain rejected block: {e}"),
            Self::Mempool(e) => write!(f, "mempool rejected transaction: {e}"),
            Self::Mining(e) => write!(f, "mining failed: {e}"),
            Self::Storage(e) => write!(f, "persistence failed: {e}"),
            Self::StorageNotConfigured => write!(f, "storage is not configured"),
            Self::HeightOverflow => write!(f, "block height overflow"),
        }
    }
}
impl Error for NodeError {}
impl From<ChainError> for NodeError {
    fn from(e: ChainError) -> Self {
        Self::Chain(e)
    }
}
impl From<MempoolError> for NodeError {
    fn from(e: MempoolError) -> Self {
        Self::Mempool(e)
    }
}
impl From<PowError> for NodeError {
    fn from(e: PowError) -> Self {
        Self::Mining(e)
    }
}
impl From<StorageError> for NodeError {
    fn from(e: StorageError) -> Self {
        Self::Storage(e)
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    genesis_config: GenesisConfig,
    blockchain: Blockchain,
    mempool: Mempool,
    storage_path: Option<PathBuf>,
}

impl Node {
    pub fn new(
        genesis_config: GenesisConfig,
        storage_path: Option<PathBuf>,
    ) -> Result<Self, NodeError> {
        let blockchain = Blockchain::from_genesis_config(&genesis_config)?;
        let mempool = Mempool::new(genesis_config.chain_id());
        Ok(Self {
            genesis_config,
            blockchain,
            mempool,
            storage_path,
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, NodeError> {
        let path = path.as_ref().to_path_buf();
        let (genesis_config, blockchain) = blockguard_storage::load(&path)?.into_parts();
        let mempool = Mempool::new(genesis_config.chain_id());
        Ok(Self {
            genesis_config,
            blockchain,
            mempool,
            storage_path: Some(path),
        })
    }

    pub fn genesis_config(&self) -> &GenesisConfig {
        &self.genesis_config
    }
    pub fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }
    pub fn mempool(&self) -> &Mempool {
        &self.mempool
    }

    pub fn submit_transaction(
        &mut self,
        tx: SignedTransaction,
    ) -> Result<TransactionId, NodeError> {
        Ok(self.mempool.admit(tx, self.blockchain.state())?)
    }

    pub fn produce_block(
        &mut self,
        timestamp: BlockTimestamp,
        max_transactions: usize,
    ) -> Result<Block, NodeError> {
        let transactions = self
            .mempool
            .candidates(self.blockchain.state(), max_transactions);
        let mut next_state = self.blockchain.state().clone();
        for tx in &transactions {
            next_state
                .apply_transaction(tx)
                .expect("mempool candidates were executed during selection");
        }
        let header = BlockHeader::new(
            BlockVersion::V1,
            self.blockchain.chain_id(),
            self.blockchain
                .tip()
                .header()
                .height()
                .checked_increment()
                .ok_or(NodeError::HeightOverflow)?,
            self.blockchain.canonical_tip_hash(),
            merkle_root(&transactions),
            next_state.state_root(),
            timestamp,
            PowNonce::ZERO,
        );
        let (header, _) = mine_header(header, &self.blockchain.pow_target())?;
        let block = Block::new(header, transactions);
        self.accept_block(block.clone())?;
        Ok(block)
    }

    pub fn accept_block(&mut self, block: Block) -> Result<BlockHash, NodeError> {
        let old_tip = self.blockchain.canonical_tip_hash();
        let old_chain = self.blockchain.blocks().to_vec();
        let hash = self.blockchain.append_block(block)?;
        if self.blockchain.canonical_tip_hash() != old_tip {
            self.revalidate_after_canonical_change(old_chain);
        }
        if self.storage_path.is_some() {
            self.save()?;
        }
        Ok(hash)
    }

    pub fn save(&self) -> Result<(), NodeError> {
        let path = self
            .storage_path
            .as_ref()
            .ok_or(NodeError::StorageNotConfigured)?;
        blockguard_storage::save(path, &self.genesis_config, &self.blockchain)?;
        Ok(())
    }

    fn revalidate_after_canonical_change(&mut self, old_chain: Vec<Block>) {
        let confirmed: HashSet<_> = self
            .blockchain
            .blocks()
            .iter()
            .flat_map(|b| b.transactions())
            .map(transaction_id)
            .collect();
        let mut orphaned: Vec<_> = old_chain
            .iter()
            .flat_map(|b| b.transactions())
            .filter(|tx| !confirmed.contains(&transaction_id(tx)))
            .cloned()
            .collect();
        orphaned.sort_unstable_by_key(|tx| *transaction_id(tx).as_bytes());
        self.mempool.revalidate(self.blockchain.state());
        loop {
            let before = orphaned.len();
            let mut deferred = Vec::new();
            for tx in orphaned {
                match self.mempool.admit(tx.clone(), self.blockchain.state()) {
                    Ok(_) => {}
                    Err(MempoolError::NonceGap) => deferred.push(tx),
                    Err(_) => {}
                }
            }
            if deferred.is_empty() || deferred.len() == before {
                break;
            }
            orphaned = deferred;
        }
    }
}
