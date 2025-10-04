// TODO: Write better doc comments for the functions.
// P.S. I don't use any emoji renderer's in my editor. I just like using these slack type emote syntax (insert :bite_me_emote:).
use crate::error::{Result, StormDbError};

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
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// var int_read = page::read_int(5)?;
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
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page::write_int(5, 50)?;
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
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// var bytes_read = page::read_bytes(5)?;
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
            .get((offset + sz)..(offset + varint as usize))
        {
            Some(bytes) => Ok(bytes.into()),
            None => return Err(StormDbError::Corrupt("Invalid String.".to_string())),
        }
    }

    /// Writes the payload as the size of the payload as a `varint` followed by the actual payload at the given offset.
    /// ```
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page::write_bytes(5, vec![0u8, 1, 2, 3])?;
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
        self.byte_buffer[offset + sz..offset + bytes_len].copy_from_slice(&bytes);

        Ok(())
    }

    /// Read the string from the given offset. Returns a string if present, and an error otherwise.
    /// ```
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// var string_read = page::read_string(5)?;
    /// ```
    pub fn read_string(&self, offset: usize) -> Result<String> {
        let string_bytes = self.read_bytes(offset)?;
        Ok(String::from_utf8(string_bytes).map_err(|_| StormDbError::InvalidUtf8)?)
    }

    /// Write the string to the given offset.
    /// ```
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page::write_string(5, "value".to_string())?;
    /// ```
    pub fn write_string(&mut self, offset: usize, value: String) -> Result<()> {
        let string_bytes = value.into_bytes();
        self.write_bytes(offset, string_bytes)?;
        Ok(())
    }

    // Booleans are gonna be 1 byte internally, maybe down the line bit packing might be something I look into.
    /// Reads a boolean value from the given offset.
    /// ```
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// var bool = page::read_bool(5)?;
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
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// page::write_bool(5)?;
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
    /// use file_manager::Page;
    ///
    /// var page = PageBuilder::new().with_block_size(50).with_buffer().build();
    /// var page_bytes = page::bytes(5);
    /// ```
    pub fn bytes(&self) -> &[u8] {
        &self.byte_buffer
    }

    /// Returns the maximum lenght in bytes storing a string would take.
    /// ```
    /// use file_manager::Page;
    ///
    /// var string_size_on_page = Page::max_len("Some String".to_string());
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

// I decided to give claude-code a try. It wrote some good doc comments.
// What I implemented: https://github.com/SumitPatel12/sand/blob/c93298270a7bc5199cc83997dccfba992d5756f5/src/page/file_structures.rs#L432
// Then I decided to check turso for their implementation and look over what could be improved. This one is from truso.
/// Reads a variable-length integer (varint) from a byte buffer.
///
/// This function decodes a big-endian varint starting from the first byte of the provided buffer.
/// Varints are encoded using a continuation bit scheme where the most significant bit (MSB) of each
/// byte indicates whether more bytes follow. A varint can be between 1 and 9 bytes long.
///
/// # Arguments
/// * `buffer` - A byte slice containing the varint to decode
///
/// # Returns
/// Returns a tuple containing:
/// * The decoded `u64` value
/// * The number of bytes consumed (between 1 and 9)
///
/// # Errors
/// Returns `StormDbError::Corrupt` if the buffer contains an invalid varint (e.g., insufficient bytes).
///
/// # Use
/// ```
/// use stormdb_io::{read_varint, write_varint};
///
/// // Read the varint back
/// let (value, varint_size) = read_varint(&buffer).unwrap();
/// ```
pub fn read_varint(buffer: &[u8]) -> Result<(u64, usize)> {
    let mut varint: u64 = 0;

    // The max size of the varint is 9 bytes, and the last byte would be taken as a whole value.
    // Thus we iterate over the firt 8 bytes via for and the last one if present is handled separately.
    for i in 0..8 {
        match buffer.get(i) {
            Some(next_byte) => {
                // Since we reached here we've got a value so shift the original one by 7 and add the next byte after clearing the MSB (most significant bit).
                varint = (varint << 7) + (next_byte & 0x7f) as u64;

                // I initially did next_byte < 0x80. Seemed logically correct. Don't know if using a bitwise and leads to any performance benefits.
                // Tried it in c and the assembly has a cmp for less than version while the bitwise operation did not. Maybe that is the reason.
                if (next_byte & 0x80) == 0 {
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

/// Writes a variable-length integer (varint) to a byte buffer.
///
/// This function encodes a `u64` value as a big-endian varint and writes it to the provided buffer.
/// Varints use a continuation bit scheme where the most significant bit (MSB) of each byte indicates
/// whether more bytes follow. The encoding is space-efficient: smaller values use fewer bytes.
///
/// # Arguments
/// * `buffer` - A mutable byte slice to write the varint to (must have at least 9 bytes available)
/// * `value` - The `u64` value to encode
///
/// # Returns
/// The number of bytes written (between 1 and 9).
///
/// # Varint Size
/// * Values 0-127: 1 byte
/// * Values 128-16,383: 2 bytes
/// * Values up to 2^63-1: up to 9 bytes
///
/// # Use
/// ```
/// use stormdb_io::write_varint;
///
/// // Small value uses 1 byte
/// let mut buffer = vec![0u8; 10];
/// let size = write_varint(&mut buffer, 100);
///
/// // Larger value uses more bytes this one uses 3.
/// let mut buffer = vec![0u8; 10];
/// let size = write_varint(&mut buffer, 50000);
/// ```
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

        for i in (0..8).rev() {
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

    // The while loop above always sets the MSB(most significant bit) to 1, but it shouldn't be so for the last byte.
    // So we're setting it back to 0.
    encoded_varint[0] &= 0x7f;
    // Now since we are going BE (big endian), we'll have to assign the encoded varint to the buffer in reverse order.
    for i in 0..current_varint_size {
        buffer[i] = encoded_varint[current_varint_size - i - 1];
    }

    current_varint_size
}

/// SQLites implementation of the varint. Here only for testing purposes.
pub fn write_varint_sqlite(buf: &mut [u8], value: u64) -> usize {
    if value <= 0x7f {
        buf[0] = (value & 0x7f) as u8;
        return 1;
    }

    if value <= 0x3fff {
        buf[0] = (((value >> 7) & 0x7f) | 0x80) as u8;
        buf[1] = (value & 0x7f) as u8;
        return 2;
    }

    let mut value = value;
    if (value & ((0xff000000_u64) << 32)) > 0 {
        buf[8] = value as u8;
        value >>= 8;
        for i in (0..8).rev() {
            buf[i] = ((value & 0x7f) | 0x80) as u8;
            value >>= 7;
        }
        return 9;
    }

    let mut encoded: [u8; 10] = [0; 10];
    let mut bytes = value;
    let mut n = 0;
    while bytes != 0 {
        let v = 0x80 | (bytes & 0x7f);
        encoded[n] = v as u8;
        bytes >>= 7;
        n += 1;
    }
    encoded[0] &= 0x7f;
    for i in 0..n {
        buf[i] = encoded[n - 1 - i];
    }
    n
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

    #[rstest]
    #[case(100)]
    #[case(20000)]
    #[case(50000000000)]
    #[case(123456890)]
    #[case(887770066111444)]
    #[case(768331757604415801)]
    fn test_write_varint(#[case] value: u64) -> Result<()> {
        let mut buffer_my_fun = vec![0u8; 10];
        let mut buffer_sqlite_fun = vec![0u8; 10];

        let my_varint_size = write_varint(&mut buffer_my_fun, value);
        let sqlite_varint_size = write_varint_sqlite(&mut buffer_sqlite_fun, value);

        assert_eq!(buffer_my_fun, buffer_sqlite_fun);
        assert_eq!(my_varint_size, sqlite_varint_size);
        Ok(())
    }

    #[rstest]
    #[case(100)]
    #[case(20000)]
    #[case(50000000000)]
    #[case(123456890)]
    #[case(887770066111444)]
    fn test_read_varint(#[case] value: u64) -> Result<()> {
        let mut buffer_sqlite_fun = vec![0u8; 10];
        let sqlite_varint_size = write_varint_sqlite(&mut buffer_sqlite_fun, value);

        let (varint_read, varint_size) = read_varint(&mut buffer_sqlite_fun)?;
        assert_eq!(varint_read, value);
        assert_eq!(varint_size, sqlite_varint_size);
        Ok(())
    }
}
