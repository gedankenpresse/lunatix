use crate::caps::{CSpace, CapabilityIface, Uninit};
use crate::{Correspondence, TreeNodeData, TreeNodeOps};
use allocators::bump_allocator::ForwardBumpingAllocator;
use core::mem::ManuallyDrop;

/// A Union type for bundling all builtin + test capabilities together
pub struct TestCapUnion {
    pub tag: TestCapTag,
    pub tree_data: TreeNodeData<Self>,
    pub payload: TestCapPayload,
}

#[derive(Debug, Eq, PartialEq)]
pub enum TestCapTag {
    Uninit,
    CSpace,
    UsizeValue,
}

pub union TestCapPayload {
    pub uninit: ManuallyDrop<Uninit>,
    pub cspace:
        ManuallyDrop<CSpace<'static, 'static, ForwardBumpingAllocator<'static>, TestCapUnion>>,
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
        }
    }

    fn destroy(&self, target: &TestCapUnion) {
        match self {
            TestCapTag::Uninit => UninitIface.destroy(target),
            TestCapTag::CSpace => CSpaceIface.destroy(target),
            TestCapTag::UsizeValue => ValueCapIface.destroy(target),
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
