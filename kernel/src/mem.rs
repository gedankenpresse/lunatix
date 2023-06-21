pub type Page = [u8; 4096];
pub const PAGESIZE: usize = core::mem::size_of::<Page>();