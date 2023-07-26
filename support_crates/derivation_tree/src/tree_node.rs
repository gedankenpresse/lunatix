use core::cell::Cell;
use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::{mem, ptr};
use ctors::Ctor;

/// The links to other nodes of a tree as well as depth encoding.
///
/// See the module documentation about how this is enough information to model the whole tree
/// structure.
pub struct TreeLinks<T>
where
    T: TreeNodeOps,
{
    prev: Cell<*const T>,
    next: Cell<*const T>,

    /// The 1-indexed depth of this element in the tree.
    ///
    /// Depth of 0 signals this node not being a member of any collection.
    depth: Cell<usize>,
}

impl<T: TreeNodeOps> TreeLinks<T> {
    /// Construct a new instance that is unlinked and has 0 depth
    fn new_unlinked() -> Self {
        Self {
            next: Cell::new(ptr::null()),
            prev: Cell::new(ptr::null()),
            depth: Cell::new(0),
        }
    }

    pub fn is_in_collection(&self) -> bool {
        self.depth.get() != 0
    }

    /// # Safety
    /// The structs type parameter `T` must be `!Unpin`
    unsafe fn get_next<'a>(&self) -> Option<Pin<&'a T>> {
        self.next
            .get()
            .as_ref()
            .map(|next| unsafe { Pin::new_unchecked(next) })
    }

    /// # Safety
    /// The structs type parameter `T` must be `!Unpin`
    unsafe fn get_prev<'a>(&self) -> Option<Pin<&'a T>> {
        self.prev
            .get()
            .as_ref()
            .map(|prev| unsafe { Pin::new_unchecked(prev) })
    }
}

impl<T: TreeNodeOps> Drop for TreeLinks<T> {
    fn drop(&mut self) {
        let prev_ptr = self.prev.get();
        let next_ptr = self.next.get();

        if let Some(prev) = unsafe { self.get_prev() } {
            prev.get_links().next.set(next_ptr);
        }
        self.prev.set(ptr::null());
        if let Some(next) = unsafe { self.get_next() } {
            next.get_links().prev.set(prev_ptr);
        }
        self.next.set(ptr::null());
    }
}

/// Operations that are supported by any node in the derivation tree.
///
/// # Safety
/// Self must be `!Unpin`
pub unsafe trait TreeNodeOps
where
    Self: Sized,
{
    /// Get the links object describing the placement of this node in the derivation tree.
    fn get_links(self: Pin<&Self>) -> Pin<&TreeLinks<Self>>;

    fn get_links_mut(self: Pin<&mut Self>) -> Pin<&mut TreeLinks<Self>>;

    /// Append into the tree as a sibling of self.
    ///
    /// # Panics
    /// This function panics if `other` is already in the tree.
    fn append_after(mut self: Pin<&Self>, other: Pin<&Self>) {
        // retrieve all relevant pointers
        let self_ptr = unsafe { Pin::into_inner_unchecked(self.as_ref()) as *const Self };
        let self_links = self.as_ref().get_links();
        let other_ptr = unsafe { Pin::into_inner_unchecked(other) as *const Self };
        let other_links = other.as_ref().get_links();
        let next_ptr = self_links.next.get();

        assert!(
            !other_links.is_in_collection(),
            "cannot append other link into the derivation tree because it is already part of it"
        );

        // actually perform the insertion
        if let Some(next) = unsafe { self_links.get_next() } {
            next.get_links().prev.set(other_ptr);
        }
        other_links.next.set(next_ptr);
        self_links.next.set(other_ptr);
        other_links.prev.set(self_ptr);
        other_links.depth.set(self_links.depth.get())
    }

    /// Append into the tree as a child of self.
    ///
    /// # Panics
    /// This function panics if `other` is already in the tree.
    fn append_below(self: Pin<&Self>, mut other: Pin<&Self>) {
        self.append_after(other);
        let old = other.as_ref().get_links().depth.get();
        other.as_ref().get_links().depth.set(old + 1);
    }
}

/// A wrapper implementation of [`TreeNodeOps`] that simply holds an owned value.
pub struct TreeNode<T> {
    value: T,
    links: TreeLinks<Self>,
    _pin: PhantomPinned,
}

impl<T> TreeNode<T> {
    pub fn new(value: T) -> impl Ctor<Self> {
        move |dest: Pin<&mut MaybeUninit<TreeNode<T>>>| Self::init(value, dest)
    }

    pub fn new2(value: T) -> Self {
        Self {
            value,
            links: TreeLinks::new_unlinked(),
            _pin: PhantomPinned::default(),
        }
    }

    pub fn init(value: T, dest: Pin<&mut MaybeUninit<Self>>) {
        unsafe {
            let inner = Pin::into_inner_unchecked(dest);
            inner.write(Self {
                value,
                links: TreeLinks::new_unlinked(),
                _pin: PhantomPinned::default(),
            });
        };
    }

    pub fn get(self: Pin<&Self>) -> Pin<&T> {
        unsafe { self.map_unchecked(|s| &s.value) }
    }

    pub fn get_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|s| &mut s.value) }
    }
}

unsafe impl<T> TreeNodeOps for TreeNode<T> {
    fn get_links(self: Pin<&Self>) -> Pin<&TreeLinks<Self>> {
        unsafe { self.map_unchecked(|s| &s.links) }
    }

    fn get_links_mut(self: Pin<&mut Self>) -> Pin<&mut TreeLinks<Self>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.links) }
    }
}

impl<T> Drop for TreeNode<T> {
    fn drop(&mut self) {
        let pinned_self = unsafe { Pin::new_unchecked(self) };

        let x = pinned_self.get_links_mut();
        mem::swap(x.get_mut(), &mut TreeLinks::new_unlinked());
    }
}

#[cfg(test)]
pub mod test {
    extern crate std;

    use crate::tree_node::{TreeNode, TreeNodeOps};
    use core::ptr;
    use ctors::{emplace, slot};

    #[test]
    fn test_linking_two_nodes() {
        // arrange
        emplace!(node1 = TreeNode::new(1));
        emplace!(node2 = TreeNode::new(2));

        // act
        node1.as_ref().append_after(node2.as_ref());

        // assert
        assert_eq!(node1.as_ref().get_links().prev.get(), ptr::null());
        assert_ne!(node1.as_ref().get_links().next.get(), ptr::null());
        assert_eq!(node2.as_ref().get_links().next.get(), ptr::null());
        assert_ne!(node2.as_ref().get_links().prev.get(), ptr::null());
    }

    #[test]
    fn test_dropping_node_after_link() {
        // arrange
        emplace!(node1 = TreeNode::new(1));

        // act
        {
            emplace!(node2 = TreeNode::new(2));
            drop(node2);
        }

        // assert
        assert_eq!(node1.as_ref().get_links().next.get(), ptr::null());
        assert_eq!(node1.as_ref().get_links().prev.get(), ptr::null());
    }
}
