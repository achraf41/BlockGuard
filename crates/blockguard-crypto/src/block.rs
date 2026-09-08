use blockguard_core::{
    BlockHash,
    BlockHeader,
    encode_block_header_for_hash,
};

use crate::sha256;

pub fn block_hash(
    header: &BlockHeader,
) -> BlockHash {
    let encoded =
        encode_block_header_for_hash(header);

    let hash =
        sha256(&encoded);

    BlockHash::from_hash(hash)
}