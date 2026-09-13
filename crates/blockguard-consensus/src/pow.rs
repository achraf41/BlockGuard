use blockguard_core::{BlockHash, BlockHeader, PowNonce, PowTarget};

use crate::PowError;
use blockguard_crypto::block_hash;
use primitive_types::{U256, U512};

pub const TARGET_BLOCK_TIME: u64 = 10;
pub const DIFFICULTY_ADJUSTMENT_INTERVAL: u64 = 10;
pub const EXPECTED_TIMESPAN: u64 = TARGET_BLOCK_TIME * DIFFICULTY_ADJUSTMENT_INTERVAL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowWork([u8; 32]);
impl PowWork {
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

pub fn hash_meets_target(hash: &BlockHash, target: &PowTarget) -> bool {
    hash.as_bytes() <= target.as_bytes()
}

pub fn validate_pow(header: &BlockHeader) -> bool {
    let hash = block_hash(header);
    hash_meets_target(&hash, &header.pow_target())
}

pub fn adjusted_target(old: PowTarget, actual_timespan: u64, pow_limit: PowTarget) -> PowTarget {
    let timespan = actual_timespan.clamp(EXPECTED_TIMESPAN / 4, EXPECTED_TIMESPAN * 4);
    let adjusted = U512::from(U256::from_big_endian(old.as_bytes())) * U512::from(timespan)
        / U512::from(EXPECTED_TIMESPAN);
    let limit = U512::from(U256::from_big_endian(pow_limit.as_bytes()));
    let value: U256 = adjusted
        .min(limit)
        .min(U512::from(U256::MAX))
        .try_into()
        .expect("bounded to U256");
    PowTarget::from_bytes(value.to_big_endian())
}

pub fn target_work(target: PowTarget) -> PowWork {
    let target = U512::from(U256::from_big_endian(target.as_bytes()));
    let quotient = (U512::one() << 256) / (target + U512::one());
    let value = if quotient > U512::from(U256::MAX) {
        U256::MAX
    } else {
        quotient.try_into().expect("bounded to U256")
    };
    PowWork(value.to_big_endian())
}

pub fn mine_header(
    mut header: BlockHeader,
    target: &PowTarget,
) -> Result<(BlockHeader, BlockHash), PowError> {
    let mut nonce = PowNonce::ZERO;

    loop {
        header.set_pow_nonce(nonce);

        let hash = block_hash(&header);

        if hash_meets_target(&hash, target) {
            return Ok((header, hash));
        }

        nonce = nonce.checked_increment().ok_or(PowError::NonceExhausted)?;
    }
}
