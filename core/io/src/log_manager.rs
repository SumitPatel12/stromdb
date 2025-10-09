/*
Log Manager API:
  public LogMgr(FileMgr fm, String logfile);
  public int append(byte[] rec);
  public void flush(int lsn);
  public Iterator<byte[]> iterator();
 */
#![allow(dead_code)]

use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use crate::{
    BlockMetadata, FileManager, Page, PageBuilder, StormDbError, error::Result, get_varint_len,
};

pub struct LogIterator {
    file_manager: Rc<RefCell<FileManager>>,
    log_page: Page,
    block_id: BlockMetadata,
    current_offset: u32,
}

// This one reads form the start of the last page and keeps going back.
impl LogIterator {
    pub fn new(file_manager: Rc<RefCell<FileManager>>, block: &BlockMetadata) -> Self {
        let file_manager_borrowed = file_manager.borrow_mut();
        let bytes = vec![0; file_manager_borrowed.block_size()];
        let mut page = Page::builder()
            .with_block_size(file_manager_borrowed.block_size())
            .with_log_buffer(bytes)
            .build();

        let boundary = Self::move_to_block(file_manager_borrowed, block, &mut page);

        Self {
            file_manager,
            log_page: page,
            block_id: block.clone(),
            current_offset: boundary,
        }
    }

    fn move_to_block(
        mut file_manager: RefMut<FileManager>,
        block: &BlockMetadata,
        log_page: &mut Page,
    ) -> u32 {
        file_manager
            .read(block, log_page)
            .expect("Error reading block to log page.");
        log_page
            .read_u32(0)
            .expect("Error reading boundary from log page.")
    }
}

impl Iterator for LogIterator2 {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        // The boundary at offset 0 indicates where records start
        // We read backwards from current_offset
        // If current_offset <= boundary, we're at the start of this block
        if self.current_offset <= self.log_page.read_u32(0).ok()? {
            // If we're on the first block (block 0), we're done
            if self.block_id.block_number() == 0 {
                return None;
            }

            // Move to the previous block
            self.block_id =
                BlockMetadata::new(&self.block_id.file_name(), self.block_id.block_number() - 1);

            Self::move_to_block(
                self.file_manager.borrow_mut(),
                &self.block_id,
                &mut self.log_page,
            );

            // Set current_offset to the end of the block (block_size - 1)
            self.current_offset = (self.file_manager.borrow().block_size() - 1) as u32;
        }

        // Read the record from the end backwards
        let record_bytes = self
            .log_page
            .read_bytes_for_log_2(self.current_offset as usize)
            .ok()?;

        // Move the offset backwards by the size of the record + varint
        let record_len = record_bytes.len();
        let varint_size = crate::varint::get_varint_len(record_len as u64);
        self.current_offset -= (record_len + varint_size) as u32;

        Some(record_bytes)
    }
}

impl Iterator for LogIterator {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        // If the current block does not have any more records we'd have to check if there is a block before it.
        if self.current_offset >= self.file_manager.borrow_mut().block_size() as u32 {
            // If we're on the last block and we're out of records then we're done for good.
            if self.block_id.block_number() == 0 {
                return None;
            } else {
                // Otherwise load the previous block into the page. And ensure that the current_offset is also set correctly.
                self.block_id = BlockMetadata::new(
                    &self.block_id.file_name(),
                    self.block_id.block_number() - 1,
                );

                Self::move_to_block(
                    self.file_manager.borrow_mut(),
                    &self.block_id,
                    &mut self.log_page,
                );

                self.current_offset = self
                    .log_page
                    .read_u32(0)
                    .expect("Error reading boundary for newly loaded block.");
            }
        }

        let record_bytes = self
            .log_page
            .read_bytes(self.current_offset as usize)
            .expect("Error reading record bytes for iterator");

        self.current_offset +=
            record_bytes.len() as u32 + get_varint_len(record_bytes.len() as u64) as u32;
        Some(record_bytes)
    }
}

pub struct LogIterator2 {
    file_manager: Rc<RefCell<FileManager>>,
    log_page: Page,
    block_id: BlockMetadata,
    current_offset: u32,
}

// This one reads form the end of the last page and keeps going back.
impl LogIterator2 {
    pub fn new(file_manager: Rc<RefCell<FileManager>>, block: &BlockMetadata) -> Self {
        let file_manager_borrowed = file_manager.borrow_mut();
        let bytes = vec![0; file_manager_borrowed.block_size()];
        let mut page = Page::builder()
            .with_block_size(file_manager_borrowed.block_size())
            .with_log_buffer(bytes)
            .build();

        let boundary = Self::move_to_block(file_manager_borrowed, block, &mut page);

        Self {
            file_manager,
            log_page: page,
            block_id: block.clone(),
            current_offset: boundary,
        }
    }

    fn move_to_block(
        mut file_manager: RefMut<FileManager>,
        block: &BlockMetadata,
        log_page: &mut Page,
    ) -> u32 {
        file_manager
            .read(block, log_page)
            .expect("Error reading block to log page.");
        log_page
            .read_u32(0)
            .expect("Error reading boundary from log page.")
    }
}

pub struct LogManager {
    log_file: String,
    file_manager: Rc<RefCell<FileManager>>,
    log_page: Page,
    current_block: BlockMetadata,
    // I think u32 should be more than enough for the lsn numbers for my purposes. We'll see if that needs to change down the line.
    latest_lsn: u32,
    latest_flushed_lsn: u32,
}

impl LogManager {
    pub fn builder(log_file: String, file_manager: Rc<RefCell<FileManager>>) -> LogManagerBuilder {
        LogManagerBuilder::new(log_file, file_manager)
    }

    /// Flushes the values in the log_page to the disk. Only does this if the latest flushed record is smaller than the latest written record.
    pub fn flush(&mut self) {
        if self.latest_lsn >= self.latest_flushed_lsn {
            self.flush_to_file()
        }
    }

    // Appends records from right to left. Boundary is where the latest record should start from. The first 4 bytes will always be a u32 representing the boundary.
    // Block would look something like this:                                 boundary ..................(boundary points here)record1.
    // After one more record insertino Block would look something like this: boundary2......(now boundary points here)record2 record1.
    pub fn append(&mut self, record: Vec<u8>) -> Result<u32> {
        let record_length = record.len();
        // Since bytes are added as varitn of the size followed by the actual bytes, we'd need the varint length for the page fit calculations
        let bytes_needed = get_varint_len(record_length as u64) + record_length;

        match self.log_page.read_u32(0) {
            Ok(mut boundary) => {
                if (boundary as usize - bytes_needed) < size_of::<u32>() {
                    self.flush();
                    self.current_block = self.append_new_block()?;
                    boundary = self.log_page.read_u32(0)?;
                }
                let record_position = boundary as usize - bytes_needed;
                // The question is how would I read it? The varint is stored at the start not the end, so how would the iterator go over it?
                // Ok I read further and it seems like in the book, they just read the first record of the page and onwards.
                // Doesn't really sit well with me. But there's no better way I can think of,
                // other than appending the varint at the end, that also in reverse order so I can read it. :melting_face: :shrug:
                // You know what I'm gonna try that, but then how would I have 9 bytes represented? Lol
                // Also 9 bytes record size is likely never gonna happen.
                self.log_page.write_bytes(record_position, record)?;
                self.log_page.write_u32(0, record_position as u32)?;
                self.latest_lsn += 1;
                Ok(self.latest_lsn)
            }
            // TODO: Maybe have better error reporting.
            Err(_) => {
                return Err(StormDbError::Corrupt(
                    "No Page Availabe for Log Records.".to_string(),
                ));
            }
        }
    }

    pub fn iterator(&self) -> LogIterator {
        LogIterator::new(self.file_manager.clone(), &self.current_block)
    }

    fn flush_to_file(&mut self) {
        self.file_manager
            .borrow_mut()
            .write(&self.current_block, &mut self.log_page)
            .expect("error writing to log file");
        self.latest_flushed_lsn = self.latest_lsn;
    }

    /// Appends a new block to the end of the log_page.
    fn append_new_block(&mut self) -> Result<BlockMetadata> {
        let block_metadata = self.file_manager.borrow_mut().append(&self.log_file)?;
        self.log_page
            .write_u32(0, self.file_manager.borrow_mut().block_size() as u32)?;
        self.file_manager
            .borrow_mut()
            .write(&block_metadata, &mut self.log_page)
            .expect("could not write block id in to log file");

        Ok(block_metadata)
    }
}

// This one will write records in a bit of a differnt format. It will write the record data first then the varint in reverse order.
// Will Likely not be the most performant one. I want to try writing this none the less.
pub struct LogManager2 {
    log_file: String,
    file_manager: Rc<RefCell<FileManager>>,
    log_page: Page,
    current_block: BlockMetadata,
    // I think u32 should be more than enough for the lsn numbers for my purposes. We'll see if that needs to change down the line.
    latest_lsn: u32,
    latest_flushed_lsn: u32,
}

impl LogManager2 {
    pub fn builder(log_file: String, file_manager: Rc<RefCell<FileManager>>) -> LogManagerBuilder {
        LogManagerBuilder::new(log_file, file_manager)
    }

    /// Flushes the values in the log_page to the disk. Only does this if the latest flushed record is smaller than the latest written record.
    pub fn flush(&mut self) {
        if self.latest_lsn >= self.latest_flushed_lsn {
            self.flush_to_file()
        }
    }

    // Appends records from right to left. Boundary is where the latest record should start from. The first 4 bytes will always be a u32 representing the boundary.
    // Block would look something like this:                                 boundary ..................(boundary points here)record1.
    // After one more record insertino Block would look something like this: boundary2......(now boundary points here)record2 record1.
    pub fn append(&mut self, record: Vec<u8>) -> Result<u32> {
        let record_length = record.len();
        // Since bytes are added as varitn of the size followed by the actual bytes, we'd need the varint length for the page fit calculations
        let bytes_needed = get_varint_len(record_length as u64) + record_length;

        match self.log_page.read_u32(0) {
            Ok(mut boundary) => {
                if (boundary as usize - bytes_needed) < size_of::<u32>() {
                    self.flush();
                    self.current_block = self.append_new_block()?;
                    boundary = self.log_page.read_u32(0)?;
                }
                let record_position = boundary as usize - bytes_needed;
                // The question is how would I read it? The varint is stored at the start not the end, so how would the iterator go over it?
                // Ok I read further and it seems like in the book, they just read the first record of the page and onwards.
                // Doesn't really sit well with me. But there's no better way I can think of,
                // other than appending the varint at the end, that also in reverse order so I can read it. :melting_face: :shrug:
                // You know what I'm gonna try that, but then how would I have 9 bytes represented? Lol
                // Also 9 bytes record size is likely never gonna happen.
                self.log_page
                    .write_bytes_for_log_2(record_position, record)?;
                self.log_page.write_u32(0, record_position as u32)?;
                self.latest_lsn += 1;
                Ok(self.latest_lsn)
            }
            // TODO: Maybe have better error reporting.
            Err(_) => {
                return Err(StormDbError::Corrupt(
                    "No Page Availabe for Log Records.".to_string(),
                ));
            }
        }
    }

    pub fn iterator(&self) -> LogIterator {
        LogIterator::new(self.file_manager.clone(), &self.current_block)
    }

    fn flush_to_file(&mut self) {
        self.file_manager
            .borrow_mut()
            .write(&self.current_block, &mut self.log_page)
            .expect("error writing to log file");
        self.latest_flushed_lsn = self.latest_lsn;
    }

    /// Appends a new block to the end of the log_page.
    fn append_new_block(&mut self) -> Result<BlockMetadata> {
        let block_metadata = self.file_manager.borrow_mut().append(&self.log_file)?;
        self.log_page
            .write_u32(0, self.file_manager.borrow_mut().block_size() as u32)?;
        self.file_manager
            .borrow_mut()
            .write(&block_metadata, &mut self.log_page)
            .expect("could not write block id in to log file");

        Ok(block_metadata)
    }
}

// So I tried without a builder method first and it was atrocious to say the least. Code duplication. Needing a separate method for append_new_block that is not on self.
// Having to clone the file_manager multiple times. Alas builder is a vice I must endulge in.
pub struct LogManagerBuilder {
    log_file: String,
    file_manager: Rc<RefCell<FileManager>>,
    log_page: Page,
}

impl LogManagerBuilder {
    pub fn new(log_file: String, file_manager: Rc<RefCell<FileManager>>) -> Self {
        let log_page = PageBuilder::new()
            .with_log_buffer(vec![0; file_manager.borrow().block_size()])
            .build();
        Self {
            log_file,
            file_manager,
            log_page,
        }
    }

    pub fn build(mut self) -> Result<LogManager> {
        let file_manager = self.file_manager.clone();
        let file_last_block_index = file_manager.borrow_mut().last_block_index(&self.log_file);

        let block_metadata = match file_last_block_index {
            Some(last_block_index) => {
                let block_metadata = BlockMetadata::new(&self.log_file, last_block_index);
                self.file_manager
                    .borrow_mut()
                    .read(&block_metadata, &mut self.log_page)
                    .expect("could not read block id in to page");
                block_metadata
            }
            None => self.append_new_block()?,
        };

        Ok(LogManager {
            log_file: self.log_file,
            file_manager: self.file_manager,
            log_page: self.log_page,
            current_block: block_metadata,
            latest_lsn: 0,
            latest_flushed_lsn: 0,
        })
    }

    // Much cleaner than having a method with signature like LogManager::append_new_block(file_manager: Rc<RefCell<FileManager>>, log_file: &str, log_page: &mut Page).
    fn append_new_block(&mut self) -> Result<BlockMetadata> {
        let block_metadata = self.file_manager.borrow_mut().append(&self.log_file)?;
        self.log_page
            .write_u32(0, self.file_manager.borrow_mut().block_size() as u32)?;
        self.file_manager
            .borrow_mut()
            .write(&block_metadata, &mut self.log_page)
            .expect("could not write block id in to log file");

        Ok(block_metadata)
    }
}
