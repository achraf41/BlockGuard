use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use blockguard_chain::{Blockchain, GenesisAllocation, GenesisConfig};
use blockguard_consensus::mine_header;
use blockguard_core::{
    Address, Amount, Block, BlockHeader, BlockTimestamp, BlockVersion, ChainId, Nonce, PowNonce,
    PowTarget, SignedTransaction, TransactionVersion, UnsignedTransaction,
};
use blockguard_crypto::{KeyPair, block_hash, derive_address, merkle_root, sign_transaction};
use blockguard_state::State;
use blockguard_storage::{load, save};

fn key(byte: u8) -> KeyPair {
    KeyPair::from_private_key_bytes([byte; 32]).unwrap()
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
    let alice = key(1);
    let bob = key(2);
    let charlie = key(3);
    GenesisConfig::new(
        ChainId::new(1),
        BlockTimestamp::new(1_700_000_000),
        PowTarget::MAX,
        vec![
            GenesisAllocation::new(
                derive_address(&alice.public_key()),
                Amount::new(1000),
                Nonce::ZERO,
            ),
            GenesisAllocation::new(derive_address(&bob.public_key()), Amount::ZERO, Nonce::ZERO),
            GenesisAllocation::new(
                derive_address(&charlie.public_key()),
                Amount::ZERO,
                Nonce::ZERO,
            ),
        ],
    )
    .unwrap()
}

fn block_after(
    parent: &Block,
    parent_state: &State,
    transactions: Vec<SignedTransaction>,
    timestamp: u64,
) -> (Block, State) {
    let mut state = parent_state.clone();
    for transaction in &transactions {
        state.apply_transaction(transaction).unwrap();
    }
    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        parent.header().height().checked_increment().unwrap(),
        block_hash(parent.header()),
        merkle_root(&transactions),
        state.state_root(),
        parent.header().pow_target(),
        BlockTimestamp::new(timestamp),
        PowNonce::ZERO,
    );
    let (header, _) = mine_header(header, &PowTarget::MAX).unwrap();
    (Block::new(header, transactions), state)
}

fn path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "blockguard-{name}-{}-{unique}.dat",
        std::process::id()
    ))
}

fn forked_chain() -> (GenesisConfig, Blockchain, blockguard_core::BlockHash) {
    let config = config();
    let genesis = config.genesis_block();
    let genesis_state = config.initial_state();
    let alice = key(1);
    let bob = key(2);
    let charlie = key(3);
    let bob_address = derive_address(&bob.public_key());
    let charlie_address = derive_address(&charlie.public_key());
    let mut chain = Blockchain::from_genesis_config(&config).unwrap();

    let (block_a, _) = block_after(
        &genesis,
        &genesis_state,
        vec![transfer(&alice, bob_address, 100, 0)],
        1_700_000_010,
    );
    let block_a_hash = chain.append_block(block_a).unwrap();
    let (block_b, state_b) = block_after(
        &genesis,
        &genesis_state,
        vec![transfer(&alice, charlie_address, 200, 0)],
        1_700_000_011,
    );
    let block_b_parent = block_b.clone();
    chain.append_block(block_b).unwrap();
    let (block_c, _) = block_after(
        &block_b_parent,
        &state_b,
        vec![transfer(&charlie, bob_address, 50, 0)],
        1_700_000_020,
    );
    chain.append_block(block_c).unwrap();
    (config, chain, block_a_hash)
}

#[test]
fn round_trip_preserves_forks_tip_state_and_can_continue() {
    let (config, chain, side_branch_hash) = forked_chain();
    let file = path("round-trip");
    let expected_tip = chain.canonical_tip_hash();

    save(&file, &config, &chain).unwrap();
    let loaded = load(&file).unwrap();
    let (_, mut restored) = loaded.into_parts();
    fs::remove_file(&file).unwrap();

    assert_eq!(restored.canonical_tip_hash(), expected_tip);
    assert_eq!(restored.indexed_block_count(), 4);
    assert!(restored.block(&side_branch_hash).is_some());

    let alice = key(1);
    let bob = key(2);
    let charlie = key(3);
    let alice_address = derive_address(&alice.public_key());
    let bob_address = derive_address(&bob.public_key());
    let charlie_address = derive_address(&charlie.public_key());
    assert_eq!(
        restored.state().balance(&alice_address).unwrap(),
        Amount::new(800)
    );
    assert_eq!(
        restored.state().balance(&bob_address).unwrap(),
        Amount::new(50)
    );
    assert_eq!(
        restored.state().balance(&charlie_address).unwrap(),
        Amount::new(150)
    );
    assert_eq!(
        restored.state().nonce(&alice_address).unwrap(),
        Nonce::new(1)
    );
    assert_eq!(
        restored.state().nonce(&charlie_address).unwrap(),
        Nonce::new(1)
    );

    let (next, _) = block_after(
        restored.tip(),
        restored.state(),
        vec![transfer(&bob, alice_address, 10, 0)],
        1_700_000_030,
    );
    let next_hash = restored.append_block(next).unwrap();
    assert_eq!(restored.canonical_tip_hash(), next_hash);
}

#[test]
fn corrupted_data_is_rejected() {
    let config = config();
    let chain = Blockchain::from_genesis_config(&config).unwrap();
    let file = path("corrupt");
    save(&file, &config, &chain).unwrap();
    let mut bytes = fs::read(&file).unwrap();
    bytes[0] ^= 0xff;
    fs::write(&file, bytes).unwrap();

    assert!(load(&file).is_err());
    fs::remove_file(&file).unwrap();
}

#[test]
fn obsolete_storage_version_is_rejected() {
    let config = config();
    let chain = Blockchain::from_genesis_config(&config).unwrap();
    let file = path("old-version");
    save(&file, &config, &chain).unwrap();
    let mut bytes = fs::read(&file).unwrap();
    bytes[..8].copy_from_slice(b"BGSTORE1");
    bytes[8..10].copy_from_slice(&1u16.to_be_bytes());
    fs::write(&file, bytes).unwrap();
    assert!(load(&file).is_err());
    fs::remove_file(file).unwrap();
}

#[test]
fn round_trip_preserves_adjusted_targets_and_work() {
    let config = config();
    let mut chain = Blockchain::from_genesis_config(&config).unwrap();
    for timestamp in 1_700_000_001..=1_700_000_010 {
        let parent = chain.tip().clone();
        let target = chain.next_pow_target().unwrap();
        let transactions = vec![];
        let header = BlockHeader::new(
            BlockVersion::V1,
            ChainId::new(1),
            parent.header().height().checked_increment().unwrap(),
            block_hash(parent.header()),
            merkle_root(&transactions),
            chain.state().state_root(),
            target,
            BlockTimestamp::new(timestamp),
            PowNonce::ZERO,
        );
        let (header, _) = mine_header(header, &target).unwrap();
        chain
            .append_block(Block::new(header, transactions))
            .unwrap();
    }
    let expected_target = chain.tip().header().pow_target();
    let expected_work = chain
        .metadata(&chain.canonical_tip_hash())
        .unwrap()
        .cumulative_work();
    let file = path("difficulty");
    save(&file, &config, &chain).unwrap();
    let (_, restored) = load(&file).unwrap().into_parts();
    fs::remove_file(file).unwrap();
    assert_eq!(restored.tip().header().pow_target(), expected_target);
    assert_eq!(
        restored
            .metadata(&restored.canonical_tip_hash())
            .unwrap()
            .cumulative_work(),
        expected_work
    );
    assert_eq!(
        restored.next_pow_target().unwrap(),
        chain.next_pow_target().unwrap()
    );
}
