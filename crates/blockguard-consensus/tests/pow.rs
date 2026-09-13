use blockguard_consensus::{
    EXPECTED_TIMESPAN, adjusted_target, mine_header, target_work, validate_pow,
};

use blockguard_core::{
    BlockHash, BlockHeader, BlockHeight, BlockTimestamp, BlockVersion, ChainId, MerkleRoot,
    PowNonce, PowTarget, StateRoot,
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
        StateRoot::ZERO,
        test_target(),
        BlockTimestamp::new(1_700_000_000),
        PowNonce::ZERO,
    );

    let target = test_target();

    let (mined_header, _hash) = mine_header(header, &target).unwrap();

    assert!(validate_pow(&mined_header));
}

#[test]
fn target_adjustment_clamps_fast_and_slow_intervals() {
    let old = PowTarget::from_bytes([0x40; 32]);
    assert_eq!(
        adjusted_target(old, 0, PowTarget::MAX),
        adjusted_target(old, EXPECTED_TIMESPAN / 4, PowTarget::MAX)
    );
    assert_eq!(
        adjusted_target(old, u64::MAX, PowTarget::MAX),
        adjusted_target(old, EXPECTED_TIMESPAN * 4, PowTarget::MAX)
    );
}

#[test]
fn fast_intervals_harden_and_slow_intervals_ease_target() {
    let old = PowTarget::from_bytes([0x40; 32]);
    let fast = adjusted_target(old, EXPECTED_TIMESPAN / 2, PowTarget::MAX);
    let slow = adjusted_target(old, EXPECTED_TIMESPAN * 2, PowTarget::MAX);
    assert!(fast.as_bytes() < old.as_bytes());
    assert!(slow.as_bytes() > old.as_bytes());
}

#[test]
fn target_adjustment_obeys_pow_limit() {
    let limit = PowTarget::from_bytes([0x7f; 32]);
    assert_eq!(adjusted_target(limit, EXPECTED_TIMESPAN * 4, limit), limit);
}

#[test]
fn harder_targets_have_more_deterministic_work() {
    let easy = target_work(PowTarget::MAX);
    let hard_target = PowTarget::from_bytes([0x3f; 32]);
    let hard = target_work(hard_target);
    assert!(hard.as_bytes() > easy.as_bytes());
    assert_eq!(hard, target_work(hard_target));
    assert_eq!(easy.as_bytes()[31], 1);
}
