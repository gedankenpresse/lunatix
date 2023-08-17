use crate::caps::{CSpace, CapabilityIface, Memory, Uninit};
use crate::{Correspondence, TreeNodeData, TreeNodeOps};
use allocators::bump_allocator::{BumpAllocator, ForwardBumpingAllocator};
use allocators::Allocator;
use core::mem::ManuallyDrop;
use core::ops::Deref;

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
    pub cspace:
        ManuallyDrop<CSpace<'static, 'static, ForwardBumpingAllocator<'static>, TestCapUnion>>,
    pub memory: ManuallyDrop<
        Memory<
            'static,
            'static,
            ForwardBumpingAllocator<'static>,
            ForwardBumpingAllocator<'static>,
        >,
    >,
    pub usize_value: ManuallyDrop<ValueCap<usize>>,
}

impl TreeNodeOps for TestCapUnion {
    fn get_tree_data(&self) -> &TreeNodeData<Self> {
        &self.tree_data
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

    fn init(&self, _target: &mut TestCapUnion, _args: Self::InitArgs) {
        unimplemented!()
    }

    fn copy(&self, src: &TestCapUnion, dst: &mut TestCapUnion) {
        match self {
            TestCapTag::Uninit => UninitIface.copy(src, dst),
            TestCapTag::CSpace => CSpaceIface.copy(src, dst),
            TestCapTag::UsizeValue => ValueCapIface.copy(src, dst),
            TestCapTag::Memory => MemoryIface.copy(src, dst),
        }
    }

    fn destroy(&self, target: &TestCapUnion) {
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

    fn init(&self, target: &mut TestCapUnion, args: Self::InitArgs) {
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);

        target.tag = TestCapTag::UsizeValue;
        target.payload = TestCapPayload {
            usize_value: ManuallyDrop::new(ValueCap { value: args }),
        }
    }

    fn copy(&self, src: &TestCapUnion, dst: &mut TestCapUnion) {
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

    fn destroy(&self, target: &TestCapUnion) {
        todo!()
    }
}

impl<T> Correspondence for ValueCap<T> {
    fn corresponds_to(&self, other: &Self) -> bool {
        false
    }
}

pub struct UninitIface;

impl CapabilityIface<TestCapUnion> for UninitIface {
    type InitArgs = ();

    fn init(&self, target: &mut TestCapUnion, _args: Self::InitArgs) {
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);
    }

    fn copy(&self, src: &TestCapUnion, dst: &mut TestCapUnion) {
        assert_eq!(dst.tag, TestCapTag::Uninit)
    }

    fn destroy(&self, target: &TestCapUnion) {
        // noop
    }
}

pub struct CSpaceIface;

impl CapabilityIface<TestCapUnion> for CSpaceIface {
    type InitArgs = (&'static ForwardBumpingAllocator<'static>, usize);

    fn init(&self, target: &mut TestCapUnion, args: Self::InitArgs) {
        let (allocator, num_slots) = args;
        assert!(!target.tree_data.is_linked());
        assert_eq!(target.tag, TestCapTag::Uninit);

        target.tag = TestCapTag::CSpace;
        target.payload = TestCapPayload {
            cspace: ManuallyDrop::new(CSpace::alloc_new(allocator, num_slots).unwrap()),
        };
    }

    fn copy(&self, src: &TestCapUnion, dst: &mut TestCapUnion) {
        todo!()
    }

    fn destroy(&self, target: &TestCapUnion) {
        todo!()
    }
}

pub struct MemoryIface;

impl MemoryIface {
    /// Derive the desired capability from this memory capability (`mem`) and store it in `target`.
    pub fn derive(
        &self,
        mem: &'static TestCapUnion,
        target: &'static mut TestCapUnion,
        target_capability: TestCapTag,
        size_if_applicable: usize,
    ) {
        assert_eq!(target.tag, TestCapTag::Uninit);

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

        assert_eq!(target.tag, target_capability);
        unsafe { mem.insert_derivation(target) };
    }
}

impl CapabilityIface<TestCapUnion> for MemoryIface {
    type InitArgs = (&'static ForwardBumpingAllocator<'static>, usize);

    fn init(&self, target: &mut TestCapUnion, args: Self::InitArgs) {
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

    fn copy(&self, src: &TestCapUnion, dst: &mut TestCapUnion) {
        todo!()
    }

    fn destroy(&self, target: &TestCapUnion) {
        todo!()
    }
}
