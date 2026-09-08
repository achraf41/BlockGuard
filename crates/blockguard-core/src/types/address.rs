#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address([u8; 20]);

impl Address {
    pub const LENGTH: usize = 20;

    pub const fn from_bytes(bytes: [u8; Self::LENGTH]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; Self::LENGTH] {
        &self.0
    }
}
