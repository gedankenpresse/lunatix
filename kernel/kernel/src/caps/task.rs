use crate::caps;
use core::ptr;
use derivation_tree::caps::CapabilityIface;
use riscv::trap::TrapFrame;

use super::Capability;

pub struct TaskState {
    pub frame: TrapFrame,
    pub cspace: caps::Capability,
    pub vspace: caps::Capability,
}

pub struct Task {
    pub state: *mut TaskState,
}

/*
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
*/

#[derive(Copy, Clone)]
pub struct TaskIface;

impl CapabilityIface<Capability> for TaskIface {
    type InitArgs = ();

    fn init(
        &self,
        target: &mut impl derivation_tree::AsStaticMut<Capability>,
        args: Self::InitArgs,
    ) {
        todo!()
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl derivation_tree::AsStaticMut<Capability>,
    ) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
