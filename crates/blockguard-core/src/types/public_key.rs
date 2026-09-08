#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKeyBytes([u8; 33]);

impl PublicKeyBytes {
    pub const LENGTH: usize = 33;

    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}
