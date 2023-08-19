use super::{Capability, KernelAlloc};
use crate::caps::{Memory, Tag, Variant};
use allocators::bump_allocator::BumpAllocator;
use core::mem;
use core::mem::ManuallyDrop;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::AsStaticMut;

#[derive(Copy, Clone)]
pub struct MemoryIface;

impl MemoryIface {
    /// Create a memory capability appropriate for the init task in the target slot.
    pub(crate) fn create_init(
        &self,
        target_slot: &mut Capability,
        alloc: &'static KernelAlloc,
    ) -> Result<(), super::Error> {
        // convert the remaining memory of the source allocator into a memory capability
        let mem_cap = unsafe {
            // TODO "Allocator first and then remaining bytes". Bene knows what he means by that
            Memory::alloc_new(
                alloc,
                alloc.get_free_bytes() - mem::size_of::<KernelAlloc>(),
                |mem| KernelAlloc::new(mem),
            )
        }
        .map_err(|_| super::Error::NoMem)?;

        // put it into the target slot
        target_slot.tag = Tag::Memory;
        target_slot.variant = Variant {
            memory: ManuallyDrop::new(mem_cap),
        };
        Ok(())
    }
}

impl CapabilityIface<Capability> for MemoryIface {
    type InitArgs = (Memory, usize);

    fn init(&self, target: &mut impl AsStaticMut<Capability>, args: Self::InitArgs) {
        todo!()
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl AsStaticMut<Capability>,
    ) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
