#![no_std]

use core::alloc::Layout;

static mut MMAP_ALLOC: *mut u8 = 0x2a_0000_0000 as *mut u8;

#[derive(Debug)]
pub struct RawRegion {
    pub start: *mut u8,
    pub bytes: usize,
}

pub fn allocate_raw(layout: Layout) -> Result<RawRegion, ()> {
    log::debug!("mmap alloc: {:?}, start: {:p}", &layout, unsafe {
        MMAP_ALLOC
    });
    assert!(layout.size() > 0);
    // only alloc with page alignment
    let layout = layout.align_to(4096).unwrap();

    // get aligned start pointer
    let start_unaligned = unsafe { MMAP_ALLOC };
    let align_offset = start_unaligned.align_offset(layout.align());
    assert!(align_offset != usize::MAX);
    let start = start_unaligned.wrapping_add(align_offset);
    assert!(start.align_offset(layout.align()) == 0);

    // update end pointer
    // hack because some code allocs outside of region
    let end = start.wrapping_add(layout.size() + 4096);
    unsafe { MMAP_ALLOC = end };

    let res = RawRegion {
        start,
        bytes: layout.size(),
    };
    log::debug!("result: {:0x?}", &res);
    Ok(res)
}
