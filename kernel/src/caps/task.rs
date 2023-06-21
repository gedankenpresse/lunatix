use crate::arch;
use crate::caps;

pub struct TaskState {
    frame: *mut arch::trap::TrapFrame,
    cspace: caps::CSlot, 
}

pub struct Task {
    state: *mut TaskState,
}

impl TaskState {
    pub fn init(mem: &mut caps::Memory) -> Result<caps::Cap<Self>, caps::errors::NoMem> {
        use core::mem::size_of;
        assert!(size_of::<Self>() <= crate::mem::PAGESIZE);
        let ptr = mem.alloc_pages_raw(1)?;
        let cap = caps::Cap::from_content(Self {
            frame: ptr as *mut arch::trap::TrapFrame,
            cspace: caps::CSlot::default(),
        });
        Ok(cap)
    }
}