use blockguard_core::PublicKeyBytes;

use k256::{ecdsa::SigningKey, elliptic_curve::Generate};

use crate::CryptoError;

pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    pub fn generate() -> Self {
        Self {
            signing_key: SigningKey::generate(),
        }
    }

    pub fn from_private_key_bytes(bytes: [u8; 32]) -> Result<Self, CryptoError> {
        let signing_key =
            SigningKey::from_slice(&bytes).map_err(|_| CryptoError::InvalidPrivatKey)?;
        Ok(Self { signing_key })
    }

    pub fn public_key(&self) -> PublicKeyBytes {
        let encoded = self.signing_key.verifying_key().to_sec1_point(true);

        let encoded_bytes = encoded.as_bytes();
        let mut bytes = [0u8; 33];

        bytes.copy_from_slice(encoded_bytes);

        PublicKeyBytes::from_bytes(bytes)
    }

    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
}
