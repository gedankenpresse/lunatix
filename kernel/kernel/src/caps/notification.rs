use crate::caps::{CapCounted, Capability, Tag, TaskIface, Variant};
use allocators::Box;
use core::cell::RefCell;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef, Correspondence};

#[derive(Eq, PartialEq)]
pub struct NotificationState {
    /// The notification value contained in this notification.
    /// A value of `0` indicates that the notification is unset.
    pub value: usize,

    /// A task that is currently waiting on this notification.
    pub wait_set: Option<*mut Capability>,
}

/// A notification capability
#[derive(Clone)]
pub struct Notification {
    state: CapCounted<RefCell<NotificationState>>,
}

impl Correspondence for Notification {
    fn corresponds_to(&self, other: &Self) -> bool {
        self.state.is_same_pointer_as(&other.state)
    }
}

pub struct NotificationIface;

impl NotificationIface {
    /// Derive a new Notification capability from a memory capability.
    pub fn derive(&self, src_mem: &Capability, target_slot: &mut Capability) {
        assert_eq!(src_mem.tag, Tag::Memory);
        assert_eq!(target_slot.tag, Tag::Uninit);

        // initialize shared state
        let state = RefCell::new(NotificationState {
            value: 0,
            wait_set: None,
        });
        let state = unsafe {
            Box::new(state, src_mem.get_inner_memory().unwrap().allocator.deref())
                .unwrap()
                .ignore_lifetimes()
        };

        // create a new notification in the target slot
        target_slot.tag = Tag::Notification;
        target_slot.variant = Variant {
            notification: ManuallyDrop::new(Notification {
                state: CapCounted::from_box(state),
            }),
        };

        unsafe {
            src_mem.insert_derivation(target_slot);
        }
    }

    /// Set the notification to active and wake all tasks waiting on it
    pub fn notify(&self, notification: &Capability) {
        assert_eq!(notification.tag, Tag::Notification);
        let mut state = notification
            .get_inner_notification()
            .unwrap()
            .state
            .borrow_mut();

        // TODO support setting the notification to a specific value
        state.value = 1;
        if let Some(task) = state.wait_set {
            // TODO use cursor
            let task = unsafe { &mut *task };
            TaskIface.wake(task)
        }
    }

    /// Get the currently contained value and clear it
    pub fn take_value(&self, notification: &Capability) -> usize {
        assert_eq!(notification.tag, Tag::Notification);
        let mut state = notification
            .get_inner_notification()
            .unwrap()
            .state
            .borrow_mut();
        let value = state.value;
        state.value = 0;
        value
    }

    /// Remove the given task from the notifications wait_set.
    ///
    /// If the task is not part of the wait_set, this function is a noop.
    ///
    /// # Safety
    /// The notification may point to the task capability if the task is waiting on it.
    /// Consequently the task also points to the notification to indicate that it is waiting on it.
    /// This function only removes the pointer *notification to task* pointer which leaves the two
    /// objects in an inconsistent state.
    /// After calling this function, the *task to notification* pointer **must** also be cleared.
    pub unsafe fn remove_from_wait_set(&self, notification: &Capability, task: *mut Capability) {
        assert_eq!(notification.tag, Tag::Notification);
        let mut state = notification
            .get_inner_notification()
            .unwrap()
            .state
            .borrow_mut();
        match state.wait_set {
            None => {}
            Some(waiting_task) => {
                if waiting_task == task {
                    state.wait_set = None;
                }
            }
        }
    }

    /// Add the task to the notifications wait_set
    ///
    /// # Safety
    /// Ensure that the task also has its `waiting_on` field set to this notification.
    pub unsafe fn add_to_wait_set(&self, notification: &Capability, task: *mut Capability) {
        assert_eq!(notification.tag, Tag::Notification);
        let mut state = notification
            .get_inner_notification()
            .unwrap()
            .state
            .borrow_mut();
        match state.wait_set {
            Some(existing_task) => {
                if existing_task != task {
                    panic!("notification already has a waiting task")
                }
            }
            None => state.wait_set = Some(task),
        }
    }
}

impl CapabilityIface<Capability> for NotificationIface {
    type InitArgs = ();

    fn init(&self, target: &mut impl AsStaticMut<Capability>, args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::Notification);
        assert_eq!(dst.tag, Tag::Uninit);

        // semantically copy the notification
        dst.tag = Tag::Notification;
        {
            let src_notification = src.get_inner_notification().unwrap();
            dst.variant = Variant {
                notification: ManuallyDrop::new(Notification {
                    state: src_notification.state.clone(),
                }),
            }
        }

        // insert the new copy into the derivation tree
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
