use crate::caps::irq::Irq;
use crate::caps::{CapCounted, Capability, KernelAlloc, Tag, Variant};
use allocators::bump_allocator::BumpAllocator;
use allocators::{Allocator, Box};
use core::cell::RefCell;
use core::mem;
use core::mem::{ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::ptr::addr_of_mut;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef, Correspondence};

/// How many interrupt lines are present in the RISCV plic.
///
/// The spec defines 1024 interrupt sources which are technically multiplied by interrupt contexts
/// (dot product of hart id, privilege level, etc) but because userspace does not care about the
/// context, we only need to keep track of the different interrupt sources.
const NUM_INTERRUPT_LINES: usize = 1024;

/// The internal shared state of an IrqControl capability
pub struct IrqControlState {
    /// An tracking which interrupt lines are already mapped to interrupt handlers (which are IRQ capabilities).
    interrupt_lines: [RefCell<Capability>; NUM_INTERRUPT_LINES],
}

impl IrqControlState {
    /// Create a new state object where all interrupt lines are unclaimed
    fn init(value: &mut MaybeUninit<Self>) {
        let interrupt_lines = addr_of_mut!(
            unsafe { value.as_mut_ptr().as_mut() }
                .unwrap()
                .interrupt_lines
        );

        // initialize interrupt lines as a unclaimed capabilities
        for i in 0..NUM_INTERRUPT_LINES {
            unsafe {
                interrupt_lines
                    .cast::<RefCell<Capability>>()
                    .add(i)
                    .write(RefCell::new(Capability::empty()))
            };
        }
    }
}

/// An IrqControl capability used for claiming the handling of specific interrupt lines.
pub struct IrqControl {
    pub state: CapCounted<IrqControlState>,
}

impl Correspondence for IrqControl {
    fn corresponds_to(&self, other: &Self) -> bool {
        todo!("correspondence not implemented for task")
    }
}

/// An interface for interacting with IrqControl capabilities
#[derive(Copy, Clone)]
pub struct IrqControlIface;

impl IrqControlIface {
    /// Try to claim a specific interrupt line or fail if it is already claimed.
    ///
    /// Return a pointer to an IRQ capability that handles the claimed interrupt line.
    pub fn try_claim_line(&self, cap: &mut Capability, line: usize) -> Result<*mut Capability, ()> {
        let irq_control = cap.get_inner_irq_control_mut().unwrap();

        // initialize a notification capability into the slot of the interrupt line
        let mut irq_slot = irq_control
            .state
            .interrupt_lines
            .get(line)
            .ok_or(())?
            .borrow_mut();
        irq_slot.tag = Tag::Irq;
        irq_slot.variant = Variant {
            irq: ManuallyDrop::new(Irq {
                interrupt_line: line,
            }),
        };

        // insert the newly created irq into the derivation tree
        unsafe {
            // this is needed to remove the lifetime bound to state which needs to be dropped to mutably use cap again
            //
            // it is safe to do because aliases can not occur while we have an &mut reference to cap which is the only
            // place where a reference to the irq_slot could be obtained from
            let irq_slot2 = &mut *(irq_slot.deref_mut() as *mut Capability);
            drop(irq_slot);
            cap.insert_derivation(irq_slot2);
            Ok(irq_slot2 as *mut _)
        }
    }

    /// Initialize a new [`IrqControl`](IrqControl) capability that stores its internal state in kernel allocated memory.
    pub fn init(&self, mem: &Capability, target_slot: &mut Capability) {
        assert_eq!(mem.tag, Tag::Memory);
        assert_eq!(target_slot.tag, Tag::Uninit);

        let allocator: &KernelAlloc = &mem.get_inner_memory().unwrap().allocator;

        // create a new zeroed capability state
        log::debug!(
            "needed = {}, available = {}",
            mem::size_of::<IrqControlState>(),
            allocator.get_free_bytes()
        );
        let mut state = Box::new_uninit(allocator).unwrap();
        IrqControlState::init(&mut state);
        let state = unsafe { state.assume_init() };

        // save the capability into the target slot
        target_slot.tag = Tag::IrqControl;
        target_slot.variant = Variant {
            irq_control: ManuallyDrop::new(IrqControl {
                state: CapCounted::from_box(unsafe { Box::ignore_lifetimes(state) }),
            }),
        };
    }
}

impl CapabilityIface<Capability> for IrqControlIface {
    type InitArgs = ();

    fn init(&self, target: &mut impl AsStaticMut<Capability>, args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
