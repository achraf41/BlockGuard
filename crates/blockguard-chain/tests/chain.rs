use blockguard_chain::{Blockchain, ChainError};

use blockguard_core::{
    Amount, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion, ChainId,
    Nonce, PowNonce, PowTarget, SignedTransaction, TransactionVersion, UnsignedTransaction,
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

fn genesis_block() -> Block {
    let transactions = Vec::new();

    let root = merkle_root(&transactions);

    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::ZERO,
        BlockHash::ZERO,
        root,
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    Block::new(header, transactions)
}
fn next_block(chain: &Blockchain, transactions: Vec<SignedTransaction>, timestamp: u64) -> Block {
    let previous_hash = block_hash(chain.tip().header());

    let height = chain.tip().header().height().checked_increment().unwrap();

    let root = merkle_root(&transactions);

    let header = BlockHeader::new(
        BlockVersion::V1,
        chain.chain_id(),
        height,
        previous_hash,
        root,
        BlockTimestamp::new(timestamp),
        PowNonce::ZERO,
    );

    let target = chain.pow_target();

    let (mined_header, _) = mine_header(header, &target).unwrap();

    Block::new(mined_header, transactions)
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

    let genesis = genesis_block();

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
fn wrong_previous_hash_is_rejected() {
    let state = State::new();

    let genesis = genesis_block();

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

    let genesis = genesis_block();

    let mut chain = Blockchain::new(ChainId::new(1), PowTarget::MAX, genesis, state).unwrap();

    // This one is valid.
    let tx1 = signed_transfer(&alice, bob_address, 100, 0);

    // Correct next nonce,
    // but Alice cannot afford 1000.
    let tx2 = signed_transfer(&alice, bob_address, 1000, 1);

    let block = next_block(&chain, vec![tx1, tx2], 1_700_000_010);

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
