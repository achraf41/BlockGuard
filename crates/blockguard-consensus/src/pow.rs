use blockguard_core::{BlockHash, BlockHeader, PowNonce, PowTarget};

use crate::PowError;
use blockguard_crypto::block_hash;

pub fn hash_meets_target(hash: &BlockHash, target: &PowTarget) -> bool {
    hash.as_bytes() <= target.as_bytes()
}

pub fn validate_pow(header: &BlockHeader, target: &PowTarget) -> bool {
    let hash = block_hash(header);

    hash_meets_target(&hash, target)
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
