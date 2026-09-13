use blockguard_chain::{Blockchain, ChainError};

use blockguard_core::{
    Amount, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion, ChainId,
    Nonce, PowNonce, PowTarget, SignedTransaction, StateRoot, TransactionVersion,
    UnsignedTransaction,
};

use blockguard_crypto::{KeyPair, block_hash, derive_address, merkle_root, sign_transaction};

use blockguard_state::{Account, State};

use blockguard_consensus::mine_header;

fn alice_key() -> KeyPair {
    KeyPair::from_private_key_bytes([1u8; 32]).unwrap()
}

fn bob_key() -> KeyPair {
    KeyPair::from_private_key_bytes([2u8; 32]).unwrap()
}

fn charlie_key() -> KeyPair {
    KeyPair::from_private_key_bytes([3u8; 32]).unwrap()
}

fn signed_transfer(
    sender: &KeyPair,
    recipient: blockguard_core::Address,
    amount: u64,
    nonce: u64,
) -> SignedTransaction {
    let tx = UnsignedTransaction::new(
        TransactionVersion::V1,
        ChainId::new(1),
        sender.public_key(),
        recipient,
        Amount::new(amount),
        Nonce::new(nonce),
    );

    sign_transaction(sender, tx).unwrap()
}

fn test_pow_target() -> PowTarget {
    let mut bytes = [0xff; 32];

    bytes[0] = 0x00;

    PowTarget::from_bytes(bytes)
}

fn genesis_block(initial_state: &State) -> Block {
    let transactions = Vec::new();

    let root = merkle_root(&transactions);

    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::ZERO,
        BlockHash::ZERO,
        root,
        initial_state.state_root(),
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    Block::new(header, transactions)
}
fn next_block(chain: &Blockchain, transactions: Vec<SignedTransaction>, timestamp: u64) -> Block {
    block_after(
        chain.tip(),
        chain.state(),
        chain.chain_id(),
        chain.pow_target(),
        transactions,
        timestamp,
    )
}

fn block_after(
    parent: &Block,
    parent_state: &State,
    chain_id: ChainId,
    target: PowTarget,
    transactions: Vec<SignedTransaction>,
    timestamp: u64,
) -> Block {
    let mut resulting_state = parent_state.clone();

    for transaction in &transactions {
        resulting_state.apply_transaction(transaction).unwrap();
    }

    block_with_state_root(
        parent,
        chain_id,
        target,
        transactions,
        resulting_state.state_root(),
        timestamp,
    )
}

fn block_with_state_root(
    parent: &Block,
    chain_id: ChainId,
    target: PowTarget,
    transactions: Vec<SignedTransaction>,
    state_root: StateRoot,
    timestamp: u64,
) -> Block {
    let previous_hash = block_hash(parent.header());
    let height = parent.header().height().checked_increment().unwrap();
    let root = merkle_root(&transactions);
    let header = BlockHeader::new(
        BlockVersion::V1,
        chain_id,
        height,
        previous_hash,
        root,
        state_root,
        BlockTimestamp::new(timestamp),
        PowNonce::ZERO,
    );

    let (mined_header, _) = mine_header(header, &target).unwrap();

    Block::new(mined_header, transactions)
}

#[test]
fn two_competing_children_of_the_same_parent_are_indexed() {
    let genesis = genesis_block(&State::new());
    let genesis_hash = block_hash(genesis.header());
    let mut chain = Blockchain::new(
        ChainId::new(1),
        PowTarget::MAX,
        genesis.clone(),
        State::new(),
    )
    .unwrap();

    let child_a = block_after(
        &genesis,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_010,
    );
    let child_b = block_after(
        &genesis,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_011,
    );

    let child_a_hash = chain.append_block(child_a).unwrap();
    let child_b_hash = chain.append_block(child_b).unwrap();

    assert_eq!(chain.indexed_block_count(), 3);
    assert!(chain.block(&child_a_hash).is_some());
    assert!(chain.block(&child_b_hash).is_some());
    assert_eq!(
        chain.metadata(&child_a_hash).unwrap().parent(),
        genesis_hash
    );
    assert_eq!(
        chain.metadata(&child_b_hash).unwrap().parent(),
        genesis_hash
    );
    assert_eq!(
        chain.metadata(&child_a_hash).unwrap().height(),
        BlockHeight::new(1)
    );
    assert_eq!(
        chain
            .metadata(&child_a_hash)
            .unwrap()
            .cumulative_work()
            .value(),
        1
    );
}

#[test]
fn equal_work_fork_does_not_replace_the_current_tip() {
    let genesis = genesis_block(&State::new());
    let mut chain = Blockchain::new(
        ChainId::new(1),
        PowTarget::MAX,
        genesis.clone(),
        State::new(),
    )
    .unwrap();
    let child_a = block_after(
        &genesis,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_010,
    );
    let child_b = block_after(
        &genesis,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_011,
    );

    let child_a_hash = chain.append_block(child_a).unwrap();
    chain.append_block(child_b).unwrap();

    assert_eq!(chain.canonical_tip_hash(), child_a_hash);
    assert_eq!(chain.block_count(), 2);
}

#[test]
fn more_work_on_a_non_canonical_fork_makes_it_preferred() {
    let genesis = genesis_block(&State::new());
    let mut chain = Blockchain::new(
        ChainId::new(1),
        PowTarget::MAX,
        genesis.clone(),
        State::new(),
    )
    .unwrap();
    let child_a = block_after(
        &genesis,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_010,
    );
    let child_b = block_after(
        &genesis,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_011,
    );
    let child_b_for_extension = child_b.clone();

    chain.append_block(child_a).unwrap();
    let child_b_hash = chain.append_block(child_b).unwrap();
    assert_ne!(chain.canonical_tip_hash(), child_b_hash);

    let grandchild_b = block_after(
        &child_b_for_extension,
        &State::new(),
        chain.chain_id(),
        chain.pow_target(),
        Vec::new(),
        1_700_000_020,
    );
    let grandchild_b_hash = chain.append_block(grandchild_b).unwrap();

    assert_eq!(chain.canonical_tip_hash(), grandchild_b_hash);
    assert_eq!(chain.blocks()[1].header(), child_b_for_extension.header());
    assert_eq!(chain.block_count(), 3);
}

#[test]
fn canonical_state_reorganizes_across_a_competing_fork() {
    let alice = alice_key();
    let bob = bob_key();
    let charlie = charlie_key();
    let alice_address = derive_address(&alice.public_key());
    let bob_address = derive_address(&bob.public_key());
    let charlie_address = derive_address(&charlie.public_key());
    let mut genesis_state = State::new();

    genesis_state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));
    genesis_state.set_account(bob_address, Account::new(Amount::ZERO, Nonce::ZERO));
    genesis_state.set_account(charlie_address, Account::new(Amount::ZERO, Nonce::ZERO));

    let genesis = genesis_block(&genesis_state);
    let genesis_hash = block_hash(genesis.header());
    let mut chain = Blockchain::new(
        ChainId::new(1),
        PowTarget::MAX,
        genesis.clone(),
        genesis_state.clone(),
    )
    .unwrap();

    let block_a = block_after(
        &genesis,
        &genesis_state,
        chain.chain_id(),
        chain.pow_target(),
        vec![signed_transfer(&alice, bob_address, 100, 0)],
        1_700_000_010,
    );
    let block_a_hash = chain.append_block(block_a).unwrap();

    assert_eq!(chain.canonical_tip_hash(), block_a_hash);
    assert_eq!(
        chain.state().balance(&alice_address).unwrap(),
        Amount::new(900)
    );
    assert_eq!(
        chain.state().balance(&bob_address).unwrap(),
        Amount::new(100)
    );
    assert_eq!(
        chain.state().balance(&charlie_address).unwrap(),
        Amount::ZERO
    );
    assert_eq!(chain.state().nonce(&alice_address).unwrap(), Nonce::new(1));

    let transaction_b = signed_transfer(&alice, charlie_address, 200, 0);
    let mut branch_b_state = genesis_state.clone();
    branch_b_state.apply_transaction(&transaction_b).unwrap();
    let block_b = block_after(
        &genesis,
        &genesis_state,
        chain.chain_id(),
        chain.pow_target(),
        vec![transaction_b],
        1_700_000_011,
    );
    let block_b_for_extension = block_b.clone();
    let block_b_hash = chain.append_block(block_b).unwrap();

    assert_eq!(chain.canonical_tip_hash(), block_a_hash);
    assert_eq!(
        chain.state().balance(&alice_address).unwrap(),
        Amount::new(900)
    );
    assert_eq!(
        chain.state().balance(&bob_address).unwrap(),
        Amount::new(100)
    );
    assert_eq!(
        chain.state().balance(&charlie_address).unwrap(),
        Amount::ZERO
    );

    let block_c = block_after(
        &block_b_for_extension,
        &branch_b_state,
        chain.chain_id(),
        chain.pow_target(),
        vec![signed_transfer(&charlie, bob_address, 50, 0)],
        1_700_000_020,
    );
    let block_c_hash = chain.append_block(block_c).unwrap();

    assert_eq!(chain.canonical_tip_hash(), block_c_hash);
    assert_eq!(chain.block_count(), 3);
    assert_eq!(block_hash(chain.blocks()[0].header()), genesis_hash);
    assert_eq!(block_hash(chain.blocks()[1].header()), block_b_hash);
    assert_eq!(block_hash(chain.blocks()[2].header()), block_c_hash);
    assert_eq!(
        chain.state().balance(&alice_address).unwrap(),
        Amount::new(800)
    );
    assert_eq!(
        chain.state().balance(&bob_address).unwrap(),
        Amount::new(50)
    );
    assert_eq!(
        chain.state().balance(&charlie_address).unwrap(),
        Amount::new(150)
    );
    assert_eq!(chain.state().nonce(&alice_address).unwrap(), Nonce::new(1));
    assert_eq!(
        chain.state().nonce(&charlie_address).unwrap(),
        Nonce::new(1)
    );
}

#[test]
fn valid_block_extends_chain_and_updates_state() {
    let alice = alice_key();

    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));

    state.set_account(bob_address, Account::new(Amount::ZERO, Nonce::ZERO));

    let genesis = genesis_block(&state);

    let mut chain = Blockchain::new(ChainId::new(1), test_pow_target(), genesis, state).unwrap();

    let tx = signed_transfer(&alice, bob_address, 250, 0);

    let block = next_block(&chain, vec![tx], 1_700_000_010);

    chain.append_block(block).unwrap();

    assert_eq!(chain.block_count(), 2,);

    assert_eq!(
        chain.state().balance(&alice_address).unwrap(),
        Amount::new(750),
    );

    assert_eq!(
        chain.state().balance(&bob_address).unwrap(),
        Amount::new(250),
    );
}

#[test]
fn incorrect_state_root_is_rejected_without_changing_chain_or_state() {
    let alice = alice_key();
    let bob = bob_key();
    let alice_address = derive_address(&alice.public_key());
    let bob_address = derive_address(&bob.public_key());
    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));
    state.set_account(bob_address, Account::new(Amount::ZERO, Nonce::ZERO));

    let genesis = genesis_block(&state);
    let mut chain = Blockchain::new(ChainId::new(1), PowTarget::MAX, genesis, state).unwrap();
    let original_tip = chain.canonical_tip_hash();
    let transaction = signed_transfer(&alice, bob_address, 250, 0);
    let block = block_with_state_root(
        chain.tip(),
        chain.chain_id(),
        chain.pow_target(),
        vec![transaction],
        StateRoot::ZERO,
        1_700_000_010,
    );

    assert_eq!(chain.append_block(block), Err(ChainError::InvalidStateRoot));
    assert_eq!(chain.canonical_tip_hash(), original_tip);
    assert_eq!(chain.block_count(), 1);
    assert_eq!(chain.indexed_block_count(), 1);
    assert_eq!(
        chain.state().balance(&alice_address).unwrap(),
        Amount::new(1000)
    );
    assert_eq!(chain.state().balance(&bob_address).unwrap(), Amount::ZERO);
    assert_eq!(chain.state().nonce(&alice_address).unwrap(), Nonce::ZERO);
}

#[test]
fn genesis_with_incorrect_initial_state_root_is_rejected() {
    let alice = alice_key();
    let alice_address = derive_address(&alice.public_key());
    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(1000), Nonce::ZERO));

    let transactions = Vec::new();
    let genesis = Block::new(
        BlockHeader::new(
            BlockVersion::V1,
            ChainId::new(1),
            BlockHeight::ZERO,
            BlockHash::ZERO,
            merkle_root(&transactions),
            StateRoot::ZERO,
            BlockTimestamp::new(1_700_000_000),
            PowNonce::ZERO,
        ),
        transactions,
    );

    let result = Blockchain::new(ChainId::new(1), PowTarget::MAX, genesis, state);

    assert!(matches!(result, Err(ChainError::InvalidStateRoot)));
}

#[test]
fn unknown_parent_is_rejected() {
    let state = State::new();

    let genesis = genesis_block(&state);

    let mut chain = Blockchain::new(ChainId::new(1), PowTarget::MAX, genesis, state).unwrap();

    let transactions = Vec::new();

    let root = merkle_root(&transactions);

    let bad_block = Block::new(
        BlockHeader::new(
            BlockVersion::V1,
            ChainId::new(1),
            BlockHeight::new(1),
            // Wrong parent.
            BlockHash::ZERO,
            root,
            chain.state().state_root(),
            BlockTimestamp::new(1_700_000_010),
            PowNonce::ZERO,
        ),
        transactions,
    );

    let result = chain.append_block(bad_block);

    assert_eq!(result, Err(ChainError::InvalidPreviousBlockHash),);

    assert_eq!(chain.block_count(), 1,);
}
#[test]
fn invalid_transaction_rejects_entire_block() {
    let alice = alice_key();

    let bob = bob_key();

    let alice_address = derive_address(&alice.public_key());

    let bob_address = derive_address(&bob.public_key());

    let mut state = State::new();

    state.set_account(alice_address, Account::new(Amount::new(500), Nonce::ZERO));

    state.set_account(bob_address, Account::new(Amount::ZERO, Nonce::ZERO));

    let genesis = genesis_block(&state);

    let mut chain = Blockchain::new(ChainId::new(1), PowTarget::MAX, genesis, state).unwrap();

    // This one is valid.
    let tx1 = signed_transfer(&alice, bob_address, 100, 0);

    // Correct next nonce,
    // but Alice cannot afford 1000.
    let tx2 = signed_transfer(&alice, bob_address, 1000, 1);

    let block = block_with_state_root(
        chain.tip(),
        chain.chain_id(),
        chain.pow_target(),
        vec![tx1, tx2],
        StateRoot::ZERO,
        1_700_000_010,
    );

    let result = chain.append_block(block);

    assert!(result.is_err());

    // Block wasn't added.
    assert_eq!(chain.block_count(), 1,);

    // tx1 must also have been rolled back.
    assert_eq!(
        chain.state().balance(&alice_address).unwrap(),
        Amount::new(500),
    );

    assert_eq!(chain.state().balance(&bob_address).unwrap(), Amount::ZERO,);

    assert_eq!(chain.state().nonce(&alice_address).unwrap(), Nonce::ZERO,);
}
