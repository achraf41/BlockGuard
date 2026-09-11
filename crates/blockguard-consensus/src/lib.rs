mod error;
mod pow;

pub use error::PowError;
pub use pow::{hash_meets_target, mine_header, validate_pow};
