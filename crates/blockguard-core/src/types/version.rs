#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionVersion(u16);

impl TransactionVersion {
    pub const V1: Self = Self(1);

    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}
