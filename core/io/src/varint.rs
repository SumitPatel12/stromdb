use crate::error::{Result, StormDbError};

// What I implemented: https://github.com/SumitPatel12/sand/blob/c93298270a7bc5199cc83997dccfba992d5756f5/src/page/file_structures.rs#L432
// Then I decided to check turso for their implementation and look over what could be improved. This one is from truso.
/// Reads a variable-length integer (varint) from a byte buffer.
///
/// This function decodes a big-endian varint starting from the first byte of the provided buffer.
/// Varints are encoded using a continuation bit scheme where the most significant bit (MSB) of each
/// byte indicates whether more bytes follow. A varint can be between 1 and 9 bytes long.
///
/// # Arguments
///
/// * `buffer` - A byte slice containing the varint to decode
///
/// # Returns
///
/// Returns a tuple containing:
/// * The decoded `u64` value
/// * The number of bytes consumed (between 1 and 9)
///
/// # Errors
///
/// Returns `StormDbError::Corrupt` if the buffer contains an invalid varint (e.g., insufficient bytes).
///
/// # Example
///
/// ```
/// use file_manager::{read_varint, write_varint};
///
/// // Write a varint to a buffer
/// let mut buffer = vec![0u8; 10];
/// let bytes_written = write_varint(&mut buffer, 12345);
///
/// // Read the varint back
/// let (value, bytes_read) = read_varint(&buffer).unwrap();
/// assert_eq!(value, 12345);
/// assert_eq!(bytes_read, bytes_written);
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
///
/// * `buffer` - A mutable byte slice to write the varint to (must have at least 9 bytes available)
/// * `value` - The `u64` value to encode
///
/// # Returns
///
/// The number of bytes written (between 1 and 9).
///
/// # Varint Size
///
/// * Values 0-127: 1 byte
/// * Values 128-16,383: 2 bytes
/// * Values up to 2^63-1: up to 9 bytes
///
/// # Example
///
/// ```
/// use file_manager::write_varint;
///
/// // Small value uses 1 byte
/// let mut buffer = vec![0u8; 10];
/// let size = write_varint(&mut buffer, 100);
/// assert_eq!(size, 1);
///
/// // Larger value uses more bytes
/// let mut buffer = vec![0u8; 10];
/// let size = write_varint(&mut buffer, 50000);
/// assert_eq!(size, 3);
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

#[cfg(test)]
mod test {
    use rstest::rstest;

    use super::*;

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
