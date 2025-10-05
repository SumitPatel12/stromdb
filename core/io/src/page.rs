// TODO: Write better doc comments for the functions.
// P.S. I don't use any emoji renderer's in my editor. I just like using these slack type emote syntax (insert :bite_me_emote:).
use crate::error::{Result, StormDbError};
use crate::varint::{get_varint_len, read_varint, write_varint};

// Should we have some more data here? Block size, max page size, metadata?
// Yup, the block size is passed as a paramater to one of the constructor methods. I'd rather it be a part of the page itself.
pub struct Page {
    // pub(crate) cause I don't want the file manager calling stupid getters and setters. Might just be my bias. :shrug:
    pub(crate) block_size: usize,
    pub(crate) byte_buffer: Vec<u8>,
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
    /// ```
    /// use file_manager::{Page, PageBuilder};
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_int(5, 100).unwrap();
    /// let int_read = page.read_int(5).unwrap();
    /// ```
    pub fn read_int(&self, offset: usize) -> Result<i32> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        // Initially used 4 directly, but since the language gives us a method I thought of using that. Maybe decreasing a function call would yield better performance?
        // Don't ask me how much time went into finding the syntax of std::mem::size_of::<i32>()
        // AI sometimes does give good suggestions.
        if offset + Self::I32_SIZE >= self.block_size {
            return Err(StormDbError::OutOfBound(
                "Reached end of file before reading the complete int value.".to_string(),
            ));
        }

        // If you use from_le_bytes, *you're a maniac* and I'd love to talk to you about why you choose that.
        Ok(i32::from_be_bytes(
            // I don't think this should fail. (Famous Last Words)
            // Down the line I'll see if unwraping should be removed for some manual checks.
            self.byte_buffer[offset..offset + Self::I32_SIZE]
                .try_into()
                .unwrap(),
        ))
    }

    /// Puts the provided `i32` at the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_int(5, 50).unwrap();
    /// ```
    pub fn write_int(&mut self, offset: usize, value: i32) -> Result<()> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        if offset + Self::I32_SIZE >= self.block_size {
            return Err(StormDbError::OutOfBound(
                "Reached end of file before writing the complete int value.".to_string(),
            ));
        }

        self.byte_buffer[offset..offset + Self::I32_SIZE].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Reads bytes from the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_bytes(5, vec![1, 2, 3]).unwrap();
    /// let bytes_read = page.read_bytes(5).unwrap();
    /// ```
    pub fn read_bytes(&self, offset: usize) -> Result<Vec<u8>> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        let (varint, sz) = read_varint(&self.byte_buffer[offset..])?;

        if offset + Self::I32_SIZE >= self.block_size {
            return Err(StormDbError::OutOfBound(format!(
                "Cannot read {} bytes from offset {}. Last index is {}",
                sz,
                offset,
                self.block_size - 1
            )));
        }

        match self
            .byte_buffer
            // TODO: This can likely be incorrect, for 32 bit systems usize is gonna be 32 while the varint can be 64.
            // We're gonna have to revisit this, possibley overflow pages will solve this. Page size is quite small compared to 32-bit max so I'm hoping that'd do the trick.
            .get((offset + sz)..(offset + sz + varint as usize))
        {
            Some(bytes) => Ok(bytes.into()),
            None => return Err(StormDbError::Corrupt("Invalid String.".to_string())),
        }
    }

    /// Writes the payload as the size of the payload as a `varint` followed by the actual payload at the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_bytes(5, vec![0u8, 1, 2, 3]).unwrap();
    /// ```
    pub fn write_bytes(&mut self, offset: usize, bytes: Vec<u8>) -> Result<()> {
        let bytes_len = bytes.len();
        if offset + bytes_len >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        // This shouldn't happen, hopefully.
        if offset + Self::I32_SIZE >= self.block_size {
            return Err(StormDbError::OutOfBound(
                "Insufficient space to write given bytes".to_string(),
            ));
        }

        let sz = get_varint_len(bytes_len as u64);
        // String won't fit onto the page so we reutrn an error.
        if offset + bytes_len + sz >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        // Write the lenght of the payload as a varint followed by the payload itself.
        write_varint(&mut self.byte_buffer[offset..], bytes_len as u64);
        self.byte_buffer[offset + sz..offset + sz + bytes_len].copy_from_slice(&bytes);

        Ok(())
    }

    /// Read the string from the given offset. Returns a string if present, and an error otherwise.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_string(5, "hello".to_string()).unwrap();
    /// let string_read = page.read_string(5).unwrap();
    /// ```
    pub fn read_string(&self, offset: usize) -> Result<String> {
        let string_bytes = self.read_bytes(offset)?;
        Ok(String::from_utf8(string_bytes).map_err(|_| StormDbError::InvalidUtf8)?)
    }

    /// Write the string to the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_string(5, "value".to_string()).unwrap();
    /// ```
    pub fn write_string(&mut self, offset: usize, value: String) -> Result<()> {
        let string_bytes = value.into_bytes();
        self.write_bytes(offset, string_bytes)?;
        Ok(())
    }

    // Booleans are gonna be 1 byte internally, maybe down the line bit packing might be something I look into.
    /// Reads a boolean value from the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_bool(5, true).unwrap();
    /// let value = page.read_bool(5).unwrap();
    /// ```
    pub fn read_bool(&self, offset: usize) -> Result<bool> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        match self.byte_buffer[offset] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(StormDbError::InvalidBool),
        }
    }

    /// Writes a boolean value to the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_bool(5, true).unwrap();
    /// ```
    pub fn write_bool(&mut self, offset: usize, value: bool) -> Result<()> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        self.byte_buffer[offset] = value as u8;
        Ok(())
    }

    /// Returns an immutable reference to the byte_buffer of the Page.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// let page_bytes = page.bytes();
    /// ```
    pub fn bytes(&self) -> &[u8] {
        &self.byte_buffer
    }

    /// Returns the maximum lenght in bytes storing a string would take.
    /// ```
    /// use file_manager::Page;
    ///
    /// let string_size_on_page = Page::max_len("Some String");
    /// assert!(string_size_on_page > 0);
    /// ```
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
/// use file_manager::Page;
///
/// Page::builder()
///      .with_block_size(50)
///      .with_buffer()
///      .build();
/// ```
pub struct PageBuilder {
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

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(55, vec![0x00, 0x00, 0x00, 0x37])]
    #[case(-55, vec![0xff, 0xff, 0xff, 0xC9])]
    fn test_write_and_read_int(#[case] input: i32, #[case] output: Vec<u8>) -> Result<()> {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        page.write_int(5, input)?;
        assert_eq!(page.bytes()[5..5 + Page::I32_SIZE].to_vec(), output);

        assert_eq!(page.read_int(5)?, input);
        Ok(())
    }

    #[rstest]
    fn test_write_int_offset_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_int(55, 55);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_write_int_offset_plus_size_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_int(48, 55);

        assert_eq!(
            err,
            Err(StormDbError::OutOfBound(
                "Reached end of file before writing the complete int value.".to_string()
            ))
        );
    }

    #[rstest]
    #[case(true, 1u8)]
    #[case(false, 0u8)]
    fn test_write_and_read_bool(#[case] input: bool, #[case] expected_byte: u8) -> Result<()> {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        page.write_bool(5, input)?;
        assert_eq!(page.bytes()[5], expected_byte);

        assert_eq!(page.read_bool(5)?, input);
        Ok(())
    }

    #[rstest]
    fn test_write_bool_offset_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_bool(55, true);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_read_bool_offset_out_of_bounds() {
        let page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.read_bool(55);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    #[case(2u8)]
    #[case(255u8)]
    #[case(128u8)]
    fn test_read_bool_invalid_byte_value(#[case] invalid_byte: u8) {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        // Manually set an invalid byte value
        page.byte_buffer[10] = invalid_byte;

        let err = page.read_bool(10);
        assert_eq!(err, Err(StormDbError::InvalidBool));
    }

    #[rstest]
    fn test_bool_operations_at_different_offsets() -> Result<()> {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        // Test writing and reading bools at various offsets
        page.write_bool(0, true)?;
        page.write_bool(1, false)?;
        page.write_bool(25, true)?;
        page.write_bool(49, false)?; // Last valid index

        assert_eq!(page.read_bool(0)?, true);
        assert_eq!(page.read_bool(1)?, false);
        assert_eq!(page.read_bool(25)?, true);
        assert_eq!(page.read_bool(49)?, false);

        Ok(())
    }

    #[rstest]
    fn test_bool_overwrites_existing_value() -> Result<()> {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        // Write true, then overwrite with false
        page.write_bool(10, true)?;
        assert_eq!(page.read_bool(10)?, true);
        assert_eq!(page.bytes()[10], 1u8);

        page.write_bool(10, false)?;
        assert_eq!(page.read_bool(10)?, false);
        assert_eq!(page.bytes()[10], 0u8);

        Ok(())
    }
}
