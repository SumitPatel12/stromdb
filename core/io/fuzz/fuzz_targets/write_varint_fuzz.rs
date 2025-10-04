#![no_main]

use libfuzzer_sys::fuzz_target;

use file_manager::{write_varint, write_varint_sqlite};

fuzz_target!(|data: u64| {
    let mut buffer_my_fun = vec![0u8; 10];
    let mut buffer_sqlite_fun = vec![0u8; 10];

    let my_varint_size = write_varint(&mut buffer_my_fun, data);
    let sqlite_varint_size = write_varint_sqlite(&mut buffer_sqlite_fun, data);

    assert_eq!(buffer_my_fun, buffer_sqlite_fun);
    assert_eq!(my_varint_size, sqlite_varint_size);
});
