use blockguard_core::{
    Address, Amount, Block, BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion,
    ChainId, Hash256, MerkleRoot, Nonce, PowNonce, PowTarget, SignatureBytes, SignedTransaction,
    StateRoot, TransactionVersion, UnsignedTransaction,
};
use blockguard_network::{
    Handshake, MAGIC, MAX_FRAME_SIZE, Message, NetworkError, PROTOCOL_VERSION, encode_frame,
    read_message,
};
use std::io::Cursor;

fn transaction() -> SignedTransaction {
    SignedTransaction::new(
        UnsignedTransaction::new(
            TransactionVersion::V1,
            ChainId::new(3),
            blockguard_core::PublicKeyBytes::from_bytes([2; 33]),
            Address::from_bytes([4; 20]),
            Amount::new(5),
            Nonce::new(6),
        ),
        SignatureBytes::from_bytes([7; 64]),
    )
}
fn block() -> Block {
    Block::new(
        BlockHeader::new(
            BlockVersion::V1,
            ChainId::new(3),
            BlockHeight::new(1),
            BlockHash::from_hash(Hash256::from_bytes([1; 32])),
            MerkleRoot::from_hash(Hash256::from_bytes([2; 32])),
            StateRoot::from_hash(Hash256::from_bytes([3; 32])),
            PowTarget::MAX,
            BlockTimestamp::new(9),
            PowNonce::new(10),
        ),
        vec![transaction()],
    )
}
fn round(message: Message) {
    let encoded = encode_frame(&message).unwrap();
    assert_eq!(read_message(Cursor::new(encoded)).unwrap(), message)
}

#[test]
fn handshake_round_trip() {
    round(Message::Handshake(Handshake::new(
        ChainId::new(3),
        BlockHash::from_hash(Hash256::from_bytes([8; 32])),
        [9; 16],
    )))
}
#[test]
fn transaction_round_trip() {
    round(Message::Transaction(transaction()))
}
#[test]
fn block_round_trip() {
    round(Message::Block(block()))
}
#[test]
fn unknown_message_is_rejected() {
    let mut f = encode_frame(&Message::GetTip).unwrap();
    f[6] = 99;
    assert!(matches!(
        read_message(Cursor::new(f)),
        Err(NetworkError::UnknownMessageType(99))
    ))
}
#[test]
fn unsupported_frame_version_is_rejected() {
    let mut f = encode_frame(&Message::GetTip).unwrap();
    f[4..6].copy_from_slice(&(PROTOCOL_VERSION + 1).to_be_bytes());
    assert!(matches!(
        read_message(Cursor::new(f)),
        Err(NetworkError::UnsupportedVersion(_))
    ))
}
#[test]
fn truncated_frame_is_rejected() {
    let mut f = encode_frame(&Message::Ping(1)).unwrap();
    f.pop();
    assert!(matches!(
        read_message(Cursor::new(f)),
        Err(NetworkError::TruncatedFrame)
    ))
}
#[test]
fn oversized_frame_is_rejected_before_payload_read() {
    let mut f = Vec::new();
    f.extend_from_slice(&MAGIC);
    f.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    f.push(2);
    f.extend_from_slice(&((MAX_FRAME_SIZE as u32) + 1).to_be_bytes());
    assert!(matches!(
        read_message(Cursor::new(f)),
        Err(NetworkError::OversizedFrame(_))
    ))
}
