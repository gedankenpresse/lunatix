//! Abstraction library for working with capability derivations

#![no_std]

extern crate alloc;

mod correspondence;
mod cspace;
mod cursors;
mod node;
mod tree;
mod uninit;

pub use correspondence::{CapabilityOps, Correspondence};
pub use cursors::{AliasingError, CursorHandle, CursorSet, OutOfCursorsError};
pub use node::{TreeNodeData, TreeNodeOps};
pub use tree::DerivationTree;

#[cfg(test)]
pub(crate) use test::assume_init_box;

#[cfg(test)]
mod test {
    extern crate std;

    use alloc::boxed::Box as StdBox;
    use core::mem::MaybeUninit;

    pub unsafe fn assume_init_box<T>(value: StdBox<MaybeUninit<T>>) -> StdBox<T> {
        let raw = StdBox::into_raw(value);
        StdBox::from_raw(raw as *mut T)
    }

    pub mod node_tests {
        use crate::{CapabilityOps, Correspondence, TreeNodeData, TreeNodeOps};
        use core::mem::MaybeUninit;

        pub struct TestNode {
            pub tree_data: TreeNodeData<Self>,
            pub value: usize,
        }

        impl TestNode {
            pub fn new(value: usize) -> Self {
                Self {
                    tree_data: unsafe { TreeNodeData::new() },
                    value,
                }
            }
        }

        impl TreeNodeOps for TestNode {
            fn get_tree_data(&self) -> &TreeNodeData<Self> {
                &self.tree_data
            }
        }

        impl Correspondence for TestNode {
            fn corresponds_to(&self, other: &Self) -> bool {
                false
            }
        }

        impl CapabilityOps for TestNode {
            fn cap_copy(source: &Self, dest: &mut MaybeUninit<Self>) {
                todo!()
            }

            fn destroy(&self) {}
        }
    }

    pub mod full_capability_tests {
        use crate::cspace::CSpace;
        use crate::test::assume_init_box;
        use crate::uninit::Uninit;
        use crate::{CapabilityOps, Correspondence, DerivationTree, TreeNodeData, TreeNodeOps};
        use alloc::boxed::Box as StdBox;
        use alloc::vec;
        use alloc::vec::Vec;
        use allocators::bump_allocator::BumpAllocator;
        use allocators::Allocator;
        use core::mem::{ManuallyDrop, MaybeUninit};
        use core::ptr::addr_of_mut;

        /// A dummy capability that holds a value of type `V`
        pub struct ValueCap<C: TreeNodeOps, V: Clone> {
            tree_data: TreeNodeData<C>,
            value: V,
        }

        impl<C: TreeNodeOps, V: Clone> ValueCap<C, V> {
            pub fn new(value: V) -> Self {
                Self {
                    tree_data: unsafe { TreeNodeData::new() },
                    value,
                }
            }
        }

        impl<C: TreeNodeOps, V: Clone> Correspondence for ValueCap<C, V> {
            fn corresponds_to(&self, other: &Self) -> bool {
                false
            }
        }

        impl<C: TreeNodeOps, V: Clone> CapabilityOps for ValueCap<C, V> {
            fn cap_copy(source: &Self, dest: &mut MaybeUninit<Self>) {
                unsafe { dest.write(ValueCap::new(source.value.clone())) };
            }

            fn destroy(&self) {
                // no-op
            }
        }

        /// An enum-like collection of all possible capability types
        #[repr(C)]
        pub struct TestCapCollection<'alloc, 'mem, A: Allocator<'mem>> {
            tag: TestCapTag,
            payload: TestCapPayload<'alloc, 'mem, A, Self>,
        }

        #[repr(u8)]
        enum TestCapTag {
            CSpace,
            UsizeCap,
            Uninit,
        }

        #[repr(C)]
        union TestCapPayload<'alloc, 'mem, A: Allocator<'mem>, C: TreeNodeOps> {
            cspace: ManuallyDrop<CSpace<'alloc, 'mem, A, C>>,
            usize_cap: ManuallyDrop<ValueCap<C, usize>>,
            uninit: ManuallyDrop<Uninit<C>>,
        }

        impl<'mem, A: Allocator<'mem>> TestCapCollection<'_, 'mem, A> {
            fn init_cap(&mut self, value: Self) {
                match self.tag {
                    TestCapTag::Uninit => unsafe { (self as *mut Self).write(value) },
                    _ => panic!("only uninit caps can be initialized"),
                }
            }
        }

        impl<'mem, A: Allocator<'mem>> Default for TestCapCollection<'_, 'mem, A> {
            fn default() -> Self {
                Self {
                    tag: TestCapTag::Uninit,
                    payload: TestCapPayload {
                        uninit: ManuallyDrop::new(Uninit::new()),
                    },
                }
            }
        }

        impl<'mem, A: Allocator<'mem>> TreeNodeOps for TestCapCollection<'_, 'mem, A> {
            fn get_tree_data(&self) -> &TreeNodeData<Self> {
                match self.tag {
                    TestCapTag::CSpace => unsafe { &self.payload.cspace.tree_data },
                    TestCapTag::UsizeCap => unsafe { &self.payload.usize_cap.tree_data },
                    TestCapTag::Uninit => unsafe { &self.payload.uninit.tree_data },
                }
            }
        }

        impl<'mem, A: Allocator<'mem>> Correspondence for TestCapCollection<'_, 'mem, A> {
            fn corresponds_to(&self, other: &Self) -> bool {
                match (&self.tag, &other.tag) {
                    (TestCapTag::CSpace, TestCapTag::CSpace) => todo!(),
                    (TestCapTag::Uninit, TestCapTag::Uninit) => todo!(),
                    (TestCapTag::UsizeCap, TestCapTag::UsizeCap) => todo!(),
                    _ => false,
                }
            }
        }

        impl<'mem, A: Allocator<'mem>> CapabilityOps for TestCapCollection<'_, 'mem, A> {
            fn cap_copy(source: &Self, dest: &mut MaybeUninit<Self>) {
                match &source.tag {
                    TestCapTag::CSpace => unimplemented!(),
                    TestCapTag::Uninit => unimplemented!(),
                    TestCapTag::UsizeCap => {
                        unsafe {
                            addr_of_mut!((*dest.as_mut_ptr()).tag).write(TestCapTag::UsizeCap);
                            ValueCap::cap_copy(
                                &source.payload.usize_cap,
                                &mut *(addr_of_mut!((*dest.as_mut_ptr()).payload)
                                    as *mut MaybeUninit<TestCapPayload<A, Self>>
                                    as *mut MaybeUninit<ValueCap<Self, usize>>),
                            )
                        }
                        todo!()
                    }
                }
            }

            fn destroy(&self) {
                todo!()
            }
        }

        #[test]
        fn full_tree_with_cspaces() {
            // arrange
            use allocators::bump_allocator::ForwardBumpingAllocator;
            type Cap<'alloc, 'mem> = TestCapCollection<'alloc, 'mem, ForwardBumpingAllocator<'mem>>;
            const BYTES: usize = 4096;
            let mut mem: Vec<u8> = vec![0; BYTES];
            let allocator = StdBox::new(ForwardBumpingAllocator::new(&mut mem[..]));
            let mut loc = StdBox::new(MaybeUninit::uninit());

            // act
            // initialize a tree with a CSpace node as root
            let tree = unsafe {
                DerivationTree::init_with_root_value(&mut loc, Cap::default());
                assume_init_box(loc)
            };
            let mut cursor = tree.get_root_cursor().unwrap();
            let mut cspace_cap = cursor.get_exclusive().unwrap();
            cspace_cap.init_cap(TestCapCollection {
                tag: TestCapTag::CSpace,
                payload: TestCapPayload {
                    cspace: ManuallyDrop::new(CSpace::alloc_new(&*allocator, 4).unwrap()),
                },
            });
            if let TestCapTag::CSpace = cspace_cap.tag {
                unsafe {
                    // create a new UsizeCap and store it as a derivation of the CSpace (this semantically does not make sense but we want to test)
                    let usize_cap = &mut *cspace_cap.payload.cspace.lookup_raw(0).unwrap();
                    usize_cap.init_cap(TestCapCollection {
                        tag: TestCapTag::UsizeCap,
                        payload: TestCapPayload {
                            usize_cap: ManuallyDrop::new(ValueCap::new(42)),
                        },
                    });
                    cspace_cap.insert_derivation(usize_cap);

                    // copy the UsizeCap
                    let usize_cap2 = &mut *(cspace_cap.payload.cspace.lookup_raw(1).unwrap()
                        as *mut MaybeUninit<Cap>);
                    TestCapCollection::cap_copy(usize_cap, usize_cap2);
                    usize_cap.insert_copy(usize_cap2.assume_init_mut());
                }
            }

            // assert
        }
    }
}
