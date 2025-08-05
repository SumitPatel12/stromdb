use std::string;

// TODO: Write better doc comments for the functions.
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

    // Okay after going for varint this might very likely become irrelevant :woozy_face:
    // I'll still keep this maybe I'll provide a data-type for i32 who knows.
    /// Reads and returns an `i32` from the given offset if present, None otherwise.
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

    /// Puts the provided `i32` at the given offset.
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

        let (varint, sz) = read_varint(&self.byte_buffer[offset..])?;
        match self
            .byte_buffer
            // TODO: This can likely be incorrect, for 32 bit systems usize is gonna be 32 while the varint can be 64.
            // We're gonna have to revisit this, possibley overflow pages will solve this. Page size is quite small compared to 32-bit max so I'm hoping that'd do the trick.
            .get((offset + sz)..(offset + varint as usize))
        {
            Some(bytes) => Ok(bytes.into()),
            None => return Err(StormDbError::Corrupt("Invalid String.".to_string())),
        }
    }

    /// Writes the payload as the size of the payload as a `varint` followed by the actual payload at the given offset.
    pub fn set_bytes(&mut self, offset: usize, bytes: Vec<u8>) -> Result<(), StormDbError> {
        let bytes_len = bytes.len();
        if offset + bytes_len > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size));
        }

        // TODO: Add some error for this case as well.
        // if offset + bytes.len() > self.block_size {
        //     return Err();
        // }

        let sz = get_varint_len(bytes_len as u64);
        // String won't fit onto the page so we reutrn an error.
        if offset + bytes_len + sz > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size));
        }

        // Write the lenght of the payload as a varint followed by the payload itself.
        write_varint(&mut self.byte_buffer[offset..], bytes_len as u64);
        self.byte_buffer[offset + sz..offset + bytes_len].copy_from_slice(&bytes);

        Ok(())
    }

    /// Read the string from the given offset. Returns a string if present, and an error otherwise.
    pub fn get_string(&self, offset: usize) -> Result<String, StormDbError> {
        let string_bytes = self.get_bytes(offset)?;
        Ok(String::from_utf8(string_bytes).map_err(|_| StormDbError::InvalidUtf8)?)
    }

    /// Write the string to the given offset.
    pub fn set_string(&mut self, offset: usize, value: String) -> Result<(), StormDbError> {
        let string_bytes = value.into_bytes();
        self.set_bytes(offset, string_bytes)?;
        Ok(())
    }

    /// Returns an immutable reference to the byte_buffer of the Page.
    pub fn bytes(&self) -> &[u8] {
        &self.byte_buffer
    }

    /// Returns the maximum lenght in bytes storing a string would take.
    pub fn max_len(string: &str) -> usize {
        let string_bytes = string.as_bytes();
        get_varint_len(string_bytes.len() as u64) + string_bytes.len()
    }
}

// The implementation in java wants two methods for initializing the Page object.
//  1. For data buffers.
//  2. For log buffers.
//
// This might rub some people the wrong way but my C#(that's what I write for my day job :rolling_on_the_floor_laughing:) instincts are telling me to go with a builder,
// so that's what I'm gonna do. I don't get the hate against this. Builders makes it so much easier to setup and execute tests, also chaining methods to make the whole object
// in one go is goated (bite me :stuck_out_tongue:)
/// Builder for the struct `Page`.
/// Use:
/// ```
/// Page::builder()
///      .block_size(desired_block_size)
///      .with_buffer()
///      .build();
/// ```
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
            None => return Err(StormDbError::Corrupt("Invalid Varint.".to_string())),
        }
    }

    if let Some(last_byte) = buffer.get(8) {
        varint = (varint << 8) + (*last_byte as u64);
        Ok((varint, 9))
    } else {
        return Err(StormDbError::Corrupt("Invalid Varint.".to_string()));
    }
}

/// Writes a varint to the given buffer and returns the length of the varint written.
pub fn write_varint(buffer: &mut [u8], value: u64) -> usize {
    if value <= 0x7f {
        buffer[0] = (value & 0x7f) as u8;
        return 1;
    }

    let mut value = value;

    // If any of the bits from 63-56 are set we know for sure it's going to be 9 bytes varint.
    if (value & ((0xff000000_u64) << 32)) > 0 {
        // Big endian so we start assigning from 9th bit towards the 1st one.
        buffer[8] = value as u8;
        value >>= 8;

        for i in (1..8).rev() {
            // Take the 7 least significant bits and set the 8th one to 1.
            buffer[i] = ((value & 0x7f) | 0x80) as u8;
            value >>= 7;
        }

        return 9;
    }

    // Since max size is 9 initializing by that amount.
    let mut encoded_varint = [0u8; 9];
    let mut current_varint_size = 0;

    // As long as the value is still non-zero we will keep taking 7 bytes off of the value and assigning them to the encoded_varint array.
    while value > 0 {
        // Take the 7 least significant bits and set the 8th one to 1.
        let byte = (value & 0x7f) | 0x80;
        encoded_varint[current_varint_size] = byte as u8;

        value >>= 7;
        current_varint_size += 1;
    }

    // Now since we are going BE (big endian), we'll have to assign the encoded varint to the buffer in reverse order.
    for i in 0..current_varint_size {
        buffer[i] = encoded_varint[current_varint_size - i - 1];
    }

    current_varint_size
}

// Prolly not going to use it, implemented as an exercies. :shrug:
/// Calculates the varint size for the given value.
pub fn get_varint_len(value: u64) -> usize {
    if value <= 0x7f {
        return 1;
    }

    if (value & ((0xff000000_u64) << 32)) > 0 {
        return 9;
    }

    let mut value = value;
    let mut n = 0;
    while value != 0 {
        value >>= 7;
        n += 1;
    }
    n
}

// TODO: Write Tests and also pass the function through a fuzzer just in case we need the encoded varint array to be of size 10, and the fuzzer finds something.
