use blockguard_core::Hash256;

use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> Hash256 {
    let digest = Sha256::digest(data);

    let mut bytes = [0u8; 32];

    bytes.copy_from_slice(&digest);

    Hash256::from_bytes(bytes)
}
