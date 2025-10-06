use std::{fmt::Display, hash::Hash};

/// Stores the metadata for a block. Might change down the line.
/// Currently stores the following two things:
///  1. Name of the file containing the block.
///  2. Logical index/number of the block in the said file.
#[derive(Eq, PartialEq, Hash)]
pub struct BlockMetadata {
    file_name: String,
    block_number: usize,
}

impl BlockMetadata {
    pub fn new(file_name: &str, block_number: usize) -> Self {
        BlockMetadata {
            file_name: file_name.to_string(),
            block_number,
        }
    }

    // I'm not really sure why getter would be required here. I'll look into why this would be a good practice.
    // Maybe because we don't want anyone directly accessing our internal members, makes sense when you think like that.
    // We don't want anyone tinkering directly with the objects data.
    /// Returns the name of the file that contains the block.
    pub fn file_name(&self) -> String {
        self.file_name.clone()
    }

    /// Returns the logical index of the block in the file.
    pub fn block_number(&self) -> usize {
        self.block_number
    }
}

impl Display for BlockMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!(
                "[file {}, block number {}]",
                self.file_name(),
                self.block_number
            ),
        )
    }
}
