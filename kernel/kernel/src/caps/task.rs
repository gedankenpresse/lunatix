use allocators::Box;
use core::cell::RefCell;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::CursorHandle;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::Correspondence;
use riscv::pt::MemoryPage;
use riscv::trap::TrapFrame;

use super::CapCounted;
use super::Capability;
use super::Tag;
use super::Variant;

#[derive(Debug, Eq, PartialEq)]
pub enum TaskExecutionState {
    Running,
    Waiting,
    Idle,
}

pub struct TaskState {
    pub frame: TrapFrame,
    pub cspace: Capability,
    pub vspace: Capability,
    pub ipc_buffer: Option<*mut MemoryPage>,
    pub execution_state: TaskExecutionState,
    pub waiting_on: Option<*const Capability>,
}

pub struct Task {
    // TODO: check if this refcell is needed
    pub state: CapCounted<RefCell<TaskState>>,
}

impl Task {
    pub fn get_cspace(&self) -> CursorHandle<'static, Capability> {
        let state = unsafe { self.state.as_ptr().as_ref().unwrap() };
        state.cspace.cursor_handle()
    }

    pub fn get_vspace(&self) -> CursorHandle<'static, Capability> {
        let state = unsafe { self.state.as_ptr().as_ref().unwrap() };
        state.vspace.cursor_handle()
    }
}

impl Correspondence for Task {
    fn corresponds_to(&self, other: &Self) -> bool {
        todo!("correspondence not implemented for task")
    }
}

#[derive(Copy, Clone)]
pub struct TaskIface;

impl TaskIface {
    /// Derive a new [`Task`](super::Task) capability from a memory capability.
    pub fn derive(&self, src_mem: &Capability, target_slot: &mut Capability) {
        assert_eq!(target_slot.tag, Tag::Uninit);

        // create a new (uninitialized) task state
        let task_state = Box::new(
            RefCell::new(TaskState {
                vspace: Capability::empty(),
                cspace: Capability::empty(),
                frame: TrapFrame::null(),
                ipc_buffer: None,
                execution_state: TaskExecutionState::Idle,
                waiting_on: None,
            }),
            src_mem.get_inner_memory().unwrap().allocator.deref(),
        )
        .unwrap();

        // save the capability into the target slot
        target_slot.tag = Tag::Task;
        target_slot.variant = Variant {
            task: ManuallyDrop::new(Task {
                // Safety: it is safe to ignore lifetimes for this box, because the derivation tree ensures correct lifetimes at runtime
                state: CapCounted::from_box(unsafe { Box::ignore_lifetimes(task_state) }),
            }),
        };

        unsafe {
            src_mem.insert_derivation(target_slot);
        }
    }

    /// Wake the task from its waiting state so that it can be scheduled again
    pub fn wake(&self, task: &Capability) {
        assert_eq!(task.tag, Tag::Task);
        let mut state = task.get_inner_task().unwrap().state.borrow_mut();
        log::debug!("waking task");
        state.execution_state = TaskExecutionState::Idle;
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
