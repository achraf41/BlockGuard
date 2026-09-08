#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockVersion(u16);

impl BlockVersion {
    pub const V1: Self = Self(1);

    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}
