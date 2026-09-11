#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PowTarget([u8; 32]);

impl PowTarget {
    pub const MAX: Self = Self([0xff; 32]);
    pub const ZERO: Self = Self([0u8; 32]);

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
