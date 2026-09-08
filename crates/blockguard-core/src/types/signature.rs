#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignatureBytes([u8; 64]);

impl SignatureBytes {
    pub const LENGTH: usize = 64;

    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}
