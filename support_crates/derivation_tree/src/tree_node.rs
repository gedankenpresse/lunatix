use core::cell::Cell;
use core::marker::PhantomPinned;
use core::mem::MaybeUninit;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr;
use ctors::Ctor;

pub struct CursorSet<T> {
    cursor: Cursor<T>,
}

#[macro_export]
macro_rules! tree {
    ($name:ident, $root:ident = $ctor:expr) => {
        let mut $name = core::pin::pin!(CursorSet::new_uninit());
        emplace!(mut $root = $ctor);
        $name.as_mut().init();
        $name.as_mut().init_cursor_pin($root.as_mut());
    };
}

impl<T> CursorSet<T> {
    fn invalidate(self: Pin<&Self>, node: *const TreeNode<T>) {
        unsafe { Pin::new_unchecked(&self.cursor).invalidate(node) };
    }

    pub fn new_uninit() -> Self {
        Self {
            cursor: Cursor {
                prev: Cell::new(ptr::null()),
                next: Cell::new(ptr::null()),
                position: Cell::new(CursorPosition::Uninit),
                _pin: PhantomPinned,
            },
        }
    }

    pub fn init(self: Pin<&mut Self>) {
        let cursor_ptr =
            unsafe { &Pin::into_inner_unchecked(self.as_ref()).cursor as *const Cursor<_> };
        self.cursor.next.set(cursor_ptr);
        self.cursor.prev.set(cursor_ptr);
    }

    pub fn init_cursor(self: Pin<&mut Self>, root_ptr: *mut TreeNode<T>) {
        self.cursor.position.set(CursorPosition::Loc(root_ptr))
    }

    pub fn init_cursor_pin(mut self: Pin<&mut Self>, mut root: Pin<&mut TreeNode<T>>) {
        let root_ptr = unsafe { Pin::into_inner_unchecked(root.as_mut()) as *mut TreeNode<T> };
        unsafe {
            Pin::into_inner_unchecked(root).collection =
                Pin::into_inner_unchecked(self.as_mut()) as *const _
        };
        self.cursor.position.set(CursorPosition::Loc(root_ptr));
    }

    pub fn uninit_cursor(self: Pin<&mut Self>) {
        let cursor_ptr = unsafe { &Pin::into_inner_unchecked(self.as_ref()).cursor as *const _ };
        assert_eq!(
            self.cursor.next.get(),
            cursor_ptr,
            "tried to uninit tree with active cursors"
        );
        assert_eq!(
            self.cursor.prev.get(),
            cursor_ptr,
            "tried to uninit tree with active cursors"
        );
        self.cursor.position.set(CursorPosition::Uninit);
    }

    pub fn root_cursor(self: Pin<&mut Self>) -> impl Ctor<Cursor<T>> + '_ {
        move |dest: Pin<&mut MaybeUninit<_>>| {
            let root_cursor = &self.cursor;
            unsafe {
                let dest_ptr = Pin::into_inner_unchecked(dest).as_mut_ptr();
                ptr::write(
                    dest_ptr,
                    Cursor {
                        prev: Cell::new(root_cursor as *const Cursor<T>),
                        next: Cell::new(root_cursor.next.get()),
                        position: Cell::new(CursorPosition::Loc(root_cursor.get_loc())),
                        _pin: PhantomPinned,
                    },
                );
                root_cursor.next.set(dest_ptr);
            };
        }
    }
}

/// A wrapper implementation of [`TreeNodeOps`] that simply holds an owned value.
#[derive(Debug)]
pub struct TreeNode<T> {
    pub value: T,
    prev: Cell<*const TreeNode<T>>,
    next: Cell<*const TreeNode<T>>,
    depth: Cell<usize>,
    collection: *const CursorSet<T>,
    _pin: PhantomPinned,
}

impl<T> TreeNode<T> {
    pub fn new(value: T) -> impl Ctor<Self> {
        move |dest: Pin<&mut MaybeUninit<TreeNode<T>>>| Self::init(value, dest)
    }

    pub fn new2(value: T) -> Self {
        Self {
            value,
            next: Cell::new(ptr::null()),
            prev: Cell::new(ptr::null()),
            depth: Cell::new(0),
            collection: ptr::null(),
            _pin: PhantomPinned::default(),
        }
    }

    pub fn new_root(value: T, col: *const CursorSet<T>) -> impl Ctor<Self> {
        move |dest: Pin<&mut MaybeUninit<_>>| Self::init_root(value, col, dest)
    }

    pub fn init(value: T, dest: Pin<&mut MaybeUninit<Self>>) {
        unsafe {
            let inner = Pin::into_inner_unchecked(dest);
            inner.write(Self {
                value,
                next: Cell::new(ptr::null()),
                prev: Cell::new(ptr::null()),
                depth: Cell::new(0),
                collection: ptr::null(),
                _pin: PhantomPinned::default(),
            });
        };
    }

    pub fn init_root(value: T, col: *const CursorSet<T>, dest: Pin<&mut MaybeUninit<Self>>) {
        unsafe {
            let inner = Pin::into_inner_unchecked(dest);
            inner.write(Self {
                value,
                next: Cell::new(ptr::null()),
                prev: Cell::new(ptr::null()),
                depth: Cell::new(0),
                collection: col,
                _pin: PhantomPinned::default(),
            });
        }
    }

    pub fn get(self: Pin<&Self>) -> Pin<&T> {
        unsafe { self.map_unchecked(|s| &s.value) }
    }

    pub fn get_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|s| &mut s.value) }
    }

    pub fn is_in_collection(self: Pin<&Self>) -> bool {
        return !self.collection.is_null();
    }

    /// Append into the tree as a sibling of self.
    ///
    /// # Panics
    /// This function panics if `other` is already in the tree.
    fn append_after(self: Pin<&Self>, other: Pin<&Self>) {
        // retrieve all relevant pointers
        let self_ptr = unsafe { Pin::into_inner_unchecked(self.as_ref()) as *const Self };
        let other_ptr = unsafe { Pin::into_inner_unchecked(other) as *const Self };
        let next_ptr = self.next.get();

        assert!(
            !other.is_in_collection(),
            "cannot append other link into the derivation tree because it is already part of it"
        );

        // actually perform the insertion
        if let Some(next) = unsafe { self.next.get().as_ref() } {
            next.prev.set(other_ptr);
        }
        other.next.set(next_ptr);
        self.next.set(other_ptr);
        other.prev.set(self_ptr);
        other.depth.set(self.depth.get())
    }

    /// Append into the tree as a child of self.
    ///
    /// # Panics
    /// This function panics if `other` is already in the tree.
    fn append_below(self: Pin<&Self>, other: Pin<&Self>) {
        self.append_after(other);
        let old = other.as_ref().depth.get();
        other.as_ref().depth.set(old + 1);
    }
}

impl<T> Drop for TreeNode<T> {
    fn drop(&mut self) {
        pinned_drop(unsafe { Pin::new_unchecked(self) });
        fn pinned_drop<T>(this: Pin<&mut TreeNode<T>>) {
            let prev_ptr = this.prev.get();
            let next_ptr = this.next.get();

            if let Some(prev) = unsafe { this.prev.get().as_ref() } {
                prev.next.set(next_ptr);
            }
            this.prev.set(ptr::null());
            if let Some(next) = unsafe { this.next.get().as_ref() } {
                next.prev.set(prev_ptr);
            }
            this.next.set(ptr::null());

            if let Some(coll) = unsafe { this.collection.as_ref() } {
                let coll = unsafe { Pin::new_unchecked(coll) };
                let this_ptr: *const TreeNode<T> =
                    unsafe { Pin::into_inner_unchecked(this) as *const TreeNode<T> };
                coll.invalidate(this_ptr);
            }
        }
    }
}

pub struct Cursor<T> {
    prev: Cell<*const Cursor<T>>,
    next: Cell<*const Cursor<T>>,
    position: Cell<CursorPosition<T>>,
    _pin: PhantomPinned,
}

enum CursorPosition<T> {
    Loc(*mut TreeNode<T>),
    Shared(*mut TreeNode<T>),
    Mut(*mut TreeNode<T>),
    Uninit,
}

impl<T> Copy for CursorPosition<T> {}
impl<T> Clone for CursorPosition<T> {
    fn clone(&self) -> Self {
        match *self {
            Self::Loc(ptr) => Self::Loc(ptr),
            Self::Shared(ptr) => Self::Shared(ptr),
            Self::Mut(ptr) => Self::Mut(ptr),
            Self::Uninit => Self::Uninit,
        }
    }
}

pub struct CursorRef<'a, T> {
    cursor: Pin<&'a mut Cursor<T>>,
    value: *mut TreeNode<T>,
}

impl<'a, T> Deref for CursorRef<'a, T> {
    type Target = TreeNode<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.value) }
    }
}

impl<'a, T> Drop for CursorRef<'a, T> {
    fn drop(&mut self) {
        self.cursor.position.set(CursorPosition::Loc(self.value));
    }
}

pub struct CursorMut<'a, T> {
    cursor: Pin<&'a mut Cursor<T>>,
    value: *mut TreeNode<T>,
}

impl<'a, T> Deref for CursorMut<'a, T> {
    type Target = TreeNode<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &(*self.value) }
    }
}

impl<'a, T> DerefMut for CursorMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut (*self.value) }
    }
}

impl<'a, T> Drop for CursorMut<'a, T> {
    fn drop(&mut self) {
        self.cursor.position.set(CursorPosition::Loc(self.value));
    }
}

impl<T> Cursor<T> {
    fn new() -> impl Ctor<Cursor<T>> {
        |dest: Pin<&mut MaybeUninit<Cursor<T>>>| Self::init(dest)
    }

    fn init(dest: Pin<&mut MaybeUninit<Self>>) {
        unsafe {
            let inner = Pin::into_inner_unchecked(dest);
            let self_ptr = inner.as_mut_ptr();
            inner.write(Self {
                prev: Cell::new(self_ptr),
                next: Cell::new(self_ptr),
                position: Cell::new(CursorPosition::Uninit),
                _pin: PhantomPinned,
            });
        };
    }

    fn get_loc(&self) -> *mut TreeNode<T> {
        match self.position.get() {
            CursorPosition::Loc(ptr) => ptr,
            _ => panic!(),
        }
    }

    pub fn get(self: Pin<&mut Self>) -> CursorRef<T> {
        let ptr = self.get_loc();
        // TODO: assert that there are no other mut cursors to the current ptr
        self.position.set(CursorPosition::Shared(ptr));
        CursorRef {
            cursor: self,
            value: ptr,
        }
    }

    pub fn get_mut(self: Pin<&mut Self>) -> CursorMut<T> {
        let ptr = self.get_loc();
        // TODO: assert that there are no other shared/mut cursors to the current ptr
        self.position.set(CursorPosition::Mut(ptr));
        CursorMut {
            cursor: self,
            value: ptr,
        }
    }

    fn invalidate(self: Pin<&Self>, node: *const TreeNode<T>) {
        let this = unsafe { Pin::into_inner_unchecked(self) as *const Cursor<T> };
        let mut pos = this;
        while {
            Self::invalidate_single(pos, node);
            pos = unsafe { (*pos).next.get() };
            pos != this
        } {}
    }

    fn invalidate_single(cursor: *const Cursor<T>, node: *const TreeNode<T>) {
        match unsafe { (*cursor).position.get() } {
            CursorPosition::Loc(ptr) => assert_ne!(
                ptr as *const _, node,
                "tried to invalidate node with active cursor"
            ),
            CursorPosition::Shared(ptr) => assert_ne!(
                ptr as *const _, node,
                "tried to invalidate node with active cursor"
            ),
            CursorPosition::Mut(ptr) => assert_ne!(
                ptr as *const _, node,
                "tried to invalidate node with active cursor"
            ),
            CursorPosition::Uninit => {}
        }
    }
}

impl<T> Drop for Cursor<T> {
    fn drop(&mut self) {
        let this = unsafe { Pin::new_unchecked(self) };
        let prev = this.prev.get();
        let next = this.next.get();
        unsafe {
            (*prev).next.set(next);
            (*next).prev.set(prev);
        }
    }
}

#[cfg(test)]
pub mod test {
    extern crate std;

    use super::{Cursor, CursorSet, TreeNode};
    use core::mem::MaybeUninit;
    use core::{pin, ptr};
    use ctors::{emplace, slot};
    use pin::{pin, Pin};

    #[test]
    fn test_linking_two_nodes() {
        emplace!(node1 = TreeNode::new(()));
        emplace!(node2 = TreeNode::new(()));
        // act
        node1.as_ref().append_after(node2.as_ref());

        // assert
        assert_eq!(node1.as_ref().prev.get(), ptr::null());
        assert_ne!(node1.as_ref().next.get(), ptr::null());
        assert_eq!(node2.as_ref().next.get(), ptr::null());
        assert_ne!(node2.as_ref().prev.get(), ptr::null());
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
        assert_eq!(node1.as_ref().next.get(), ptr::null());
        assert_eq!(node1.as_ref().prev.get(), ptr::null());
    }

    #[test]
    fn can_create_cursor() {
        emplace!(cursor = Cursor::<TreeNode<()>>::new());
    }

    #[test]
    fn can_create_tree() {
        let mut cursor_set: CursorSet<TreeNode<()>> = CursorSet::new_uninit();
        let cursor_set = pin!(cursor_set);
        cursor_set.init();
    }
}
