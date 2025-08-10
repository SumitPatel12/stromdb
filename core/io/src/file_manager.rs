/*
API will be like:
    public FileMgr(String dbDirectory, int blocksize);
    public void read(BlockId blk, Page p);
    public void write(BlockId blk, Page p);
    public BlockId append(String filename);
    public boolean isNew();
    public int length(String filename);
    public int blockSize();
*/

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::PathBuf,
};

use crate::{block_metadata::BlockMetadata, error::Result, page::Page};

pub struct FileManager {
    db_directory: PathBuf,
    block_size: usize,
    is_new: bool,
    open_files: HashMap<String, File>,
    stats: IOStats,
}

impl FileManager {
    // I think I'll go with Result here, there's a chance opening the directory fails or file creation fails, panicing doesn't seem like the right thing to do.
    /// Retruns a new FileManager struct.
    pub fn new(db_directory: PathBuf, block_size: usize) -> Result<Self> {
        let is_new = !db_directory.exists();
        if is_new {
            // If we fail to create the directory panicing maybe makes sense.
            fs::create_dir_all(&db_directory)?;
        }

        // Once again if this fails, then maybe we're better off panicing.
        // FML there's always a cleaner way to write something in rust. I was reading and handling results and whatnot.
        let db_files = std::fs::read_dir(&db_directory)?;

        // Remove all temp files on startup
        for file in db_files {
            if let Ok(file) = file {
                // TODO: Handle this one as well.
                if !file.file_name().into_string().unwrap().starts_with("temp") {
                    continue;
                } else {
                    std::fs::remove_file(file.path()).expect("failed to remove file");
                }
            }
        }

        Ok(FileManager {
            db_directory,
            block_size,
            is_new,
            open_files: HashMap::new(),
            stats: IOStats::new(),
        })
    }

    /// Returns whether the connection was new or not.
    pub fn is_new(&self) -> bool {
        self.is_new
    }

    /// Returns the block size of the DB instance.
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Get's the file with the specified name from the open files if present. Otherwise opens the file and adds it to the open files hash. If file does not exist one is created.
    fn get_file(&mut self, file_name: &str) -> Result<File> {
        if let Some(file) = self.open_files.get(file_name) {
            // clone returns a reference. I was stuck on that for embarrassingly long time.
            Ok(file.try_clone()?)
        } else {
            let db_table = self.db_directory.join(file_name);
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(db_table)?;
            self.open_files
                .insert(file_name.to_string(), file.try_clone()?);
            Ok(file)
        }
    }

    /// Reads block into given page.
    pub fn read(&mut self, block: BlockMetadata, page: &mut Page) -> Result<()> {
        let mut file = self.get_file(&block.file_name())?;
        file.seek(std::io::SeekFrom::Start(
            (block.block_number() * page.block_size) as u64,
        ))?;

        file.read(page.byte_buffer.as_mut_slice())?;

        Ok(())
    }

    /// Writes block to the file.
    pub fn write(&mut self, block: BlockMetadata, page: &mut Page) -> Result<()> {
        let mut file = self.get_file(&block.file_name())?;
        file.seek(std::io::SeekFrom::Start(
            (block.block_number() * page.block_size) as u64,
        ))?;

        file.write(page.byte_buffer.as_mut_slice())?;
        Ok(())
    }

    /// Appends the block to the file.
    pub fn append(&mut self, file_name: String) -> Result<BlockMetadata> {
        let mut file = self.get_file(&file_name)?;
        let file_metadata = file.metadata()?;
        let block_number = file_metadata.len() as usize / self.block_size;
        let block = BlockMetadata::new(file_name, block_number);
        let bytes = vec![0u8; self.block_size];

        file.seek(std::io::SeekFrom::End(0))?;
        file.write(&bytes)?;
        Ok(block)
    }
}

pub struct IOStats {
    blocks_read: u64,
    blocks_written: u64,
}

// TODO: Implement something in the commit and transaction logics that would keep these values up-to-date.
impl IOStats {
    pub fn new() -> Self {
        IOStats {
            blocks_read: 0,
            blocks_written: 0,
        }
    }

    pub fn blocks_read(&self) -> u64 {
        self.blocks_read
    }

    pub fn blocks_written(&self) -> u64 {
        self.blocks_written
    }

    pub fn set_blocks_read(&mut self, blocks_read: u64) {
        self.blocks_read = blocks_read;
    }

    pub fn set_blocks_write(&mut self, blocks_written: u64) {
        self.blocks_written = blocks_written;
    }
}
