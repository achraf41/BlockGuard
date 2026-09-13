use blockguard_chain::{GenesisAllocation, GenesisConfig};
use blockguard_consensus::{mine_header, validate_pow};
use blockguard_core::{
    Address, Amount, Block, BlockHeader, BlockTimestamp, BlockVersion, ChainId, Nonce, PowNonce,
    PowTarget, SignedTransaction, TransactionVersion, UnsignedTransaction,
};
use blockguard_crypto::{
    KeyPair, block_hash, derive_address, merkle_root, sign_transaction, transaction_id,
};
use blockguard_node::Node;
use blockguard_state::State;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn key(byte: u8) -> KeyPair {
    KeyPair::from_private_key_bytes([byte; 32]).unwrap()
}
fn address(key: &KeyPair) -> Address {
    derive_address(&key.public_key())
}
fn transfer(sender: &KeyPair, recipient: Address, amount: u64, nonce: u64) -> SignedTransaction {
    sign_transaction(
        sender,
        UnsignedTransaction::new(
            TransactionVersion::V1,
            ChainId::new(1),
            sender.public_key(),
            recipient,
            Amount::new(amount),
            Nonce::new(nonce),
        ),
    )
    .unwrap()
}
fn config() -> GenesisConfig {
    GenesisConfig::new(
        ChainId::new(1),
        BlockTimestamp::new(100),
        PowTarget::MAX,
        vec![
            GenesisAllocation::new(address(&key(1)), Amount::new(100), Nonce::ZERO),
            GenesisAllocation::new(address(&key(2)), Amount::new(100), Nonce::ZERO),
            GenesisAllocation::new(address(&key(3)), Amount::ZERO, Nonce::ZERO),
        ],
    )
    .unwrap()
}
fn external(
    parent: &Block,
    state: &State,
    transactions: Vec<SignedTransaction>,
    timestamp: u64,
) -> Block {
    let mut next = state.clone();
    for tx in &transactions {
        next.apply_transaction(tx).unwrap();
    }
    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        parent.header().height().checked_increment().unwrap(),
        block_hash(parent.header()),
        merkle_root(&transactions),
        next.state_root(),
        parent.header().pow_target(),
        BlockTimestamp::new(timestamp),
        PowNonce::ZERO,
    );
    let (header, _) = mine_header(header, &PowTarget::MAX).unwrap();
    Block::new(header, transactions)
}
fn path() -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("blockguard-node-{}-{id}.dat", std::process::id()))
}

#[test]
fn submit_transaction_adds_to_mempool() {
    let mut node = Node::new(config(), None).unwrap();
    let tx = transfer(&key(1), address(&key(3)), 10, 0);
    let id = node.submit_transaction(tx).unwrap();
    assert!(node.mempool().contains(&id));
}

#[test]
fn produced_block_has_valid_commitments_and_pow_and_updates_state() {
    let mut node = Node::new(config(), None).unwrap();
    node.submit_transaction(transfer(&key(1), address(&key(3)), 10, 0))
        .unwrap();
    let block = node.produce_block(BlockTimestamp::new(101), 10).unwrap();
    assert!(validate_pow(block.header()));
    assert_eq!(
        block.header().transaction_root(),
        merkle_root(block.transactions())
    );
    assert_eq!(
        block.header().state_root(),
        node.blockchain().state().state_root()
    );
    assert_eq!(
        node.blockchain()
            .state()
            .balance(&address(&key(1)))
            .unwrap(),
        Amount::new(90)
    );
    assert_eq!(
        node.blockchain().state().nonce(&address(&key(1))).unwrap(),
        Nonce::new(1)
    );
    assert_eq!(
        node.blockchain()
            .state()
            .balance(&address(&key(3)))
            .unwrap(),
        Amount::new(10)
    );
    assert!(node.mempool().is_empty());
}

#[test]
fn maximum_transaction_count_is_respected() {
    let mut node = Node::new(config(), None).unwrap();
    node.submit_transaction(transfer(&key(1), address(&key(3)), 10, 0))
        .unwrap();
    node.submit_transaction(transfer(&key(2), address(&key(3)), 10, 0))
        .unwrap();
    assert_eq!(
        node.produce_block(BlockTimestamp::new(101), 1)
            .unwrap()
            .transaction_count(),
        1
    );
    assert_eq!(node.mempool().len(), 1);
}

#[test]
fn external_canonical_block_revalidates_mempool() {
    let mut node = Node::new(config(), None).unwrap();
    let confirmed = transfer(&key(1), address(&key(3)), 10, 0);
    let id = node.submit_transaction(confirmed.clone()).unwrap();
    let block = external(
        node.blockchain().tip(),
        node.blockchain().state(),
        vec![confirmed],
        101,
    );
    node.accept_block(block).unwrap();
    assert!(!node.mempool().contains(&id));
}

#[test]
fn node_save_load_can_continue_producing() {
    let file = path();
    let mut node = Node::new(config(), Some(file.clone())).unwrap();
    node.submit_transaction(transfer(&key(1), address(&key(3)), 10, 0))
        .unwrap();
    node.produce_block(BlockTimestamp::new(101), 10).unwrap();
    let mut loaded = Node::load(&file).unwrap();
    loaded
        .submit_transaction(transfer(&key(1), address(&key(3)), 5, 1))
        .unwrap();
    loaded.produce_block(BlockTimestamp::new(102), 10).unwrap();
    assert_eq!(
        loaded
            .blockchain()
            .state()
            .balance(&address(&key(1)))
            .unwrap(),
        Amount::new(85)
    );
    assert_eq!(loaded.blockchain().block_count(), 3);
    fs::remove_file(file).unwrap();
}

#[test]
fn equal_work_side_block_does_not_change_canonical_mempool_assumptions() {
    let mut node = Node::new(config(), None).unwrap();
    let genesis = node.blockchain().tip().clone();
    let genesis_state = node.blockchain().state().clone();
    let canonical_tx = transfer(&key(1), address(&key(3)), 10, 0);
    node.accept_block(external(&genesis, &genesis_state, vec![canonical_tx], 101))
        .unwrap();
    let queued = transfer(&key(1), address(&key(3)), 5, 1);
    let queued_id = node.submit_transaction(queued).unwrap();
    let side_tx = transfer(&key(1), address(&key(2)), 20, 0);
    node.accept_block(external(&genesis, &genesis_state, vec![side_tx], 102))
        .unwrap();
    assert!(node.mempool().contains(&queued_id));
    assert_eq!(
        node.blockchain().state().nonce(&address(&key(1))).unwrap(),
        Nonce::new(1)
    );
}

#[test]
fn canonical_reorg_revalidates_pool_and_readmits_valid_orphans() {
    let mut node = Node::new(config(), None).unwrap();
    let genesis = node.blockchain().tip().clone();
    let genesis_state = node.blockchain().state().clone();
    let orphan = transfer(&key(1), address(&key(3)), 10, 0);
    let orphan_id = transaction_id(&orphan);
    node.accept_block(external(&genesis, &genesis_state, vec![orphan], 101))
        .unwrap();
    let side_one = external(&genesis, &genesis_state, vec![], 102);
    let side_state = genesis_state.clone();
    let side_two = external(&side_one, &side_state, vec![], 103);
    node.accept_block(side_one).unwrap();
    node.accept_block(side_two).unwrap();
    assert_eq!(node.blockchain().block_count(), 3);
    assert!(node.mempool().contains(&orphan_id));
}

#[test]
fn canonical_reorg_drops_transactions_made_stale_by_winning_branch() {
    let mut node = Node::new(config(), None).unwrap();
    let genesis = node.blockchain().tip().clone();
    let genesis_state = node.blockchain().state().clone();
    let canonical_zero = transfer(&key(1), address(&key(3)), 10, 0);
    node.accept_block(external(
        &genesis,
        &genesis_state,
        vec![canonical_zero],
        101,
    ))
    .unwrap();
    let queued_one = transfer(&key(1), address(&key(3)), 5, 1);
    let queued_id = node.submit_transaction(queued_one).unwrap();

    let side_zero = transfer(&key(1), address(&key(2)), 1, 0);
    let side_one = external(&genesis, &genesis_state, vec![side_zero], 102);
    let mut side_state = genesis_state.clone();
    side_state
        .apply_transaction(&side_one.transactions()[0])
        .unwrap();
    let side_nonce_one = transfer(&key(1), address(&key(2)), 1, 1);
    let side_two = external(&side_one, &side_state, vec![side_nonce_one], 103);
    node.accept_block(side_one).unwrap();
    node.accept_block(side_two).unwrap();

    assert_eq!(
        node.blockchain().state().nonce(&address(&key(1))).unwrap(),
        Nonce::new(2)
    );
    assert!(!node.mempool().contains(&queued_id));
}

#[test]
fn produced_blocks_follow_adjusted_chain_target() {
    let mut node = Node::new(config(), None).unwrap();
    for timestamp in 101..=110 {
        let expected = node.blockchain().next_pow_target().unwrap();
        let block = node
            .produce_block(BlockTimestamp::new(timestamp), 0)
            .unwrap();
        assert_eq!(block.header().pow_target(), expected);
        assert!(validate_pow(block.header()));
    }
    assert_ne!(
        node.blockchain().tip().header().pow_target(),
        PowTarget::MAX
    );
}
