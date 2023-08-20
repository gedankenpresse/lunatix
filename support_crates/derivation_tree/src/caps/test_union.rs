//! Capability interface implementation for testing purposes

#![allow(unused_variables)]

use crate::caps::{CSpace, CapabilityIface, GetCapIface, Memory, Uninit};
use crate::tree::{TreeNodeData, TreeNodeOps};
use crate::{AsStaticMut, AsStaticRef, Correspondence};
use allocators::bump_allocator::{BumpAllocator, ForwardBumpingAllocator};
use core::mem::ManuallyDrop;
use core::ops::DerefMut;

/// A Union type for bundling all builtin + test capabilities together
pub struct TestCapUnion {
    pub tag: TestCapTag,
    pub tree_data: TreeNodeData<Self>,
    pub payload: TestCapPayload,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TestCapTag {
    Uninit,
    CSpace,
    Memory,
    UsizeValue,
}

pub union TestCapPayload {
    pub uninit: ManuallyDrop<Uninit>,
    pub cspace: ManuallyDrop<CSpace<'static, 'static, TestCapUnion>>,
    pub memory: ManuallyDrop<Memory<'static, 'static, ForwardBumpingAllocator<'static>>>,
    pub usize_value: ManuallyDrop<ValueCap<usize>>,
}

impl TestCapPayload {
    pub fn new_uninit() -> Self {
        Self {
            uninit: ManuallyDrop::new(Uninit),
        }
    }
}

impl TestCapUnion {
    /// Whether a cursor to this node exists
    fn exists_cursor_to_self(&mut self) -> bool {
        let self_ptr = self as *mut _;
        self.tree_data.get_cursors().exists_cursor_to(self_ptr)
    }
}

impl TreeNodeOps for TestCapUnion {
    fn get_tree_data(&self) -> &TreeNodeData<Self> {
        &self.tree_data
    }
}

impl GetCapIface for TestCapUnion {
    type IfaceImpl = TestCapTag;

    fn get_capability_iface(&self) -> Self::IfaceImpl {
        self.tag
    }
}

impl Correspondence for TestCapUnion {
    fn corresponds_to(&self, other: &Self) -> bool {
        unsafe {
            match (&self.tag, &other.tag) {
                (TestCapTag::Uninit, TestCapTag::Uninit) => {
                    self.payload.uninit.corresponds_to(&other.payload.uninit)
                }
                (TestCapTag::CSpace, TestCapTag::CSpace) => {
                    self.payload.cspace.corresponds_to(&other.payload.cspace)
                }
                (TestCapTag::UsizeValue, TestCapTag::UsizeValue) => self
                    .payload
                    .usize_value
                    .corresponds_to(&other.payload.usize_value),
                (TestCapTag::Memory, TestCapTag::Memory) => {
                    self.payload.memory.corresponds_to(&other.payload.memory)
                }
                _ => false,
            }
        }
    }
}

impl Default for TestCapUnion {
    fn default() -> Self {
        Self {
            tag: TestCapTag::Uninit,
            tree_data: unsafe { TreeNodeData::new() },
            payload: TestCapPayload {
                uninit: ManuallyDrop::new(Uninit),
            },
        }
    }
}

impl CapabilityIface<TestCapUnion> for TestCapTag {
    type InitArgs = ();

    fn init(&self, target: &mut impl AsStaticMut<TestCapUnion>, args: Self::InitArgs) {
        unimplemented!()
    }

    fn copy(&self, src: &impl AsStaticRef<TestCapUnion>, dst: &mut impl AsStaticMut<TestCapUnion>) {
        match self {
            TestCapTag::Uninit => UninitIface.copy(src, dst),
            TestCapTag::CSpace => CSpaceIface.copy(src, dst),
            TestCapTag::UsizeValue => ValueCapIface.copy(src, dst),
            TestCapTag::Memory => MemoryIface.copy(src, dst),
        }
    }

    fn destroy(&self, target: &mut TestCapUnion) {
        match self {
            TestCapTag::Uninit => UninitIface.destroy(target),
            TestCapTag::CSpace => CSpaceIface.destroy(target),
            TestCapTag::UsizeValue => ValueCapIface.destroy(target),
            TestCapTag::Memory => MemoryIface.destroy(target),
        }
    }
}

/// Value holding capability for testing purposes
pub struct ValueCap<T> {
    value: T,
}

pub struct ValueCapIface;

impl CapabilityIface<TestCapUnion> for ValueCapIface {
    type InitArgs = usize;

    fn init(&self, target: &mut impl AsStaticMut<TestCapUnion>, args: Self::InitArgs) {
        let target = target.as_static_mut();
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);

        target.tag = TestCapTag::UsizeValue;
        target.payload = TestCapPayload {
            usize_value: ManuallyDrop::new(ValueCap { value: args }),
        }
    }

    fn copy(&self, src: &impl AsStaticRef<TestCapUnion>, dst: &mut impl AsStaticMut<TestCapUnion>) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();

        // semantically copy the capability data
        dst.tag = TestCapTag::UsizeValue;
        dst.payload = TestCapPayload {
            usize_value: ManuallyDrop::new(ValueCap {
                value: unsafe { src.payload.usize_value.value },
            }),
        };

        // insert the new node into the tree
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut TestCapUnion) {
        assert!(
            !target.exists_cursor_to_self(),
            "Cannot destroy Capability because there is still a cursor using it"
        );

        // semantically destroy the node
        unsafe {
            target.payload.usize_value.deref_mut().value = 0usize;
            ManuallyDrop::drop(&mut target.payload.usize_value);
        }

        // remove this node from the tree
        target.tree_data.unlink();
        target.tag = TestCapTag::Uninit;
        target.payload = TestCapPayload::new_uninit();
    }
}

impl<T> Correspondence for ValueCap<T> {
    fn corresponds_to(&self, _other: &Self) -> bool {
        false
    }
}

pub struct UninitIface;

impl CapabilityIface<TestCapUnion> for UninitIface {
    type InitArgs = ();

    fn init(&self, target: &mut impl AsStaticMut<TestCapUnion>, args: Self::InitArgs) {
        let target = target.as_static_mut();
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);
    }

    fn copy(&self, src: &impl AsStaticRef<TestCapUnion>, dst: &mut impl AsStaticMut<TestCapUnion>) {
        assert_eq!(dst.as_static_mut().tag, TestCapTag::Uninit)
    }

    fn destroy(&self, target: &mut TestCapUnion) {
        assert!(
            !target.exists_cursor_to_self(),
            "Cannot destroy Capability because there is still a cursor using it"
        );
        panic!("Uninit capabilities should never be destroyed")
    }
}

pub struct CSpaceIface;

impl CapabilityIface<TestCapUnion> for CSpaceIface {
    type InitArgs = (&'static ForwardBumpingAllocator<'static>, usize);

    fn init(&self, target: &mut impl AsStaticMut<TestCapUnion>, args: Self::InitArgs) {
        let target = target.as_static_mut();
        let (allocator, num_slots) = args;
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);

        target.tag = TestCapTag::CSpace;
        target.payload = TestCapPayload {
            cspace: ManuallyDrop::new(CSpace::alloc_new(allocator, num_slots).unwrap()),
        };
    }

    fn copy(&self, src: &impl AsStaticRef<TestCapUnion>, dst: &mut impl AsStaticMut<TestCapUnion>) {
        todo!()
    }

    fn destroy(&self, target: &mut TestCapUnion) {
        assert!(
            !target.exists_cursor_to_self(),
            "Cannot destroy Capability because there is still a cursor using it"
        );

        // semantically destroy the CSpace by deallocating the backing memory
        if target.is_final_copy() {
            unsafe {
                target.payload.cspace.deref_mut().deallocate();
            };
        }
        unsafe { ManuallyDrop::drop(&mut target.payload.cspace) }

        // remove the node from the tree
        target.tree_data.unlink();
        target.tag = TestCapTag::Uninit;
        target.payload = TestCapPayload::new_uninit();
    }
}

pub struct MemoryIface;

impl MemoryIface {
    /// Derive the desired capability from this memory capability (`mem`) and store it in `target`.
    pub fn derive(
        &self,
        mem: &impl AsStaticRef<TestCapUnion>,
        target: &mut impl AsStaticMut<TestCapUnion>,
        target_capability: TestCapTag,
        size_if_applicable: usize,
    ) {
        let mem = mem.as_static_ref();

        // initialize the target nodes memory
        {
            assert_eq!(target.as_static_mut().tag, TestCapTag::Uninit);

            match target_capability {
                TestCapTag::Uninit => panic!("uninit cannot be derived"),
                TestCapTag::CSpace => {
                    CSpaceIface.init(
                        target,
                        (
                            &unsafe { &mem.payload.memory }.allocator,
                            size_if_applicable,
                        ),
                    );
                }
                TestCapTag::Memory => unimplemented!(),
                TestCapTag::UsizeValue => unimplemented!(),
            }
        }

        // insert the node into the tree
        let target = target.as_static_mut();
        assert_eq!(target.tag, target_capability);
        unsafe { mem.insert_derivation(target) };
    }

    /// Destroy all capabilities that were derived from this node
    pub fn revoke(&self, target: &mut TestCapUnion) {
        // obtain the last copy of target so that the next pointer is a child node
        let mut cursor = target.tree_data.get_cursors().get_free_cursor().unwrap();
        let last_copy_ptr = unsafe { target.get_last_copy() };
        cursor.select_node(last_copy_ptr);
        let last_copy = cursor.get_exclusive().unwrap();

        // while there are children, destroy them
        while last_copy.has_derivations() {
            let mut children_cursor = target.tree_data.get_cursors().get_free_cursor().unwrap();
            children_cursor.select_node(last_copy.tree_data.next.get());
            let mut child_handle = children_cursor.get_exclusive().unwrap();
            match child_handle.tag {
                TestCapTag::Uninit => UninitIface.destroy(&mut child_handle),
                TestCapTag::CSpace => CSpaceIface.destroy(&mut child_handle),
                TestCapTag::Memory => MemoryIface.destroy(&mut child_handle),
                TestCapTag::UsizeValue => ValueCapIface.destroy(&mut child_handle),
            }
        }
    }
}

impl CapabilityIface<TestCapUnion> for MemoryIface {
    type InitArgs = (&'static ForwardBumpingAllocator<'static>, usize);

    fn init(&self, target: &mut impl AsStaticMut<TestCapUnion>, args: Self::InitArgs) {
        let target = target.as_static_mut();
        let (allocator, size) = args;
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);

        let instance = unsafe {
            Memory::alloc_new(allocator, size, |mem| ForwardBumpingAllocator::new(mem)).unwrap()
        };

        target.tag = TestCapTag::Memory;
        target.payload = TestCapPayload {
            memory: ManuallyDrop::new(instance),
        }
    }

    fn copy(&self, src: &impl AsStaticRef<TestCapUnion>, dst: &mut impl AsStaticMut<TestCapUnion>) {
        todo!()
    }

    fn destroy(&self, target: &mut TestCapUnion) {
        assert!(
            !target.exists_cursor_to_self(),
            "Cannot destroy Capability because there is still a cursor using it"
        );

        // semantically destroy the capability
        {
            if target.is_final_copy() {
                self.revoke(target);
                unsafe {
                    target.payload.memory.deref_mut().deallocate();
                }
            }
        }
        unsafe {
            ManuallyDrop::drop(&mut target.payload.memory);
        }

        // remove the node from the tree
        target.tree_data.unlink();
        target.tag = TestCapTag::Uninit;
        target.payload = TestCapPayload::new_uninit();
    }
}
