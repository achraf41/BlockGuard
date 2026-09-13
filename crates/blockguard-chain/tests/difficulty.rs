use blockguard_chain::{Blockchain, ChainError, ChainWork, GenesisConfig, next_chain_work};
use blockguard_consensus::{
    DIFFICULTY_ADJUSTMENT_INTERVAL, adjusted_target, mine_header, target_work,
};
use blockguard_core::{
    Block, BlockHeader, BlockTimestamp, BlockVersion, ChainId, PowNonce, PowTarget,
};
use blockguard_crypto::{block_hash, merkle_root};

fn new_chain() -> Blockchain {
    let config = GenesisConfig::new(
        ChainId::new(1),
        BlockTimestamp::new(0),
        PowTarget::MAX,
        vec![],
    )
    .unwrap();
    Blockchain::from_genesis_config(&config).unwrap()
}

fn empty_block(chain: &Blockchain, parent: &Block, timestamp: u64, target: PowTarget) -> Block {
    let transactions = vec![];
    let state = chain.state_at(&block_hash(parent.header())).unwrap();
    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        parent.header().height().checked_increment().unwrap(),
        block_hash(parent.header()),
        merkle_root(&transactions),
        state.state_root(),
        target,
        BlockTimestamp::new(timestamp),
        PowNonce::ZERO,
    );
    let (header, _) = mine_header(header, &target).unwrap();
    Block::new(header, transactions)
}

fn append_at(chain: &mut Blockchain, parent: &Block, timestamp: u64) -> Block {
    let target = chain
        .next_pow_target_after(block_hash(parent.header()))
        .unwrap();
    let block = empty_block(chain, parent, timestamp, target);
    chain.append_block(block.clone()).unwrap();
    block
}

#[test]
fn non_boundary_keeps_parent_target_and_boundary_retargets() {
    let mut chain = new_chain();
    let mut parent = chain.tip().clone();
    for height in 1..DIFFICULTY_ADJUSTMENT_INTERVAL {
        assert_eq!(
            chain
                .next_pow_target_after(block_hash(parent.header()))
                .unwrap(),
            parent.header().pow_target()
        );
        parent = append_at(&mut chain, &parent, height);
    }
    let expected = adjusted_target(PowTarget::MAX, 9, PowTarget::MAX);
    assert_eq!(
        chain
            .next_pow_target_after(block_hash(parent.header()))
            .unwrap(),
        expected
    );
    let boundary = append_at(&mut chain, &parent, 10);
    assert_eq!(
        boundary.header().height().value(),
        DIFFICULTY_ADJUSTMENT_INTERVAL
    );
    assert_eq!(boundary.header().pow_target(), expected);
}

#[test]
fn unexpected_easy_and_hard_targets_are_rejected() {
    let mut chain = new_chain();
    let parent = chain.tip().clone();
    let hard = PowTarget::from_bytes([0x7f; 32]);
    let block = empty_block(&chain, &parent, 1, hard);
    assert_eq!(
        chain.append_block(block),
        Err(ChainError::UnexpectedPowTarget)
    );

    let mut chain = new_chain();
    let mut parent = chain.tip().clone();
    for height in 1..10 {
        parent = append_at(&mut chain, &parent, height);
    }
    let block = empty_block(&chain, &parent, 10, PowTarget::MAX);
    assert_eq!(
        chain.append_block(block),
        Err(ChainError::UnexpectedPowTarget)
    );
}

#[test]
fn non_increasing_child_timestamp_is_rejected() {
    let mut chain = new_chain();
    let parent = chain.tip().clone();
    let block = empty_block(&chain, &parent, 0, PowTarget::MAX);
    assert_eq!(
        chain.append_block(block),
        Err(ChainError::InvalidBlockTimestamp)
    );
}

#[test]
fn fork_targets_use_branch_ancestry_and_shorter_harder_chain_can_win() {
    let mut chain = new_chain();
    let genesis = chain.tip().clone();
    let mut fast = genesis.clone();
    let mut slow = genesis;
    for height in 1..10 {
        fast = append_at(&mut chain, &fast, height);
    }
    for height in 1..10 {
        slow = append_at(&mut chain, &slow, height * 40);
    }
    let fast_target = chain
        .next_pow_target_after(block_hash(fast.header()))
        .unwrap();
    let slow_target = chain
        .next_pow_target_after(block_hash(slow.header()))
        .unwrap();
    assert!(fast_target.as_bytes() < slow_target.as_bytes());
    assert_eq!(slow_target, PowTarget::MAX);

    let slow10 = append_at(&mut chain, &slow, 400);
    let slow11 = append_at(&mut chain, &slow10, 410);
    let slow12 = append_at(&mut chain, &slow11, 420);
    assert_eq!(chain.canonical_tip_hash(), block_hash(slow12.header()));
    let fast10 = append_at(&mut chain, &fast, 10);
    assert_eq!(chain.canonical_tip_hash(), block_hash(fast10.header()));
    assert!(fast10.header().height() < slow12.header().height());
}

#[test]
fn cumulative_work_overflow_is_rejected() {
    let almost_full = ChainWork::from_bytes([0xff; 32]);
    assert_eq!(
        next_chain_work(almost_full, target_work(PowTarget::MAX)),
        Err(ChainError::ChainWorkOverflow)
    );
}
