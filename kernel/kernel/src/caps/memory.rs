use allocators::bump_allocator::BumpAllocator;
use core::mem;
use core::mem::ManuallyDrop;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef};

use crate::caps::Uninit;

use super::{Capability, KernelAlloc, Tag, Variant};
pub type Memory = derivation_tree::caps::Memory<'static, 'static, KernelAlloc>;

#[derive(Copy, Clone)]
pub struct MemoryIface;

impl MemoryIface {
    /// Create a memory capability appropriate for the init task in the target slot.
    pub(crate) fn create_init(
        &self,
        target_slot: &mut Capability,
        alloc: &'static KernelAlloc,
    ) -> Result<(), super::SyscallError> {
        assert_eq!(target_slot.tag, Tag::Uninit);
        // convert the remaining memory of the source allocator into a memory capability
        let mem_cap = unsafe {
            // TODO "Allocator first and then remaining bytes". Bene knows what he means by that
            Memory::alloc_new(
                alloc,
                alloc.get_free_bytes() - mem::size_of::<KernelAlloc>(),
                |mem| KernelAlloc::new(mem),
            )
        }
        .map_err(|_| super::SyscallError::NoMem)?;

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

    fn init(&self, _target: &mut impl AsStaticMut<Capability>, _args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::Memory);
        assert_eq!(dst.tag, Tag::Uninit);

        // semantically copy the cspace
        dst.tag = Tag::Memory;
        {
            let src_mem = src.get_inner_memory().unwrap();
            dst.variant = Variant {
                memory: ManuallyDrop::new(Memory {
                    allocator: src_mem.allocator.clone(),
                    backing_mem: src_mem.backing_mem.clone(),
                }),
            };
        }

        // insert the new copy into the derivation tree
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        assert_eq!(target.tag, Tag::Memory);

        if target.is_final_copy() {
            while target.has_derivations() {
                todo!("destroy children");
            }

            let mem = target.get_inner_memory_mut().unwrap();
            unsafe { mem.allocator.destroy() };
            unsafe { mem.backing_mem.destroy() };
        }

        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant.uninit = Uninit {};
    }
}
