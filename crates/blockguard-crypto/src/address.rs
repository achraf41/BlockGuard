use blockguard_core::{Address, PublicKeyBytes};

use crate::sha256;

pub fn derive_address(public_key: &PublicKeyBytes) -> Address {
    let hash = sha256(public_key.as_bytes());

    let hash_bytes = hash.as_bytes();

    let mut address = [0u8; 20];

    address.copy_from_slice(&hash_bytes[12..32]);

    Address::from_bytes(address)
}
