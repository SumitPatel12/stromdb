// TODO: Write better doc comments for the functions.
// P.S. I don't use any emoji renderer's in my editor. I just like using these slack type emote syntax (insert :bite_me_emote:).
use crate::error::{Result, StormDbError};
use crate::varint::{
    get_varint_len, get_varint_reversed, read_varint, read_varint_reversed, write_varint,
};

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
    const U32_SIZE: usize = std::mem::size_of::<u32>();

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
        if offset + Self::I32_SIZE > self.block_size {
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

        if offset + Self::I32_SIZE > self.block_size {
            return Err(StormDbError::OutOfBound(
                "Reached end of file before writing the complete int value.".to_string(),
            ));
        }

        self.byte_buffer[offset..offset + Self::I32_SIZE].copy_from_slice(&value.to_be_bytes());
        Ok(())
    }

    /// Reads and returns a `u32` from the given offset if present, None otherwise.
    /// ```
    /// use file_manager::{Page, PageBuilder};
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_u32(5, 100).unwrap();
    /// let uint_read = page.read_u32(5).unwrap();
    /// ```
    pub fn read_u32(&self, offset: usize) -> Result<u32> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        if offset + Self::U32_SIZE > self.block_size {
            return Err(StormDbError::OutOfBound(
                "Reached end of file before reading the complete u32 value.".to_string(),
            ));
        }

        Ok(u32::from_be_bytes(
            self.byte_buffer[offset..offset + Self::U32_SIZE]
                .try_into()
                .unwrap(),
        ))
    }

    /// Puts the provided `u32` at the given offset.
    /// ```
    /// use file_manager::PageBuilder;
    ///
    /// let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page.write_u32(5, 50).unwrap();
    /// ```
    pub fn write_u32(&mut self, offset: usize, value: u32) -> Result<()> {
        if offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        if offset + Self::U32_SIZE > self.block_size {
            return Err(StormDbError::OutOfBound(
                "Reached end of file before writing the complete u32 value.".to_string(),
            ));
        }

        self.byte_buffer[offset..offset + Self::U32_SIZE].copy_from_slice(&value.to_be_bytes());
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

        // If the bytes we're trying to read bytes size (varint) + varint size is greater than the block size then it's not a correct value
        if offset as u64 + sz as u64 + varint > self.block_size as u64 {
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
        if offset + bytes_len > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        let sz = get_varint_len(bytes_len as u64);
        // String won't fit onto the page so we reutrn an error.
        if offset + bytes_len + sz > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        // Write the lenght of the payload as a varint followed by the payload itself.
        write_varint(&mut self.byte_buffer[offset..], bytes_len as u64);
        self.byte_buffer[offset + sz..offset + sz + bytes_len].copy_from_slice(&bytes);

        Ok(())
    }

    pub fn write_bytes_for_log_2(&mut self, offset: usize, bytes: Vec<u8>) -> Result<()> {
        let bytes_len = bytes.len();
        if offset + bytes_len > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        let (varint, sz) = get_varint_reversed(bytes_len as u64);
        // String won't fit onto the page so we reutrn an error.
        if offset + bytes_len + sz > self.block_size {
            return Err(StormDbError::IndexOutOfBound(offset, self.block_size - 1));
        }

        // Write the payload first, followed by reversed varint.
        self.byte_buffer[offset..offset + bytes_len].copy_from_slice(&bytes);
        self.byte_buffer[offset + bytes_len..offset + sz + bytes_len]
            .copy_from_slice(&varint[..sz]);

        Ok(())
    }

    /// Read bytes written by write_bytes_for_log_2. This reads from the end of the record backwards.
    /// The end_offset should point to the last byte of the reversed varint.
    pub fn read_bytes_for_log_2(&self, end_offset: usize) -> Result<Vec<u8>> {
        if end_offset >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(
                end_offset,
                self.block_size - 1,
            ));
        }

        // Read the reversed varint to get the length
        let (bytes_len, varint_size) = read_varint_reversed(&self.byte_buffer, end_offset)?;

        // Calculate the start of the payload
        let payload_start = end_offset + 1 - varint_size - bytes_len as usize;
        let payload_end = end_offset + 1 - varint_size;

        if payload_start >= self.block_size {
            return Err(StormDbError::IndexOutOfBound(
                payload_start,
                self.block_size - 1,
            ));
        }

        // Read the payload
        let mut result = vec![0u8; bytes_len as usize];
        result.copy_from_slice(&self.byte_buffer[payload_start..payload_end]);

        Ok(result)
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
    #[case(55, vec![0x00, 0x00, 0x00, 0x37])]
    #[case(0, vec![0x00, 0x00, 0x00, 0x00])]
    #[case(u32::MAX, vec![0xff, 0xff, 0xff, 0xff])]
    fn test_write_and_read_u32(#[case] input: u32, #[case] output: Vec<u8>) -> Result<()> {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        page.write_u32(5, input)?;
        assert_eq!(page.bytes()[5..5 + Page::U32_SIZE].to_vec(), output);

        assert_eq!(page.read_u32(5)?, input);
        Ok(())
    }

    #[rstest]
    fn test_write_u32_offset_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_u32(55, 55);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_write_u32_offset_plus_size_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_u32(48, 55);

        assert_eq!(
            err,
            Err(StormDbError::OutOfBound(
                "Reached end of file before writing the complete u32 value.".to_string()
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

    #[rstest]
    #[case(vec![1, 2, 3])]
    #[case(vec![0, 255, 128, 64])]
    #[case(vec![])]
    fn test_write_and_read_bytes(#[case] input: Vec<u8>) -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(100)
            .with_buffer()
            .build();

        page.write_bytes(5, input.clone())?;
        let bytes_read = page.read_bytes(5)?;

        assert_eq!(bytes_read, input);
        Ok(())
    }

    #[rstest]
    fn test_write_bytes_offset_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_bytes(55, vec![1, 2, 3]);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_write_bytes_insufficient_space() {
        let mut page = PageBuilder::new().with_block_size(10).with_buffer().build();
        let err = page.write_bytes(5, vec![1, 2, 3, 4, 5, 6, 7, 8]);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(5, 9)));
    }

    #[rstest]
    fn test_read_bytes_offset_out_of_bounds() {
        let page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.read_bytes(55);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_bytes_at_different_offsets() -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(100)
            .with_buffer()
            .build();

        page.write_bytes(0, vec![1, 2, 3])?;
        page.write_bytes(20, vec![4, 5, 6, 7])?;
        page.write_bytes(50, vec![8])?;

        assert_eq!(page.read_bytes(0)?, vec![1, 2, 3]);
        assert_eq!(page.read_bytes(20)?, vec![4, 5, 6, 7]);
        assert_eq!(page.read_bytes(50)?, vec![8]);

        Ok(())
    }

    #[rstest]
    #[case("hello")]
    #[case("")]
    #[case("rust")]
    #[case("test with spaces and special chars !@#$")]
    fn test_write_and_read_string(#[case] input: &str) -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(100)
            .with_buffer()
            .build();

        page.write_string(5, input.to_string())?;
        let string_read = page.read_string(5)?;

        assert_eq!(string_read, input);
        Ok(())
    }

    #[rstest]
    fn test_write_string_offset_out_of_bounds() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.write_string(55, "hello".to_string());

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_write_string_insufficient_space() {
        let mut page = PageBuilder::new().with_block_size(10).with_buffer().build();
        let err = page.write_string(5, "this is a long string".to_string());

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(5, 9)));
    }

    #[rstest]
    fn test_read_string_offset_out_of_bounds() {
        let page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.read_string(55);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_read_string_invalid_utf8() {
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        // Write invalid UTF-8 bytes directly
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        page.write_bytes(5, invalid_utf8).unwrap();

        let err = page.read_string(5);
        assert_eq!(err, Err(StormDbError::InvalidUtf8));
    }

    #[rstest]
    fn test_string_at_different_offsets() -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(200)
            .with_buffer()
            .build();

        page.write_string(0, "first".to_string())?;
        page.write_string(30, "second".to_string())?;
        page.write_string(60, "third".to_string())?;

        assert_eq!(page.read_string(0)?, "first");
        assert_eq!(page.read_string(30)?, "second");
        assert_eq!(page.read_string(60)?, "third");

        Ok(())
    }

    #[rstest]
    fn test_unicode_string() -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(100)
            .with_buffer()
            .build();

        let unicode_str = "Hello 世界 🌍";
        page.write_string(5, unicode_str.to_string())?;
        let string_read = page.read_string(5)?;

        assert_eq!(string_read, unicode_str);
        Ok(())
    }

    #[rstest]
    #[case(vec![1, 2, 3, 4, 5])]
    #[case(vec![0xFF, 0xAA, 0x55])]
    #[case(vec![42])]
    fn test_write_and_read_bytes_for_log_2(#[case] test_bytes: Vec<u8>) -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(400)
            .with_buffer()
            .build();

        // Write bytes at offset 10
        let offset = 10;
        page.write_bytes_for_log_2(offset, test_bytes.clone())?;

        // Calculate the end offset (last byte of the reversed varint)
        // end_offset = offset + bytes_len + varint_size - 1
        let bytes_len = test_bytes.len();
        let varint_size = get_varint_len(bytes_len as u64);
        let end_offset = offset + bytes_len + varint_size - 1;

        // Read back the bytes
        let bytes_read = page.read_bytes_for_log_2(end_offset)?;

        assert_eq!(bytes_read, test_bytes);
        Ok(())
    }

    #[rstest]
    fn test_read_bytes_for_log_2_offset_out_of_bounds() {
        let page = PageBuilder::new().with_block_size(50).with_buffer().build();
        let err = page.read_bytes_for_log_2(55);

        assert_eq!(err, Err(StormDbError::IndexOutOfBound(55, 49)));
    }

    #[rstest]
    fn test_write_bytes_for_log_2_multiple_records() -> Result<()> {
        let mut page = PageBuilder::new()
            .with_block_size(400)
            .with_buffer()
            .build();

        // Write first record at offset 10
        let first_bytes = vec![1, 2, 3, 4, 5];
        let offset1 = 10;
        page.write_bytes_for_log_2(offset1, first_bytes.clone())?;

        let bytes_len1 = first_bytes.len();
        let varint_size1 = get_varint_len(bytes_len1 as u64);
        let end_offset1 = offset1 + bytes_len1 + varint_size1 - 1;

        // Write second record immediately after the first
        let second_bytes = vec![10, 20, 30];
        let offset2 = offset1 + bytes_len1 + varint_size1;
        page.write_bytes_for_log_2(offset2, second_bytes.clone())?;

        let bytes_len2 = second_bytes.len();
        let varint_size2 = get_varint_len(bytes_len2 as u64);
        let end_offset2 = offset2 + bytes_len2 + varint_size2 - 1;

        // Read both records back
        let first_read = page.read_bytes_for_log_2(end_offset1)?;
        let second_read = page.read_bytes_for_log_2(end_offset2)?;

        assert_eq!(first_read, first_bytes);
        assert_eq!(second_read, second_bytes);
        Ok(())
    }

    // Boundary condition tests - testing exact fits at block boundaries

    #[rstest]
    fn test_write_int_at_exact_boundary() -> Result<()> {
        // Block size 8: indices 0-7
        // Writing i32 (4 bytes) at offset 4 should succeed (writes to indices 4,5,6,7)
        let mut page = PageBuilder::new().with_block_size(8).with_buffer().build();

        page.write_int(4, 42)?;
        assert_eq!(page.read_int(4)?, 42);
        Ok(())
    }

    #[rstest]
    fn test_write_int_past_boundary_fails() {
        // Block size 8: indices 0-7
        // Writing i32 (4 bytes) at offset 5 should fail (would write to indices 5,6,7,8)
        let mut page = PageBuilder::new().with_block_size(8).with_buffer().build();

        let err = page.write_int(5, 42);
        assert!(err.is_err());
    }

    #[rstest]
    fn test_read_int_at_exact_boundary() -> Result<()> {
        // Block size 10: indices 0-9
        // Reading i32 (4 bytes) at offset 6 should succeed (reads from indices 6,7,8,9)
        let mut page = PageBuilder::new().with_block_size(10).with_buffer().build();

        page.write_int(6, 100)?;
        assert_eq!(page.read_int(6)?, 100);
        Ok(())
    }

    #[rstest]
    fn test_read_int_past_boundary_fails() {
        // Block size 10: indices 0-9
        // Reading i32 (4 bytes) at offset 7 should fail (would read from indices 7,8,9,10)
        let page = PageBuilder::new().with_block_size(10).with_buffer().build();

        let err = page.read_int(7);
        assert!(err.is_err());
    }

    #[rstest]
    fn test_write_u32_at_exact_boundary() -> Result<()> {
        // Block size 12: indices 0-11
        // Writing u32 (4 bytes) at offset 8 should succeed (writes to indices 8,9,10,11)
        let mut page = PageBuilder::new().with_block_size(12).with_buffer().build();

        page.write_u32(8, 999)?;
        assert_eq!(page.read_u32(8)?, 999);
        Ok(())
    }

    #[rstest]
    fn test_write_u32_past_boundary_fails() {
        // Block size 12: indices 0-11
        // Writing u32 (4 bytes) at offset 9 should fail (would write to indices 9,10,11,12)
        let mut page = PageBuilder::new().with_block_size(12).with_buffer().build();

        let err = page.write_u32(9, 999);
        assert!(err.is_err());
    }

    #[rstest]
    fn test_read_u32_at_exact_boundary() -> Result<()> {
        // Block size 20: indices 0-19
        // Reading u32 (4 bytes) at offset 16 should succeed (reads from indices 16,17,18,19)
        let mut page = PageBuilder::new().with_block_size(20).with_buffer().build();

        page.write_u32(16, 777)?;
        assert_eq!(page.read_u32(16)?, 777);
        Ok(())
    }

    #[rstest]
    fn test_read_u32_past_boundary_fails() {
        // Block size 20: indices 0-19
        // Reading u32 (4 bytes) at offset 17 should fail (would read from indices 17,18,19,20)
        let page = PageBuilder::new().with_block_size(20).with_buffer().build();

        let err = page.read_u32(17);
        assert!(err.is_err());
    }

    #[rstest]
    fn test_write_bytes_at_exact_boundary() -> Result<()> {
        // Block size 10: indices 0-9
        // Writing 3 bytes with 1-byte varint at offset 6 should succeed
        // varint(1 byte) + data(3 bytes) = 4 bytes total, writes to indices 6,7,8,9
        let mut page = PageBuilder::new().with_block_size(10).with_buffer().build();

        let data = vec![1, 2, 3];
        page.write_bytes(6, data.clone())?;
        assert_eq!(page.read_bytes(6)?, data);
        Ok(())
    }

    #[rstest]
    fn test_write_bytes_past_boundary_fails() {
        // Block size 10: indices 0-9
        // Writing 3 bytes with 1-byte varint at offset 7 should fail
        // varint(1 byte) + data(3 bytes) = 4 bytes total, would write to indices 7,8,9,10
        let mut page = PageBuilder::new().with_block_size(10).with_buffer().build();

        let data = vec![1, 2, 3];
        let err = page.write_bytes(7, data);
        assert!(err.is_err());
    }

    #[rstest]
    fn test_read_bytes_at_exact_boundary() -> Result<()> {
        // Block size 50: indices 0-49
        // Writing 5 bytes with 1-byte varint at offset 44 should succeed
        // varint(1 byte) + data(5 bytes) = 6 bytes total, writes to indices 44-49
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        let data = vec![10, 20, 30, 40, 50];
        page.write_bytes(44, data.clone())?;
        assert_eq!(page.read_bytes(44)?, data);
        Ok(())
    }

    #[rstest]
    fn test_read_bytes_past_boundary_fails() {
        // Block size 50: indices 0-49
        // Writing 5 bytes with 1-byte varint at offset 45 should fail
        // varint(1 byte) + data(5 bytes) = 6 bytes total, would write to indices 45-50
        let mut page = PageBuilder::new().with_block_size(50).with_buffer().build();

        let data = vec![10, 20, 30, 40, 50];
        let err = page.write_bytes(45, data);
        assert!(err.is_err());
    }

    #[rstest]
    fn test_write_bytes_for_log_2_at_exact_boundary() -> Result<()> {
        // Block size 20: indices 0-19
        // Writing 4 bytes with 1-byte reversed varint at offset 15 should succeed
        // data(4 bytes) + reversed varint(1 byte) = 5 bytes total, writes to indices 15-19
        let mut page = PageBuilder::new().with_block_size(20).with_buffer().build();

        let data = vec![100, 101, 102, 103];
        let offset = 15;
        page.write_bytes_for_log_2(offset, data.clone())?;

        let bytes_len = data.len();
        let varint_size = get_varint_len(bytes_len as u64);
        let end_offset = offset + bytes_len + varint_size - 1;

        assert_eq!(page.read_bytes_for_log_2(end_offset)?, data);
        Ok(())
    }

    #[rstest]
    fn test_write_bytes_for_log_2_past_boundary_fails() {
        // Block size 20: indices 0-19
        // Writing 4 bytes with 1-byte reversed varint at offset 16 should fail
        // data(4 bytes) + reversed varint(1 byte) = 5 bytes total, would write to indices 16-20
        let mut page = PageBuilder::new().with_block_size(20).with_buffer().build();

        let data = vec![100, 101, 102, 103];
        let err = page.write_bytes_for_log_2(16, data);
        assert!(err.is_err());
    }
}
