use crate::arch;
use crate::caps;
use core::ptr;

pub struct TaskState {
    pub frame: arch::trap::TrapFrame,
    pub cspace: caps::CSlot,
}

pub struct Task {
    pub state: *mut TaskState,
}

impl TaskState {
    pub fn init(mem: &mut caps::Memory) -> Result<*mut TaskState, caps::errors::NoMem> {
        // allocate a pointer from memory to store our task state
        use core::mem::size_of;
        assert!(size_of::<Self>() <= crate::mem::PAGESIZE);
        let ptr: *mut TaskState = mem.alloc_pages_raw(1)?.cast();

        // initialize the task state
        unsafe {
            ptr::addr_of_mut!((*ptr).cspace).write(caps::CSlot::default());
            ptr::addr_of_mut!((*ptr).frame).write(arch::trap::TrapFrame::null());
        }

        Ok(ptr)
    }
}

impl Task {
    pub fn init(slot: &mut caps::CSlot, mem: &mut caps::Memory) -> Result<(), caps::Error> {
        let cap = caps::Cap::from_content(Self {
            state: TaskState::init(mem)?,
        });
        slot.set(cap)?;
        Ok(())
    }
}
