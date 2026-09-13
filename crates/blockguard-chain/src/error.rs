use std::{error::Error, fmt};

use blockguard_state::StateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    InvalidGenesisHeight,
    InvalidGenesisParent,
    GenesisTransactionsNotAllowed,

    WrongChainId,
    UnsupportedBlockVersion,
    UnsupportedTransactionVersion,

    InvalidHeight,
    InvalidPreviousBlockHash,
    InvalidMerkleRoot,
    InvalidStateRoot,
    InvalidProofOfWork,
    UnexpectedPowTarget,
    InvalidBlockTimestamp,

    HeightOverflow,
    ChainWorkOverflow,

    State(StateError),
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGenesisHeight => {
                write!(f, "genesis block must have height zero")
            }

            Self::InvalidGenesisParent => {
                write!(f, "genesis block must have zero previous block hash")
            }

            Self::GenesisTransactionsNotAllowed => {
                write!(f, "genesis block cannot contain normal transactions")
            }

            Self::WrongChainId => {
                write!(f, "wrong chain id")
            }

            Self::UnsupportedBlockVersion => {
                write!(f, "unsupported block version")
            }

            Self::UnsupportedTransactionVersion => {
                write!(f, "unsupported transaction version")
            }

            Self::InvalidHeight => {
                write!(f, "invalid block height")
            }

            Self::InvalidPreviousBlockHash => {
                write!(f, "invalid previous block hash")
            }

            Self::InvalidMerkleRoot => {
                write!(f, "invalid transaction Merkle root")
            }

            Self::InvalidStateRoot => {
                write!(f, "invalid state root")
            }

            Self::HeightOverflow => {
                write!(f, "block height overflow")
            }

            Self::InvalidProofOfWork => {
                write!(f, "invalid proof of work")
            }
            Self::UnexpectedPowTarget => write!(f, "block uses an unexpected proof-of-work target"),
            Self::InvalidBlockTimestamp => {
                write!(f, "block timestamp must be greater than its parent")
            }

            Self::ChainWorkOverflow => {
                write!(f, "cumulative chain work overflow")
            }

            Self::State(error) => {
                write!(f, "transaction state transition failed: {error}")
            }
        }
    }
}

impl Error for ChainError {}

impl From<StateError> for ChainError {
    fn from(error: StateError) -> Self {
        Self::State(error)
    }
}
