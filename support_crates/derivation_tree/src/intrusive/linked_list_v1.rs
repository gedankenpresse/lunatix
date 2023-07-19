use core::{
    cell::Cell,
    marker::{PhantomData, PhantomPinned},
    pin::Pin,
};

pub struct Link<T> {
    elem: T,
    prev: Cell<*mut Link<T>>,
    next: Cell<*mut Link<T>>,
    _pin: PhantomPinned,
}

pub struct LinkedList<'this, T> {
    head: *mut Link<T>,
    tail: *mut Link<T>,
    _self: PhantomData<&'this ()>,
}

pub struct Cursor<'a, T> {
    current: *mut Link<T>,
    list: &'a LinkedList<'a, T>,
}

impl<'a, T> Cursor<'a, T> {
    pub fn get(&self) -> Option<Pin<&'a T>> {
        match unsafe { Link::pointer_as_pin(self.current) } {
            Some(link) => Some(link.into_ref().value()),
            None => None,
        }
    }

    pub fn move_next(&mut self) {
        match unsafe { self.current.as_ref() } {
            Some(link) => {
                self.current = link.next.get();
            }
            None => {}
        }
    }

    pub fn move_prev(&mut self) {
        match unsafe { self.current.as_ref() } {
            Some(link) => {
                self.current = link.prev.get();
            }
            None => {}
        }
    }
}

pub struct CursorMut<'a, 'this, T> {
    current: *mut Link<T>,
    list: &'a mut LinkedList<'this, T>,
}

impl<'a, 'this, T> CursorMut<'a, 'this, T> {
    pub fn get(&self) -> Option<Pin<&'a T>> {
        match unsafe { Link::pointer_as_pin(self.current) } {
            Some(link) => Some(link.into_ref().value()),
            None => None,
        }
    }

    pub fn move_next(&mut self) {
        match unsafe { self.current.as_ref() } {
            Some(link) => {
                self.current = link.next.get();
            }
            None => {}
        }
    }

    pub fn move_prev(&mut self) {
        match unsafe { self.current.as_ref() } {
            Some(link) => {
                self.current = link.prev.get();
            }
            None => {}
        }
    }
    pub fn insert_before(&mut self, link: Pin<&mut Link<T>>) {
        if self.current == self.list.head {
            self.list.insert_front(link);
            if self.current.is_null() {
                self.current = self.list.head;
            }
        } else {
            let cur = unsafe { Link::pointer_as_pin(self.current).unwrap() };
            cur.as_ref().insert_before(link);
        }
    }

    pub fn insert_after(&mut self, link: Pin<&mut Link<T>>) {
        if self.current == self.list.tail {
            self.list.insert_back(link);
            if self.current.is_null() {
                self.current = self.list.tail;
            }
        } else {
            let cur = unsafe { Link::pointer_as_pin(self.current).unwrap() };
            cur.as_ref().insert_after(link);
        }
    }
}

impl<'a, T: 'a> Link<T> {
    pub fn new(value: T) -> Link<T> {
        use core::ptr::null_mut;
        Link {
            elem: value,
            prev: Cell::new(null_mut()),
            next: Cell::new(null_mut()),
            _pin: PhantomPinned,
        }
    }

    unsafe fn pin_as_pointer(self: Pin<&Link<T>>) -> *mut Link<T> {
        Pin::into_inner_unchecked(self) as *const Link<T> as *mut Link<T>
    }

    unsafe fn pin_mut_as_pointer(self: Pin<&mut Link<T>>) -> *mut Link<T> {
        self.get_unchecked_mut()
    }

    unsafe fn pointer_as_pin(this: *mut Link<T>) -> Option<Pin<&'a mut Link<T>>> {
        this.as_mut().map(|r| unsafe { Pin::new_unchecked(r) })
    }

    fn value(self: Pin<&Link<T>>) -> Pin<&T> {
        unsafe { self.map_unchecked(|l| &l.elem) }
    }

    fn get_prev(self: Pin<&Link<T>>) -> *mut Link<T> {
        unsafe { Pin::into_inner_unchecked(self).prev.get() }
    }

    fn set_prev(self: Pin<&Link<T>>, prev: *mut Link<T>) {
        unsafe { Pin::into_inner_unchecked(self).prev.set(prev) }
    }

    fn get_next(self: Pin<&Link<T>>) -> *mut Link<T> {
        unsafe { Pin::into_inner_unchecked(self).next.get() }
    }

    fn set_next(self: Pin<&Link<T>>, next: *mut Link<T>) {
        unsafe { Pin::into_inner_unchecked(self).next.set(next) }
    }

    fn insert_before(self: Pin<&'a Link<T>>, link: Pin<&'a mut Link<T>>) {
        let link_ptr = unsafe { Self::pin_mut_as_pointer(link) };
        match unsafe { Self::pointer_as_pin(self.prev.get()) } {
            Some(prev) => {
                prev.as_ref().set_next(link_ptr);
                unsafe { (*link_ptr).prev.set(Self::pin_mut_as_pointer(prev)) };
            }
            None => {
                unsafe { (*link_ptr).prev.set(core::ptr::null_mut()) };
            }
        }

        self.set_prev(link_ptr);
        unsafe {
            (*link_ptr).next.set(Self::pin_as_pointer(self));
        }
    }

    fn insert_after(self: Pin<&Link<T>>, link: Pin<&mut Link<T>>) {
        let link_ptr = unsafe { Self::pin_mut_as_pointer(link) };
        match unsafe { Self::pointer_as_pin(self.next.get()) } {
            Some(next) => {
                next.as_ref().set_prev(link_ptr);
                unsafe { (*link_ptr).next.set(Self::pin_mut_as_pointer(next)) };
            }
            None => {
                unsafe { (*link_ptr).next.set(core::ptr::null_mut()) };
            }
        }

        self.set_next(link_ptr);
        unsafe {
            (*link_ptr).prev.set(Self::pin_as_pointer(self));
        }
    }
}

impl<'this, T> LinkedList<'this, T> {
    pub fn new() -> LinkedList<'this, T> {
        LinkedList {
            head: core::ptr::null_mut(),
            tail: core::ptr::null_mut(),
            _self: PhantomData,
        }
    }

    pub fn insert_front<'b: 'this>(&mut self, link: Pin<&mut Link<T>>) {
        match unsafe { Link::pointer_as_pin(self.head) } {
            Some(head) => Link::insert_before(head.as_ref(), link),
            None => {
                let link_ptr = unsafe { Link::pin_as_pointer(link.as_ref()) };
                self.head = link_ptr;
                self.tail = link_ptr;
            }
        }
    }

    pub fn insert_back<'b: 'this>(&mut self, link: Pin<&mut Link<T>>) {
        match unsafe { Link::pointer_as_pin(self.tail) } {
            Some(tail) => Link::insert_after(tail.as_ref(), link),
            None => {
                let link_ptr = unsafe { Link::pin_as_pointer(link.as_ref()) };
                self.head = link_ptr;
                self.tail = link_ptr;
            }
        }
    }

    pub fn front(&self) -> Cursor<T> {
        Cursor {
            current: self.head,
            list: self,
        }
    }

    pub fn front_mut<'a: 'this>(&'a mut self) -> CursorMut<'a, 'this, T> {
        CursorMut {
            current: self.head,
            list: self,
        }
    }

    pub fn len(&self) -> usize {
        let mut cursor = self.front();
        let mut count = 0;
        while let Some(_) = cursor.get() {
            cursor.move_next();
            count += 1;
        }
        count
    }

    pub fn iter(&self) -> impl Iterator<Item = Pin<&T>> {
        LinkedListIter {
            list: PhantomData,
            current: self.head,
        }
    }
}

struct LinkedListIter<'a, T> {
    list: PhantomData<&'a LinkedList<'a, T>>,
    current: *mut Link<T>,
}

impl<'a, T> Iterator for LinkedListIter<'a, T> {
    type Item = Pin<&'a T>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            match self.current.as_ref() {
                Some(current) => {
                    self.current = current.next.get();
                    Some(Pin::new_unchecked(&current.elem))
                }
                None => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_mut)]
    use rand::RngCore;

    use super::Link;
    use super::LinkedList;
    use core::pin::pin;
    use core::pin::Pin;
    extern crate std;
    use arbitrary::Arbitrary;
    use std::boxed::Box;
    use std::vec::Vec;

    struct VecCursorMut<'a, T> {
        vec: &'a mut Vec<T>,
        pos: usize,
    }

    #[derive(Debug, Arbitrary)]
    enum CursorCommand<T> {
        MoveNext,
        MovePrev,
        InsertBefore(T),
        InsertAfter(T),
        Get,
    }

    impl<'a, T> VecCursorMut<'a, T> {
        fn move_next(&mut self) -> bool {
            if self.pos == self.vec.len() {
                false
            } else {
                self.pos += 1;
                true
            }
        }

        fn move_prev(&mut self) -> bool {
            if self.pos == 0 {
                false
            } else {
                self.pos -= 1;
                true
            }
        }

        fn insert_before(&mut self, value: T) {
            self.vec.insert(self.pos, value);
        }

        fn insert_after(&mut self, value: T) {
            if self.pos == self.vec.len() {
                self.vec.insert(self.pos, value);
            } else {
                self.vec.insert(self.pos + 1, value);
            }
        }

        fn get(&self) -> Option<&T> {
            self.vec.get(self.pos)
        }
    }

    #[test]
    fn can_create_list() {
        let list: LinkedList<()> = LinkedList::new();
    }

    #[test]
    fn can_create_link() {
        let link = Link::new(1);
    }

    #[test]
    fn can_append_list() {
        let mut list: LinkedList<u32> = LinkedList::new();
        let mut link = pin!(Link::new(1));
        list.insert_front(link);
    }

    #[test]
    fn can_append_list_back() {
        let mut list: LinkedList<u32> = LinkedList::new();
        let mut link = pin!(Link::new(1));
        list.insert_back(link);
    }

    #[test]
    fn empty_list_can_create_cursor() {
        let mut list: LinkedList<u32> = LinkedList::new();
        let cursor = list.front();
        assert!(cursor.get().is_none());
    }

    #[test]
    fn singleton_list_can_create_cursor() {
        let mut list: LinkedList<u32> = LinkedList::new();
        let mut linka = pin!(Link::new(1));
        let mut linkb = pin!(Link::new(2));
        list.insert_back(linka);
        list.insert_back(linkb);
        let mut cursor = list.front();
        assert_eq!(*cursor.get().unwrap(), 1);
        cursor.move_next();
        assert_eq!(*cursor.get().unwrap(), 2);
        cursor.move_prev();
        assert_eq!(*cursor.get().unwrap(), 1);
    }

    #[test]
    fn can_recover_link() {
        let mut list = LinkedList::new();
        let mut linka = pin!(Link::new(1));
        let mut linkb = pin!(Link::new(2));
        {
            list.insert_back(linka);
            list.insert_back(linkb);
            drop(list);
        }
    }

    #[test]
    fn unsound_list() {
        let mut list = LinkedList::new();
        let mut linka = pin!(Link::new(1));
        list.insert_back(linka);
        //list.insert_back(&mut linka); // this should fail to compile
        assert_eq!(list.len(), 1);
    }

    fn unsound_lifetime() {
        let mut list: LinkedList<'_, _> = LinkedList::new();
        let mut linka: Pin<&mut Link<_>> = pin!(Link::new(1));
        list.insert_back(linka);
        // list.insert_back(&mut linka) // this should fail to compile
    }

    #[test]
    fn can_collect_iter() {
        let mut list = LinkedList::new();
        let mut linka = pin!(Link::new(0));
        let mut linkb = pin!(Link::new(1));
        list.insert_back(linka);
        list.insert_back(linkb);

        let refs: Vec<_> = list.iter().collect();
        for (idx, r) in refs.into_iter().enumerate() {
            assert_eq!(idx, *Pin::into_inner(r) as usize);
        }
    }

    fn arbitrary_commands<'a, T: Arbitrary<'a>>(data: &'a mut [u8]) -> Vec<CursorCommand<T>> {
        let mut rng = rand::thread_rng();
        rng.fill_bytes(data);
        let mut u = arbitrary::Unstructured::new(data);
        let commands: Vec<CursorCommand<T>> = arbitrary::Arbitrary::arbitrary(&mut u).unwrap();
        return commands;
    }

    fn commands_equal_for_cursors<T: core::fmt::Debug + Clone>(commands: &[CursorCommand<T>]) {
        let mut vec_cursor = VecCursorMut {
            vec: &mut Vec::new(),
            pos: 0,
        };
        let mut list = LinkedList::new();
        let mut list_cursor = list.front_mut();
        std::println!("commands: {:?}", commands);

        for command in commands.iter() {
            match command {
                CursorCommand::MoveNext => {
                    vec_cursor.move_next();
                    list_cursor.move_next();
                }
                CursorCommand::MovePrev => {
                    vec_cursor.move_prev();
                    list_cursor.move_prev();
                }
                CursorCommand::InsertBefore(t) => {
                    let mut value = Box::pin(Link::new(t.clone()));
                    let mut vr = value.as_mut();
                    vec_cursor.insert_before(t.clone());
                    list_cursor.insert_before(vr);
                }
                CursorCommand::InsertAfter(t) => {
                    let mut value = Box::pin(Link::new(t.clone()));
                    let mut vr = value.as_mut();
                    vec_cursor.insert_after(t.clone());
                    list_cursor.insert_after(vr);
                }
                CursorCommand::Get => match (vec_cursor.get(), list_cursor.get()) {
                    (None, None) => {}
                    (Some(vr), Some(lr)) => {}
                    _ => panic!("uneven get, commands: {:?}", commands),
                },
            }
        }
    }

    //#[test]
    fn eq_vec_cursor() {
        let mut data = [0; 1024];

        for _ in 0..100 {
            let commands = arbitrary_commands::<usize>(&mut data);
            commands_equal_for_cursors(&commands);
        }
    }

    #[test]
    fn test_eq_fail() {
        use CursorCommand::*;
        let commands = [InsertAfter(2), InsertBefore(3)];
        for i in 0..commands.len() {
            commands_equal_for_cursors(&commands[0..=i]);
        }
    }
}
