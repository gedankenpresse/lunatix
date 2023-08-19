use crate::caps::{Tag, Variant};
use allocators::{Allocator, Box};
use core::cell::RefCell;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::{AsStaticMut, AsStaticRef, CapCounted};
use riscv::trap::TrapFrame;

use super::Capability;

pub struct TaskState {
    pub frame: TrapFrame,
    pub cspace: Capability,
    pub vspace: Capability,
}

pub struct Task<'alloc, 'mem, A: Allocator<'mem>> {
    pub state: CapCounted<'alloc, 'mem, A, RefCell<TaskState>>,
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

impl TaskIface {
    /// Derive a new [`Task`](super::Task) capability from a memory capability.
    pub fn derive(&self, src_mem: &impl AsStaticRef<Capability>, target_slot: &mut Capability) {
        assert_eq!(target_slot.tag, Tag::Uninit);

        // create a new (uninitialized) task state
        let task_state = Box::new(
            RefCell::new(TaskState {
                vspace: Capability::empty(),
                cspace: Capability::empty(),
                frame: TrapFrame::null(),
            }),
            src_mem
                .as_static_ref()
                .get_inner_memory()
                .unwrap()
                .allocator
                .deref(),
        )
        .unwrap();

        // save the capability into the target slot
        target_slot.tag = Tag::Memory;
        target_slot.variant = Variant {
            task: ManuallyDrop::new(Task {
                state: CapCounted::from_box(task_state),
            }),
        };

        todo!()
    }
}

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
