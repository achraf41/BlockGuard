use blockguard_consensus::{mine_header, validate_pow};

use blockguard_core::{
    BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion, ChainId, MerkleRoot,
    PowNonce, PowTarget,
};

fn test_target() -> PowTarget {
    let mut bytes = [0xff; 32];

    bytes[0] = 0x00;

    PowTarget::from_bytes(bytes)
}

#[test]
fn mining_produces_valid_proof_of_work() {
    let header = BlockHeader::new(
        BlockVersion::V1,
        ChainId::new(1),
        BlockHeight::new(1),
        BlockHash::ZERO,
        MerkleRoot::ZERO,
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    let target = test_target();

    let (mined_header, _hash) = mine_header(header, &target).unwrap();

    assert!(validate_pow(&mined_header, &target,));
}
