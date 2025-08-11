use std::error::Error;
use std::fmt::{Display, Formatter};

/// Custom Result type for StormDB operations
pub type Result<T> = std::result::Result<T, StormDbError>;

#[derive(Debug)]
pub enum StormDbError {
    OutOfBound(String),
    IndexOutOfBound(usize, usize),
    Corrupt(String),
    InvalidUtf8,
    // Doesn't seem like it'd be easy to use ngl. Wrapping a std error in my own one. But this makes the code a bit simpler so I'll roll with it for now.
    IOError(std::io::Error),
    InvalidBool,
}

impl Error for StormDbError {}

impl Display for StormDbError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IndexOutOfBound(idx, max_idx) => {
                write!(f, "Index out of bounds {}. Max index: {}", idx, max_idx)
            }
            StormDbError::Corrupt(message) => write!(f, "{}", message),
            StormDbError::InvalidUtf8 => write!(f, "Invalid UTF8"),
            StormDbError::IOError(error) => write!(f, "{}", error),
            StormDbError::InvalidBool => write!(f, "Invalid Boolean."),
            StormDbError::OutOfBound(msg) => write!(f, "{}", msg),
        }
    }
}

// This is likely not going to be a good thing performance wise. But I'm not getting into too much premature optimizations for now.
impl From<std::io::Error> for StormDbError {
    fn from(error: std::io::Error) -> Self {
        StormDbError::IOError(error)
    }
}

impl PartialEq for StormDbError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (StormDbError::OutOfBound(a), StormDbError::OutOfBound(b)) => a == b,
            (StormDbError::IndexOutOfBound(a1, a2), StormDbError::IndexOutOfBound(b1, b2)) => {
                a1 == b1 && a2 == b2
            }
            (StormDbError::Corrupt(a), StormDbError::Corrupt(b)) => a == b,
            (StormDbError::InvalidUtf8, StormDbError::InvalidUtf8) => true,
            (StormDbError::IOError(a), StormDbError::IOError(b)) => a.kind() == b.kind(),
            (StormDbError::InvalidBool, StormDbError::InvalidBool) => true,
            _ => false,
        }
    }
}

impl Eq for StormDbError {}
