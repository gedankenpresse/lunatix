use crate::caps::{CapabilityIface, GetCapIface};
use crate::tree::TreeNodeOps;
use crate::{AsStaticMut, AsStaticRef};
use core::cell::Cell;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::ptr::addr_of_mut;
use core::{mem, ptr};

const SET_SIZE: usize = 8;

pub struct CursorSet<T: TreeNodeOps> {
    cursors: [Cursor<T>; SET_SIZE],
}

impl<T: TreeNodeOps> CursorSet<T> {
    pub(crate) unsafe fn init(loc: &mut MaybeUninit<Self>) {
        for i in 0..SET_SIZE {
            addr_of_mut!((*loc.as_mut_ptr()).cursors[i]).write(Cell::new(CursorData::Free))
        }
    }

    pub fn get_free_cursor(&self) -> Result<CursorHandle<T>, OutOfCursorsError> {
        for cursor in &self.cursors {
            if let CursorData::Free = cursor.get() {
                cursor.set(CursorData::Allocated);
                return Ok(CursorHandle {
                    source_set: self,
                    cursor,
                });
            }
        }

        Err(OutOfCursorsError)
    }

    pub fn is_empty(&self) -> bool {
        for cursor in &self.cursors {
            match cursor.get() {
                CursorData::Free => {}
                _ => return false,
            }
        }

        true
    }

    /// Check whether there is **any** cursor pointing to the given node.
    ///
    /// A cursor is considered to point to the node if it is in *Inactive*, *SharedRef* or *ExclusiveRef* state
    /// with the given node selected.
    pub fn exists_cursor_to(&self, node: *mut T) -> bool {
        self.cursors.iter().any(|cursor| match cursor.get() {
            CursorData::Inactive(c_node) => c_node == node,
            CursorData::SharedRef(c_node) => c_node == node,
            CursorData::ExclusiveRef(c_node) => c_node == node,
            _ => false,
        })
    }

    /// Check whether there is an **active** cursor pointing to the given node.
    ///
    /// A cursor is considered to be active if it is in *SharedRef* or *ExclusiveRef* state with the given node
    /// selected.
    pub(crate) fn exists_active_cursor_to(&self, node: *mut T) -> bool {
        self.cursors.iter().any(|cursor| match cursor.get() {
            CursorData::SharedRef(c_node) => c_node == node,
            CursorData::ExclusiveRef(c_node) => c_node == node,
            _ => false,
        })
    }

    /// Check whether there is an **exclusive** cursor pointing to the given node.
    pub(crate) fn exists_exclusive_cursor_to(&self, node: *mut T) -> bool {
        self.cursors.iter().any(|cursor| match cursor.get() {
            CursorData::ExclusiveRef(c_node) => c_node == node,
            _ => false,
        })
    }

    /// Get an iterator over the cursors contained in this set
    pub(crate) fn cursor_iter(&self) -> impl Iterator<Item=&Cursor<T>> {
        self.cursors.iter()
    }
}

pub type Cursor<T> = Cell<CursorData<T>>;

#[derive(Debug)]
pub enum CursorData<T: TreeNodeOps> {
    /// The cursor is currently unused and can be given out to consumers.
    Free,
    /// The cursor is used by a consumer but not yet set to a specific TreeNode.
    Allocated,
    /// The cursor is given out to a consumer and has been assigned to a specific TreeNode but that node
    /// has not yet been "locked" for access.
    Inactive(*mut T),
    /// The cursor represents a shared (`&`) reference to a specific TreeNode.
    SharedRef(*mut T),
    /// The cursor represents an exclusive (`&mut`) reference to a specific TreeNode.
    ExclusiveRef(*mut T),
}

impl<T: TreeNodeOps> CursorData<T> {
    fn get_ptr(&self) -> *mut T {
        match *self {
            CursorData::Inactive(ptr) => ptr,
            CursorData::SharedRef(ptr) => ptr,
            CursorData::ExclusiveRef(ptr) => ptr,
            _ => panic!("Cursor has no node selected"),
        }
    }
}

impl<T: TreeNodeOps> Copy for CursorData<T> {}

impl<T: TreeNodeOps> Clone for CursorData<T> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A handle to a cursor of a DerivationTree.
///
/// It can be used to safely access a selected node via [`get_shared()`](CursorHandle::get_shared) for shared
/// (`&`) references or [`get_exclusive()`](CursorHandle::get_exclusive) for exclusive and mutable (`&mut`) references.
pub struct CursorHandle<'cursor_set, T: TreeNodeOps> {
    pub(crate) cursor: &'cursor_set Cursor<T>,
    pub(crate) source_set: &'cursor_set CursorSet<T>,
}

impl<'cursor_set, T: TreeNodeOps> CursorHandle<'cursor_set, T> {
    /// Make this cursor point to the given node.
    ///
    /// # Panics
    /// This method panics if the given node is not part of the same derivation tree as the cursor.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn select_node(&mut self, node: *mut T) {
        assert_eq!(
            mem::discriminant(&self.cursor.get()),
            mem::discriminant(&CursorData::Allocated),
            "Cursor is not in a state where a node can be selected"
        );
        assert_eq!(
            unsafe { &*node }.get_tree_data().cursors.get() as *const _,
            self.source_set as *const _,
            "Cursor cannot point to a node from a different Tree"
        );

        self.cursor.set(CursorData::Inactive(node));
    }

    /// Try to get a shared reference (`&`) to this cursors node.
    ///
    /// The returned [`CursorRef`] implements `Deref` to access the nodes content.
    pub fn get_shared(&mut self) -> Result<CursorRef<'_, 'cursor_set, T>, AliasingError> {
        assert_eq!(
            mem::discriminant(&self.cursor.get()),
            mem::discriminant(&CursorData::Inactive(ptr::null_mut())),
            "Cursor has no node selected"
        );

        if self
            .source_set
            .exists_exclusive_cursor_to(self.cursor.get().get_ptr())
        {
            Err(AliasingError)
        } else {
            self.cursor
                .set(CursorData::SharedRef(self.cursor.get().get_ptr()));
            Ok(CursorRef {
                source_handle: self,
            })
        }
    }

    /// Try to get an exclusive reference (`&mut`) to this cursors node.
    ///
    /// The returned [`CursorRefMut`] implements `DerefMut` to access the nodes content.
    pub fn get_exclusive(&mut self) -> Result<CursorRefMut<'_, 'cursor_set, T>, AliasingError> {
        assert_eq!(
            mem::discriminant(&self.cursor.get()),
            mem::discriminant(&CursorData::Inactive(ptr::null_mut())),
            "Cursor is not in a state where a reference can be extracted"
        );

        if self
            .source_set
            .exists_active_cursor_to(self.cursor.get().get_ptr())
        {
            Err(AliasingError)
        } else {
            self.cursor
                .set(CursorData::ExclusiveRef(self.cursor.get().get_ptr()));
            Ok(CursorRefMut {
                source_handle: self,
            })
        }
    }

    /// Duplicate this cursor by allocating a new one from the DerivationTree that points to the same node as this one.
    pub fn duplicate(source: &Self) -> Result<Self, OutOfCursorsError> {
        let mut target_cursor = source.source_set.get_free_cursor()?;

        match &source.cursor.get() {
            CursorData::Free => unreachable!("users should never be able to obtain a free cursor"),
            CursorData::Allocated => {}
            _ => target_cursor.select_node(source.cursor.get().get_ptr()),
        };

        Ok(target_cursor)
    }
}

impl<T: TreeNodeOps + GetCapIface> CursorHandle<'_, T> {
    /// Destroy the selected capability node.
    pub fn destroy_cap(self) {
        // get node data from cursor before dropping it
        let node = unsafe { &mut *self.cursor.get().get_ptr() };
        let iface = node.get_capability_iface();

        // drop the cursor so that nothing points to the node before destroying it
        self.cursor.set(CursorData::Free);
        iface.destroy(node)
    }
}

impl<T: TreeNodeOps> Drop for CursorHandle<'_, T> {
    fn drop(&mut self) {
        self.cursor.set(CursorData::Free)
    }
}

/// A shared reference (`&`) to a node of the tree.
///
/// This type implements `Deref` for accessing the actual node.
pub struct CursorRef<'handle, 'cursor_set, T: TreeNodeOps> {
    /// The CursorHandle from which this reference was obtained
    source_handle: &'handle mut CursorHandle<'cursor_set, T>,
}

impl<T: TreeNodeOps> CursorRef<'_, '_, T> {
    /// Duplicate the underlying cursor of this reference.
    ///
    /// See [`CursorHandle::duplicate()`](CursorHandle::duplicate) for details.
    pub fn duplicate(source: &Self) -> Result<CursorHandle<T>, OutOfCursorsError> {
        CursorHandle::duplicate(source.source_handle)
    }
}

unsafe impl<T: TreeNodeOps> AsStaticRef<T> for CursorRef<'_, '_, T> {
    fn as_static_ref(&self) -> &'static T {
        unsafe { &*self.source_handle.cursor.get().get_ptr() }
    }
}

impl<T: TreeNodeOps> Deref for CursorRef<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.source_handle.cursor.get().get_ptr() }
    }
}

impl<T: TreeNodeOps> Drop for CursorRef<'_, '_, T> {
    fn drop(&mut self) {
        self.source_handle.cursor.set(CursorData::Inactive(
            self.source_handle.cursor.get().get_ptr(),
        ));
    }
}

/// An exclusive / mutable reference (`&mut`) to a node of the tree.
///
/// This type implements `DerefMut` for accessing the actual node.
pub struct CursorRefMut<'handle, 'cursor_set, T: TreeNodeOps> {
    /// The CursorHandle from which this reference was obtained
    source_handle: &'handle mut CursorHandle<'cursor_set, T>,
}

impl<T: TreeNodeOps> CursorRefMut<'_, '_, T> {
    /// Duplicate the underlying cursor handle.
    ///
    /// See [`CursorHandle::duplicate()`](CursorHandle::duplicate) for details.
    pub fn duplicate(source: &Self) -> Result<CursorHandle<T>, OutOfCursorsError> {
        CursorHandle::duplicate(source.source_handle)
    }
}

unsafe impl<T: TreeNodeOps> AsStaticRef<T> for CursorRefMut<'_, '_, T> {
    fn as_static_ref(&self) -> &'static T {
        unsafe { &*self.source_handle.cursor.get().get_ptr() }
    }
}

unsafe impl<T: TreeNodeOps> AsStaticMut<T> for CursorRefMut<'_, '_, T> {
    fn as_static_mut(&mut self) -> &'static mut T {
        unsafe { &mut *self.source_handle.cursor.get().get_ptr() }
    }
}

impl<T: TreeNodeOps> Deref for CursorRefMut<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.source_handle.cursor.get().get_ptr() }
    }
}

impl<T: TreeNodeOps> DerefMut for CursorRefMut<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.source_handle.cursor.get().get_ptr() }
    }
}

impl<T: TreeNodeOps> Drop for CursorRefMut<'_, '_, T> {
    fn drop(&mut self) {
        self.source_handle.cursor.set(CursorData::Inactive(
            self.source_handle.cursor.get().get_ptr(),
        ));
    }
}

/// The error type for when it is attempted to get overlapping handles on a tree node.
#[derive(Debug)]
pub struct AliasingError;

/// The error type for when no cursors are available for use anymore but one was requested.
#[derive(Debug)]
pub struct OutOfCursorsError;

#[cfg(test)]
mod test {
    extern crate std;

    use crate::test::assume_init_box;
    use crate::test::node_tests::TestNode;
    use crate::tree::cursors::{CursorData, CursorSet};
    use core::mem;
    use core::mem::MaybeUninit;
    use std::boxed::Box;

    #[test]
    fn test_allocate_two_cursors() {
        // arrange
        let mut loc = Box::new(MaybeUninit::<CursorSet<TestNode>>::uninit());
        let set = unsafe {
            CursorSet::init(&mut loc);
            assume_init_box(loc)
        };

        // act
        let cursor1 = set.get_free_cursor();
        let cursor2 = set.get_free_cursor();

        // assert
        assert!(cursor1.is_ok());
        assert!(cursor2.is_ok());
        assert_eq!(
            mem::discriminant(&cursor1.unwrap().cursor.get()),
            mem::discriminant(&CursorData::Allocated)
        );
        assert_eq!(
            mem::discriminant(&cursor2.unwrap().cursor.get()),
            mem::discriminant(&CursorData::Allocated)
        );
    }

    #[test]
    fn test_cursor_dropping() {
        // arrange
        let mut loc = Box::new(MaybeUninit::<CursorSet<TestNode>>::uninit());
        let set = unsafe {
            CursorSet::init(&mut loc);
            assume_init_box(loc)
        };

        // act
        assert!(set.is_empty());
        let cursor1 = set.get_free_cursor();
        assert!(!set.is_empty());
        drop(cursor1);

        // assert
        assert!(set.is_empty());
    }
}
