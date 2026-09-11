use blockguard_core::{Hash256, MerkleRoot, SignedTransaction};

use crate::{sha256, transaction_id};

const EMPTY_DOMAIN: &[u8] = b"BLOCKGUARD_MERKLE_EMPTY_V1";

const LEAF_DOMAIN: &[u8] = b"BLOCKGUARD_MERKLE_LEAF_V1";

const NODE_DOMAIN: &[u8] = b"BLOCKGUARD_MERKLE_NODE_V1";

const ODD_DOMAIN: &[u8] = b"BLOCKGUARD_MERKLE_ODD_V1";

pub fn merkle_root(transactions: &[SignedTransaction]) -> MerkleRoot {
    if transactions.is_empty() {
        return MerkleRoot::from_hash(sha256(EMPTY_DOMAIN));
    }

    let mut level: Vec<Hash256> = transactions.iter().map(hash_transaction_leaf).collect();

    while level.len() > 1 {
        let mut next_level = Vec::with_capacity((level.len() + 1) / 2);

        for pair in level.chunks(2) {
            let parent = if pair.len() == 2 {
                hash_node(&pair[0], &pair[1])
            } else {
                hash_odd_node(&pair[0])
            };

            next_level.push(parent);
        }

        level = next_level;
    }

    MerkleRoot::from_hash(level[0])
}

fn hash_transaction_leaf(tx: &SignedTransaction) -> Hash256 {
    let tx_id = transaction_id(tx);

    let mut preimage = Vec::with_capacity(LEAF_DOMAIN.len() + 32);

    preimage.extend_from_slice(LEAF_DOMAIN);

    preimage.extend_from_slice(tx_id.as_bytes());

    sha256(&preimage)
}

fn hash_node(left: &Hash256, right: &Hash256) -> Hash256 {
    let mut preimage = Vec::with_capacity(NODE_DOMAIN.len() + 64);

    preimage.extend_from_slice(NODE_DOMAIN);

    preimage.extend_from_slice(left.as_bytes());

    preimage.extend_from_slice(right.as_bytes());

    sha256(&preimage)
}

fn hash_odd_node(node: &Hash256) -> Hash256 {
    let mut preimage = Vec::with_capacity(ODD_DOMAIN.len() + 32);

    preimage.extend_from_slice(ODD_DOMAIN);

    preimage.extend_from_slice(node.as_bytes());

    sha256(&preimage)
}
