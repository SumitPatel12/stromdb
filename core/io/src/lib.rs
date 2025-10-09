/*
This is an implementation of SimpleDB from the book "Database Design Implementation Data Centric Applications"
This is going to be the library we use for file management in the StormDB. The name StormDB cause I know there is going to be a storm of bugs. :laughing_face_emote:
P.S. I wrote the :emote: thing knowingly, it doesn't render as an emote in my editor either. :stuck_out_tongue:.
P.P.S I konw my jokes are horrible, hopefully better than my code.

The api as per the book will be:
    1. BlockMetadata that'll handle block related operations. I like this name better. Don't know id just didn't seem right.
        a. BlcokManager doesn't work cause this is handling the details of single block.
        b. BlockId feels wrong cause it has the number and fileName of the block.
        c. Block feels wrong cause that name seems to suggest it would contain everything about the block and not just the metadata.
        d. BlockMetadata seems like the right choice for me. Once again can change down the line.
    2. Page struct for pages, duh.
    3. FileManager which in turn will manage the files.
*/

// TODO: Add some error type and use that throughout the library. If I'm doing it, might as well do it right.
// TODO: I think everything should be pub create and not just pub directly.

mod block_metadata;
mod error;
mod file_manager;
mod log_manager;
mod page;
pub mod varint;

pub use block_metadata::BlockMetadata;
pub use error::{Result, StormDbError};
pub use file_manager::FileManager;
pub use page::{Page, PageBuilder};
pub use varint::{
    get_varint_len, get_varint_reversed, read_varint, read_varint_reversed, write_varint,
    write_varint_sqlite,
};
