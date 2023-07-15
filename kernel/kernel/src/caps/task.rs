use crate::caps;
use core::ptr;
use libkernel::arch::trap::TrapFrame;
use libkernel::mem::PAGESIZE;

pub struct TaskState {
    pub frame: TrapFrame,
    pub cspace: caps::CSlot,
    pub vspace: caps::CSlot,
}

pub struct Task {
    pub state: *mut TaskState,
}

impl TaskState {
    pub fn init(mem: &mut caps::Memory) -> Result<*mut TaskState, caps::errors::NoMem> {
        // allocate a pointer from memory to store our task state
        use core::mem::size_of;
        assert!(size_of::<Self>() <= PAGESIZE);
        let ptr: *mut TaskState = mem.alloc_pages_raw(1)?.cast();

        // initialize the task state
        unsafe {
            ptr::addr_of_mut!((*ptr).cspace).write(caps::CSlot::empty());
            ptr::addr_of_mut!((*ptr).vspace).write(caps::CSlot::empty());
            ptr::addr_of_mut!((*ptr).frame).write(TrapFrame::null());
        }

        Ok(ptr)
    }
}

impl Task {
    pub fn init(slot: &caps::CSlot, mem: &caps::CSlot) -> Result<(), caps::Error> {
        mem.derive(slot, |mem| {
            let taskcap = Self { state: TaskState::init(mem)? };
            return Ok(taskcap.into());
        })
    }
}
