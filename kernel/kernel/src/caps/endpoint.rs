use crate::caps::{CapCounted, Capability, Tag, Uninit, Variant};
use allocators::Box;
use core::cell::RefCell;
use core::mem::ManuallyDrop;
use core::ops::Deref;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef, Correspondence};

#[derive(Debug)]
pub struct EndpointState {
    /// A task that is currently waiting to receive data from this endpoint
    pub send_set: Option<*mut Capability>,
    /// A task that is currently waiting to send data from this endpoint
    pub recv_set: Option<*mut Capability>,
}

#[derive(Clone)]
pub struct Endpoint {
    pub state: CapCounted<RefCell<EndpointState>>,
}

impl Correspondence for Endpoint {
    fn corresponds_to(&self, other: &Self) -> bool {
        self.state.is_same_pointer_as(&other.state)
    }
}

pub struct EndpointIface;

impl EndpointIface {
    pub fn derive(&self, src_mem: &Capability, target_slot: &mut Capability) {
        assert_eq!(src_mem.tag, Tag::Memory);
        assert_eq!(target_slot.tag, Tag::Uninit);

        // initialize shared state
        let state = RefCell::new(EndpointState {
            send_set: None,
            recv_set: None,
        });
        let state = unsafe {
            Box::new(state, src_mem.get_inner_memory().unwrap().allocator.deref())
                .unwrap()
                .ignore_lifetimes()
        };

        // create a new endpoint in the target slot
        target_slot.tag = Tag::Endpoint;
        target_slot.variant = Variant {
            endpoint: ManuallyDrop::new(Endpoint {
                state: CapCounted::from_box(state),
            }),
        };

        // insert the newly created endpoint into the derivation tree
        unsafe {
            src_mem.insert_derivation(target_slot);
        }
    }

    /// Add the given task to the endpoints send_set.
    ///
    /// # Safety
    /// Ensure that the task also has its `waiting_on` field set to this endpoint.
    pub unsafe fn add_sender(&self, endpoint: &Endpoint, task: *mut Capability) {
        let mut state = endpoint.state.borrow_mut();
        match state.send_set {
            Some(existing_task) => {
                if existing_task != task {
                    panic!("endpoint already has a waiting sender");
                }
            }
            None => state.send_set = Some(task),
        }
    }

    /// Add the given task to the endpoints recv_set
    ///
    /// # Safety
    /// Ensure that the task also has its `waiting_on` field set to this endpoint.
    pub unsafe fn add_receiver(&self, endpoint: &Endpoint, task: *mut Capability) {
        let mut state = endpoint.state.borrow_mut();
        match state.recv_set {
            Some(existing_task) => {
                if existing_task != task {
                    panic!("endpoint already has a waiting receiver");
                }
            }
            None => state.recv_set = Some(task),
        }
    }
}

impl CapabilityIface<Capability> for EndpointIface {
    type InitArgs = ();

    fn init(&self, _target: &mut impl AsStaticMut<Capability>, _args: Self::InitArgs) {
        todo!()
    }

    fn copy(&self, src: &impl AsStaticRef<Capability>, dst: &mut impl AsStaticMut<Capability>) {
        let src = src.as_static_ref();
        let dst = dst.as_static_mut();
        assert_eq!(src.tag, Tag::Endpoint);
        assert_eq!(dst.tag, Tag::Uninit);

        // semantically copy the endpoint
        dst.tag = Tag::Endpoint;
        {
            let src_endpoint = src.get_inner_endpoint().unwrap();
            dst.variant = Variant {
                endpoint: ManuallyDrop::new(Endpoint {
                    state: src_endpoint.state.clone(),
                }),
            }
        }

        // insert the new copy into the derivation tree
        unsafe { src.insert_copy(dst) }
    }

    fn destroy(&self, target: &mut Capability) {
        assert_eq!(target.tag, Tag::Notification);

        if target.is_final_copy() {
            let endpoint = target.get_inner_endpoint_mut().unwrap();
            {
                let state = endpoint.state.borrow();
                assert!(
                    state.recv_set.is_none() && state.send_set.is_none(),
                    "can't destroy endpoint with waiting tasks"
                );
            }

            // Safety: This is the last endpoint instance and no tasks are waiting so no pointers are left
            // pointing to this capability
            unsafe { endpoint.state.destroy() }
        }

        // remove this capability instance from the derivation tree
        target.tree_data.unlink();
        target.tag = Tag::Uninit;
        target.variant = Variant { uninit: Uninit {} };
    }
}
