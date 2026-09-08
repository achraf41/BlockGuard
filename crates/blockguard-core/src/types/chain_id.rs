#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainId(u32);

impl ChainId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}
