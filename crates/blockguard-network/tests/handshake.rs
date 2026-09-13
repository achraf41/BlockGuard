use blockguard_core::{BlockHash, ChainId, Hash256};
use blockguard_network::{Handshake, NetworkError, exchange_handshake};
use std::net::{TcpListener, TcpStream};
use std::thread;
fn hash(b: u8) -> BlockHash {
    BlockHash::from_hash(Hash256::from_bytes([b; 32]))
}
fn exchange(
    a: Handshake,
    b: Handshake,
) -> (
    Result<Handshake, NetworkError>,
    Result<Handshake, NetworkError>,
) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let t = thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        exchange_handshake(&mut s, &a)
    });
    let mut s = TcpStream::connect(addr).unwrap();
    let br = exchange_handshake(&mut s, &b);
    (t.join().unwrap(), br)
}
#[test]
fn matching_peers_handshake() {
    let (a, b) = exchange(
        Handshake::new(ChainId::new(1), hash(1), [1; 16]),
        Handshake::new(ChainId::new(1), hash(1), [2; 16]),
    );
    assert!(a.is_ok() && b.is_ok())
}
#[test]
fn wrong_chain_id_is_rejected() {
    let (a, b) = exchange(
        Handshake::new(ChainId::new(1), hash(1), [1; 16]),
        Handshake::new(ChainId::new(2), hash(1), [2; 16]),
    );
    assert!(matches!(a, Err(NetworkError::WrongChainId)));
    assert!(matches!(b, Err(NetworkError::WrongChainId)))
}
#[test]
fn different_genesis_is_rejected() {
    let (a, b) = exchange(
        Handshake::new(ChainId::new(1), hash(1), [1; 16]),
        Handshake::new(ChainId::new(1), hash(2), [2; 16]),
    );
    assert!(matches!(a, Err(NetworkError::DifferentGenesis)));
    assert!(matches!(b, Err(NetworkError::DifferentGenesis)))
}
#[test]
fn unsupported_handshake_version_is_rejected() {
    let good = Handshake::new(ChainId::new(1), hash(1), [1; 16]);
    let mut bad = good.clone();
    bad.protocol_version = 1;
    assert!(matches!(
        blockguard_network::validate_handshake(&good, &bad),
        Err(NetworkError::UnsupportedVersion(1))
    ))
}
