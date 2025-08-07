use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum StormDbError {
    IndexOutOfBound(usize, usize),
    Corrupt(String),
    InvalidUtf8,
    // Doesn't seem like it'd be easy to use ngl. Wrapping a std error in my own one. But this makes the code a bit simpler so I'll roll with it for now.
    IOError(std::io::Error),
}

impl Error for StormDbError {}

impl Display for StormDbError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Self::IndexOutOfBound(idx, max_idx) => {
                write!(f, "Index out of bounds {}. Max index: {}", idx, max_idx)
            }
            StormDbError::Corrupt(message) => write!(f, "{}", message),
            StormDbError::InvalidUtf8 => write!(f, "Invalid UTF8"),
            StormDbError::IOError(error) => write!(f, "{}", error),
        }
    }
}

// This is likely not going to be a good thing performance wise. But I'm not getting into too much premature optimizations for now.
impl From<std::io::Error> for StormDbError {
    fn from(error: std::io::Error) -> Self {
        StormDbError::IOError(error)
    }
}
