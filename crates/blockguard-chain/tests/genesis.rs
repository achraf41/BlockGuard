use blockguard_chain::{Blockchain, GenesisAllocation, GenesisConfig};
use blockguard_core::{Address, Amount, BlockTimestamp, ChainId, Nonce, PowTarget};

fn allocation(byte: u8, balance: u64) -> GenesisAllocation {
    GenesisAllocation::new(
        Address::from_bytes([byte; Address::LENGTH]),
        Amount::new(balance),
        Nonce::ZERO,
    )
}

fn config(allocations: Vec<GenesisAllocation>) -> GenesisConfig {
    GenesisConfig::new(
        ChainId::new(7),
        BlockTimestamp::new(1_700_000_000),
        PowTarget::MAX,
        allocations,
    )
    .unwrap()
}

#[test]
fn allocation_order_does_not_change_genesis_identity() {
    let first = config(vec![allocation(1, 100), allocation(2, 200)]);
    let second = config(vec![allocation(2, 200), allocation(1, 100)]);

    assert_eq!(first.state_root(), second.state_root());
    assert_eq!(first.genesis_block(), second.genesis_block());
    assert_eq!(first.genesis_hash(), second.genesis_hash());
}

#[test]
fn changing_one_allocation_changes_state_genesis_identity() {
    let first = config(vec![allocation(1, 100)]);
    let second = config(vec![allocation(1, 101)]);

    assert_ne!(first.state_root(), second.state_root());
    assert_ne!(first.genesis_hash(), second.genesis_hash());
}

#[test]
fn generated_genesis_commits_to_generated_initial_state() {
    let config = config(vec![allocation(1, 100), allocation(2, 200)]);
    let state = config.initial_state();
    let genesis = config.genesis_block();

    assert_eq!(genesis.header().state_root(), state.state_root());
    assert!(Blockchain::from_genesis_config(&config).is_ok());
}
