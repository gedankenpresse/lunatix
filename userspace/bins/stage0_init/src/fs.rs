use crate::{CADDR_DEVMEM, CADDR_IRQ_CONTROL, CADDR_MEM, CADDR_VSPACE};
use core::cell::RefCell;
use virtio_p9::{init_9p_driver, P9Driver};

pub static FS: FileSystem = FileSystem(RefCell::new(None));
pub struct FileSystem(RefCell<Option<P9Driver<'static>>>);
unsafe impl Send for FileSystem {}
unsafe impl Sync for FileSystem {}

pub fn init() {
    log::debug!("initializing filesystem over 9p");
    let p9 = init_9p_driver(CADDR_MEM, CADDR_VSPACE, CADDR_DEVMEM, CADDR_IRQ_CONTROL);
    let _ = FS.0.borrow_mut().insert(p9);
}
