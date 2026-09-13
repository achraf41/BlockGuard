use blockguard_core::{BlockHash, BlockHeight};

use crate::ChainWork;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockMetadata {
    parent: BlockHash,
    height: BlockHeight,
    cumulative_work: ChainWork,
}

impl BlockMetadata {
    pub const fn new(parent: BlockHash, height: BlockHeight, cumulative_work: ChainWork) -> Self {
        Self {
            parent,
            height,
            cumulative_work,
        }
    }

    pub const fn parent(&self) -> BlockHash {
        self.parent
    }

    pub const fn height(&self) -> BlockHeight {
        self.height
    }

    pub const fn cumulative_work(&self) -> ChainWork {
        self.cumulative_work
    }
}

pub fn candidate_is_better(current: &BlockMetadata, candidate: &BlockMetadata) -> bool {
    candidate.cumulative_work() > current.cumulative_work()
}
