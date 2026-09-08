#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash256([u8; 32]);

impl Hash256 {
    pub const LENGTH: usize = 32;
    pub const ZERO: Self = Self([0u8; 32]);

    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}
