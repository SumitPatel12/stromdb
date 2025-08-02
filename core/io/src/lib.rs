/*
This is an implementation of SimpleDB from the book: https://www.amazon.in/Database-Design-Implementation-Data-Centric-Applications/dp/3030338355
This is going to be the library we use for file management in the StormDB. The name StormDB cause I know there is going to be a storm of bugs. :laughing_face_emote:

The api as per the book will be:
    1. BlockId for managing blocks. I'll see if I can come up with a better name for this. I don't particlularyly like this name.
    2. Page struct for pages, duh.
    3. FileManager which in turn will manage the files.
*/

mod block_id;
mod file_manager;
mod page;
