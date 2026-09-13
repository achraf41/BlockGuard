use blockguard_chain::{BlockMetadata, ChainWork, candidate_is_better, next_chain_work};

use blockguard_core::{BlockHash, BlockHeight};

#[test]
fn chain_work_accumulates_per_block() {
    let genesis_work = ChainWork::ZERO;

    let block_a_work = next_chain_work(genesis_work).unwrap();

    let block_b_work = next_chain_work(block_a_work).unwrap();

    assert_eq!(block_a_work, ChainWork::new(1),);

    assert_eq!(block_b_work, ChainWork::new(2),);
}

#[test]
fn branch_with_more_work_is_preferred() {
    let current = BlockMetadata::new(BlockHash::ZERO, BlockHeight::new(2), ChainWork::new(2));

    let candidate = BlockMetadata::new(BlockHash::ZERO, BlockHeight::new(3), ChainWork::new(3));

    assert!(candidate_is_better(&current, &candidate,));
}

#[test]
fn equal_work_does_not_replace_current_tip() {
    let current = BlockMetadata::new(BlockHash::ZERO, BlockHeight::new(2), ChainWork::new(2));

    let candidate = BlockMetadata::new(BlockHash::ZERO, BlockHeight::new(2), ChainWork::new(2));

    assert!(!candidate_is_better(&current, &candidate,));
}
