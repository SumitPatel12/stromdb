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
