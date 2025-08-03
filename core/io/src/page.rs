use crate::error::StormDbError;

// Should we have some more data here? Block size, max page size, metadata?
// Yup, the block size is passed as a paramater to one of the constructor methods. I'd rather it be a part of the page itself.
pub struct Page {
    block_size: usize,
    byte_buffer: Vec<u8>,
}

impl Page {
    // This is prolly not the right thing to do. We'll see.
    const I32_SIZE: usize = std::mem::size_of::<i32>();

    pub fn builder() -> PageBuilder {
        PageBuilder::new()
    }

    /// Reads and returns an i32 from the given offset if present, None otherwise.
    pub fn get_int(&self, offset: usize) -> Result<Option<i32>, StormDbError> {
        if offset > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size));
        }

        // Initially used 4 directly, but since the language gives us a method I thought of using that. Maybe decreasing a function call would yield better performance?
        // Don't ask me how much time went into finding the syntax of std::mem::size_of::<i32>()
        // AI sometimes does give good suggestions.
        if offset + Self::I32_SIZE > self.block_size {
            return Ok(None);
        }

        // If you use from_le_bytes, *you're a maniac* and I'd love to talk to you about why you choose that.
        Ok(Some(i32::from_be_bytes(
            // I don't think this should fail. (Famous Last Words)
            // Down the line I'll see if unwraping should be removed for some manual checks.
            self.byte_buffer[offset..offset + Self::I32_SIZE]
                .try_into()
                .unwrap(),
        )))
    }

    /// Puts the provided integer value at the given offset.
    pub fn set_int(&mut self, offset: usize, value: i32) -> Result<(), StormDbError> {
        if offset > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size));
        }

        // TODO: Add some error for this case as well.
        // if offset + Self::I32_SIZE > self.block_size {
        //     return Err();
        // }

        self.byte_buffer[offset..offset + Self::I32_SIZE].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Reads bytes from the given offset.
    pub fn get_bytes(&self, offset: usize) -> Result<Vec<u8>, StormDbError> {
        if offset > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size));
        }

        // TODO: Add some error for this case as well.
        // if offset + bytes.len() > self.block_size {
        //     return Err();
        // }
    }

    /// Sets/Overwrites the slice of the buffer from offset to offset + bytes.len() by the provided bytes.
    pub fn set_bytes(&mut self, offset: usize, bytes: Vec<u8>) -> Result<(), StormDbError> {
        if offset > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size));
        }

        // TODO: Add some error for this case as well.
        // if offset + bytes.len() > self.block_size {
        //     return Err();
        // }

        self.byte_buffer[offset..offset + Self::I32_SIZE].copy_from_slice(&bytes);

        Ok(())
    }
}

// The implementation in java wants two methods for initializing the Page object.
//  1. For data buffers.
//  2. For log buffers.
//
// This might rub some people the wrong way but my C#(that's what I write for my day job :rolling_on_the_floor_laughing:) instincts are telling me to go with a builder,
// so that's what I'm gonna do. I don't get the hate against this. Builders makes it so much easier to setup and execute tests, also chaining methods to make the whole object
// in one go is goated (bite me :stuck_out_tongue:)
struct PageBuilder {
    block_size: usize,
    byte_buffer: Vec<u8>,
}

impl PageBuilder {
    // Since rust doesn't support passing a default value to the function, I'll just keep the block size initialization as its own function.
    // Don't like it much but better than passing a param in new.
    /// Initializes a PageBuilder with a block size of 0 and an empty vec.
    pub fn new() -> Self {
        PageBuilder {
            block_size: 0,
            byte_buffer: Vec::new(),
        }
    }

    /// Set the value of block size.
    pub fn with_block_size(&mut self, block_size: usize) -> &mut Self {
        self.block_size = block_size;
        self
    }

    /// Initialized the vec with a capacity of self.block_size.
    /// Make sure to set the block size before you do this. Otherwise the vec will have a capacity of 0.
    pub fn with_buffer(&mut self) -> &mut Self {
        self.byte_buffer = vec![0; self.block_size];
        self
    }

    /// Sets the buffer to the one that is provided in the params. Set's the block size to the length of the provided buffer.
    pub fn with_log_buffer(&mut self, buffer: Vec<u8>) -> &mut Self {
        self.block_size = buffer.len();
        self.byte_buffer = buffer;
        self
    }

    /// Returns a Page with the current state of the PageBuilder.
    pub fn build(&mut self) -> Page {
        // No cloning because it incurs memory overhead, at least that's what I dug out of it.
        // I think this is better, will have to ask someone though.
        Page {
            block_size: self.block_size,
            // Neat thing, I used clone initially but found out about this later.
            // This imo is better than cloning cause we'll move the original vec out, and simultaneously allocate it to default Vec::new() which does no allocation,
            byte_buffer: std::mem::take(&mut self.byte_buffer),
        }
    }
}

// What I implemented: https://github.com/SumitPatel12/sand/blob/c93298270a7bc5199cc83997dccfba992d5756f5/src/page/file_structures.rs#L432
// Then I decided to check turso for their implementation and look over what could be improved. This one is from truso.
/// Reads a big endian varint starting from the first byte of the byte slice.
pub fn read_varint(buffer: &[u8]) -> Result<(u64, usize), StormDbError> {
    let mut varint: u64 = 0;

    // The max size of the varint is 9 bytes, and the last byte would be taken as a whole value.
    // Thus we iterate over the firt 8 bytes via for and the last one if present is handled separately.
    for i in 0..8 {
        match buffer.get(i) {
            Some(next_byte) => {
                // Since we reached here we've got a value so shift the original one by 7 and add the next byte after clearing the MSB (most significant bit).
                varint = varint >> 7 + (next_byte & 0x7f) as u64;

                // I initially did next_byte < 0x80. Seemed logically correct. Don't know if using a bitwise and leads to any performance benefits.
                // Tried it in c and the assembly has a cmp for less than version while the bitwise operation did not. Maybe that is the reason.
                if next_byte & 0x80 == 0 {
                    return Ok((varint, i + 1));
                }
            }
            None => return Err(StormDbError::InvalidVarint),
        }
    }

    if let Some(last_byte) = buffer.get(8) {
        varint = (varint << 8) + (*last_byte as u64);
        Ok((varint, 9))
    } else {
        return Err(StormDbError::InvalidVarint);
    }
}

/// Writes a varint given the buffer
pub fn write_varint(buffer: &mut [u8], value: u64) -> usize {
    if value <= 0x7f {
        buffer[0] = (value & 0x7f) as u8;
        return 1;
    }

    let mut varint = value;
}
