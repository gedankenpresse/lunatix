use crate::caps;
use core::ptr;
use libkernel::arch::trap::TrapFrame;
use libkernel::mem::PAGESIZE;

use super::{CapabilityInterface, Error, Memory};

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

#[derive(Copy, Clone)]
pub struct TaskIface;

impl CapabilityInterface for TaskIface {
    fn init(&self, slot: &caps::CSlot, mem: &mut Memory) -> Result<caps::Capability, Error> {
        let taskcap = Task {
            state: TaskState::init(mem)?,
        };
        return Ok(taskcap.into());
    }

    fn init_sz(
        &self,
        slot: &caps::CSlot,
        mem: &mut Memory,
        size: usize,
    ) -> Result<caps::Capability, Error> {
        return Err(Error::InvalidOp);
    }

    fn destroy(&self, slot: &caps::CSlot) {
        todo!()
    }

    fn copy(&self, this: &caps::CSlot, target: &caps::CSlot) -> Result<(), Error> {
        todo!()
    }
}
