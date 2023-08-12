use crate::{correspondence::Correspondence, CapabilityOps, TreeNodeData, TreeNodeOps};
use core::mem::MaybeUninit;

pub struct Uninit<T: TreeNodeOps> {
    pub tree_data: TreeNodeData<T>,
}

impl<T: TreeNodeOps> Uninit<T> {
    pub fn new() -> Self {
        Self {
            tree_data: unsafe { TreeNodeData::new() },
        }
    }
}

impl<T: TreeNodeOps> Correspondence for Uninit<T> {
    fn corresponds_to(&self, other: &Self) -> bool {
        false
    }
}

impl<T: TreeNodeOps> CapabilityOps for Uninit<T> {
    fn cap_copy(source: &Self, dest: &mut MaybeUninit<Self>) {
        todo!()
    }

    fn destroy(&self) {}
}
