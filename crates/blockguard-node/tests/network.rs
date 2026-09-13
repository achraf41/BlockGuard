use blockguard_chain::{GenesisAllocation, GenesisConfig};
use blockguard_consensus::mine_header;
use blockguard_core::{
    Address, Amount, Block, BlockHeader, BlockTimestamp, BlockVersion, ChainId, Nonce, PowNonce,
    PowTarget, SignatureBytes, SignedTransaction, StateRoot, TransactionVersion,
    UnsignedTransaction,
};
use blockguard_crypto::{
    KeyPair, block_hash, derive_address, merkle_root, sign_transaction, transaction_id,
};
use blockguard_network::Message;
use blockguard_node::{NetworkNode, Node};
use blockguard_state::State;
use std::{
    net::SocketAddr,
    thread,
    time::{Duration, Instant},
};

fn key(byte: u8) -> KeyPair {
    KeyPair::from_private_key_bytes([byte; 32]).unwrap()
}
fn address(k: &KeyPair) -> Address {
    derive_address(&k.public_key())
}
fn config() -> GenesisConfig {
    GenesisConfig::new(
        ChainId::new(1),
        BlockTimestamp::new(100),
        PowTarget::MAX,
        vec![
            GenesisAllocation::new(address(&key(1)), Amount::new(100), Nonce::ZERO),
            GenesisAllocation::new(address(&key(2)), Amount::ZERO, Nonce::ZERO),
        ],
    )
    .unwrap()
}
fn tx(amount: u64, nonce: u64) -> SignedTransaction {
    sign_transaction(
        &key(1),
        UnsignedTransaction::new(
            TransactionVersion::V1,
            ChainId::new(1),
            key(1).public_key(),
            address(&key(2)),
            Amount::new(amount),
            Nonce::new(nonce),
        ),
    )
    .unwrap()
}
fn bind(id: u8) -> NetworkNode {
    NetworkNode::bind(
        Node::new(config(), None).unwrap(),
        "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
        [id; 16],
    )
    .unwrap()
}
fn wait_for(mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while Instant::now() < end {
        if condition() {
            return;
        }
        thread::sleep(Duration::from_millis(10))
    }
    panic!("condition not reached before timeout")
}
fn external(
    parent: &Block,
    state: &State,
    transactions: Vec<SignedTransaction>,
    timestamp: u64,
) -> Block {
    let mut next = state.clone();
    for tx in &transactions {
        next.apply_transaction(tx).unwrap()
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

#[test]
fn transaction_and_block_propagate_and_are_independently_validated() {
    let a = bind(1);
    let b = bind(2);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| a.peer_count() == 1 && b.peer_count() == 1);
    let transaction = tx(10, 0);
    let id = a.submit_transaction(transaction).unwrap();
    wait_for(|| b.snapshot().unwrap().mempool().contains(&id));
    let block = a.produce_block(BlockTimestamp::new(101), 10).unwrap();
    wait_for(|| {
        b.snapshot().unwrap().blockchain().canonical_tip_hash() == block_hash(block.header())
    });
    let aa = a.snapshot().unwrap();
    let bb = b.snapshot().unwrap();
    assert_eq!(
        aa.blockchain().canonical_tip_hash(),
        bb.blockchain().canonical_tip_hash()
    );
    assert_eq!(
        aa.blockchain().state().state_root(),
        bb.blockchain().state().state_root()
    );
    assert_eq!(
        bb.blockchain().state().balance(&address(&key(2))).unwrap(),
        Amount::new(10)
    );
    assert_eq!(
        bb.blockchain().state().nonce(&address(&key(1))).unwrap(),
        Nonce::new(1)
    );
}

#[test]
fn duplicate_propagation_does_not_duplicate_processing_or_loop() {
    let a = bind(3);
    let b = bind(4);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| a.peer_count() == 1 && b.peer_count() == 1);
    let transaction = tx(1, 0);
    let id = a.submit_transaction(transaction.clone()).unwrap();
    wait_for(|| b.snapshot().unwrap().mempool().contains(&id));
    a.send_to_all(&Message::Transaction(transaction));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(a.snapshot().unwrap().mempool().len(), 1);
    assert_eq!(b.snapshot().unwrap().mempool().len(), 1);
    let block = a.produce_block(BlockTimestamp::new(101), 10).unwrap();
    wait_for(|| b.snapshot().unwrap().blockchain().block_count() == 2);
    a.send_to_all(&Message::Block(block));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(a.snapshot().unwrap().blockchain().indexed_block_count(), 2);
    assert_eq!(b.snapshot().unwrap().blockchain().indexed_block_count(), 2);
}

#[test]
fn invalid_peer_transaction_is_rejected() {
    let a = bind(5);
    let b = bind(6);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| a.peer_count() == 1 && b.peer_count() == 1);
    let valid = tx(1, 0);
    let invalid =
        SignedTransaction::new(valid.payload().clone(), SignatureBytes::from_bytes([0; 64]));
    let id = transaction_id(&invalid);
    a.send_to_all(&Message::Transaction(invalid));
    thread::sleep(Duration::from_millis(100));
    assert!(!b.snapshot().unwrap().mempool().contains(&id));
}

#[test]
fn invalid_peer_block_leaves_chain_unchanged() {
    let a = bind(7);
    let b = bind(8);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| a.peer_count() == 1 && b.peer_count() == 1);
    let before = b.snapshot().unwrap().blockchain().canonical_tip_hash();
    let parent = a.snapshot().unwrap().blockchain().tip().clone();
    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        parent.header().height().checked_increment().unwrap(),
        block_hash(parent.header()),
        merkle_root(&[]),
        StateRoot::ZERO,
        parent.header().pow_target(),
        BlockTimestamp::new(101),
        PowNonce::ZERO,
    );
    let (header, _) = mine_header(header, &PowTarget::MAX).unwrap();
    a.send_to_all(&Message::Block(Block::new(header, vec![])));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        b.snapshot().unwrap().blockchain().canonical_tip_hash(),
        before
    );
}

#[test]
fn behind_node_downloads_and_validates_missing_blocks() {
    let a = bind(9);
    for time in 101..112 {
        a.produce_block(BlockTimestamp::new(time), 0).unwrap();
    }
    let b = bind(10);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| {
        b.snapshot().unwrap().blockchain().canonical_tip_hash()
            == a.snapshot().unwrap().blockchain().canonical_tip_hash()
    });
    assert_eq!(b.snapshot().unwrap().blockchain().block_count(), 12);
    assert_ne!(
        b.snapshot()
            .unwrap()
            .blockchain()
            .tip()
            .header()
            .pow_target(),
        PowTarget::MAX
    );
}

#[test]
fn fork_propagation_obeys_existing_fork_choice() {
    let a = bind(11);
    let b = bind(12);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| a.peer_count() == 1 && b.peer_count() == 1);
    let snapshot = a.snapshot().unwrap();
    let genesis = snapshot.blockchain().tip().clone();
    let state = snapshot.blockchain().state().clone();
    drop(snapshot);
    let canonical = external(&genesis, &state, vec![tx(10, 0)], 101);
    a.accept_block(canonical).unwrap();
    let side = external(&genesis, &state, vec![], 102);
    let side_state = state.clone();
    let extension = external(&side, &side_state, vec![], 103);
    a.accept_block(side).unwrap();
    a.accept_block(extension.clone()).unwrap();
    wait_for(|| {
        b.snapshot().unwrap().blockchain().canonical_tip_hash() == block_hash(extension.header())
    });
    assert_eq!(
        a.snapshot().unwrap().blockchain().canonical_tip_hash(),
        b.snapshot().unwrap().blockchain().canonical_tip_hash()
    );
}

#[test]
fn peer_cannot_forge_an_easier_target_at_adjustment() {
    let a = bind(13);
    let b = bind(14);
    b.connect(a.local_addr()).unwrap();
    wait_for(|| a.peer_count() == 1 && b.peer_count() == 1);
    for timestamp in 101..110 {
        a.produce_block(BlockTimestamp::new(timestamp), 0).unwrap();
    }
    wait_for(|| b.snapshot().unwrap().blockchain().block_count() == 10);
    let snapshot = a.snapshot().unwrap();
    let parent = snapshot.blockchain().tip().clone();
    assert_ne!(
        snapshot.blockchain().next_pow_target().unwrap(),
        PowTarget::MAX
    );
    let transactions = vec![];
    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        parent.header().height().checked_increment().unwrap(),
        block_hash(parent.header()),
        merkle_root(&transactions),
        snapshot.blockchain().state().state_root(),
        PowTarget::MAX,
        BlockTimestamp::new(110),
        PowNonce::ZERO,
    );
    let (header, _) = mine_header(header, &PowTarget::MAX).unwrap();
    let forged = Block::new(header, transactions);
    let before = b.snapshot().unwrap().blockchain().canonical_tip_hash();
    a.send_to_all(&Message::Block(forged));
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        b.snapshot().unwrap().blockchain().canonical_tip_hash(),
        before
    );
}
