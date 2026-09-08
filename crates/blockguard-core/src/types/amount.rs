#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(u64);

impl Amount {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}
