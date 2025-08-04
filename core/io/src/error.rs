use std::error::Error;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum StormDbError {
    IndexOutOfBound(usize, usize),
    Corrupt(String),
}

impl Error for StormDbError {}

impl Display for StormDbError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Self::IndexOutOfBound(idx, max_idx) => {
                write!(f, "Index out of bounds {}. Max index: {}", idx, max_idx)
            }
            StormDbError::Corrupt(message) => write!(f, message),
        }
    }
}
