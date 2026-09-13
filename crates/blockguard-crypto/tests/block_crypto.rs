use blockguard_core::{
    Amount, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion, ChainId, Nonce,
    PowNonce, PowTarget, SignedTransaction, StateRoot, TransactionVersion, UnsignedTransaction,
};

use blockguard_crypto::{KeyPair, block_hash, derive_address, merkle_root, sign_transaction};
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
    let unsigned = UnsignedTransaction::new(
        TransactionVersion::V1,
        ChainId::new(1),
        sender.public_key(),
        recipient,
        Amount::new(amount),
        Nonce::new(nonce),
    );

    sign_transaction(sender, unsigned).unwrap()
}

#[test]
fn merkle_root_is_deterministic() {
    let alice = alice_key();

    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let tx1 = signed_transfer(&alice, bob_address, 100, 0);

    let tx2 = signed_transfer(&alice, bob_address, 200, 1);

    let transactions = vec![tx1, tx2];

    let root1 = merkle_root(&transactions);

    let root2 = merkle_root(&transactions);

    assert_eq!(root1, root2,);
}

#[test]
fn changing_transaction_order_changes_merkle_root() {
    let alice = alice_key();

    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let tx1 = signed_transfer(&alice, bob_address, 100, 0);

    let tx2 = signed_transfer(&alice, bob_address, 200, 1);

    let root1 = merkle_root(&[tx1.clone(), tx2.clone()]);

    let root2 = merkle_root(&[tx2, tx1]);

    assert_ne!(root1, root2,);
}

#[test]
fn changing_transaction_changes_merkle_root() {
    let alice = alice_key();

    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let original = signed_transfer(&alice, bob_address, 100, 0);

    let modified = signed_transfer(&alice, bob_address, 101, 0);

    let original_root = merkle_root(&[original]);

    let modified_root = merkle_root(&[modified]);

    assert_ne!(original_root, modified_root,);
}

#[test]
fn empty_transaction_list_has_deterministic_root() {
    let root1 = merkle_root(&[]);

    let root2 = merkle_root(&[]);

    assert_eq!(root1, root2,);
}

#[test]
fn block_hash_is_deterministic() {
    let alice = alice_key();

    let bob = bob_key();

    let bob_address = derive_address(&bob.public_key());

    let tx = signed_transfer(&alice, bob_address, 100, 0);

    let root = merkle_root(&[tx]);

    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        root,
        StateRoot::ZERO,
        PowTarget::MAX,
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    let hash1 = block_hash(&header);

    let hash2 = block_hash(&header);

    assert_eq!(hash1, hash2,);
}

#[test]
fn changing_header_changes_block_hash() {
    let empty_root = merkle_root(&[]);

    let header1 = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        empty_root,
        StateRoot::ZERO,
        PowTarget::MAX,
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    let header2 = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        empty_root,
        StateRoot::ZERO,
        PowTarget::MAX,
        BlockTimestamp::new(1_700_000_001),
        PowNonce::ZERO,
    );

    assert_ne!(block_hash(&header1), block_hash(&header2),);
}

#[test]
fn changing_state_root_changes_block_hash() {
    let empty_root = merkle_root(&[]);
    let first_state_root = StateRoot::from_hash(blockguard_core::Hash256::from_bytes([1; 32]));
    let second_state_root = StateRoot::from_hash(blockguard_core::Hash256::from_bytes([2; 32]));
    let header1 = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        empty_root,
        first_state_root,
        PowTarget::MAX,
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );
    let header2 = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        empty_root,
        second_state_root,
        PowTarget::MAX,
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    assert_ne!(block_hash(&header1), block_hash(&header2));
}

#[test]
fn changing_only_pow_target_changes_block_hash() {
    let root = merkle_root(&[]);
    let first = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        root,
        StateRoot::ZERO,
        PowTarget::MAX,
        BlockTimestamp::new(1),
        PowNonce::ZERO,
    );
    let second = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        root,
        StateRoot::ZERO,
        PowTarget::from_bytes([0x7f; 32]),
        BlockTimestamp::new(1),
        PowNonce::ZERO,
    );
    assert_ne!(block_hash(&first), block_hash(&second));
}
