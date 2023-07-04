pub const PAGESIZE: usize = 4096;
// TODO make this a wrapper struct and add the repr(C, align(4096)) attribute to it
pub type MemoryPage = [u8; PAGESIZE];
