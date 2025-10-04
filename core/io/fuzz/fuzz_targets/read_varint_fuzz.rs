#![no_main]

use libfuzzer_sys::fuzz_target;

use file_manager::{read_varint, write_varint_sqlite};

fuzz_target!(|data: u64| {
    let mut buffer_sqlite_fun = vec![0u8; 10];
    let sqlite_varint_size = write_varint_sqlite(&mut buffer_sqlite_fun, data);

    let (varint_read, varint_size) = read_varint(&buffer_sqlite_fun).unwrap();
    assert_eq!(varint_read, data);
    assert_eq!(varint_size, sqlite_varint_size);
});
