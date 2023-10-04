use crate::caps::{Capability, Tag, Variant};
use core::mem::ManuallyDrop;
use derivation_tree::caps::CapabilityIface;
use derivation_tree::tree::TreeNodeOps;
use derivation_tree::{AsStaticMut, AsStaticRef};

/// A notification capability
#[derive(Copy, Clone)]
pub struct Notification {}

pub struct NotificationIface;

impl NotificationIface {
    /// Derive a new Notification capability from a memory capability.
    pub fn derive(&self, src_mem: &Capability, target_slot: &mut Capability) {
        assert_eq!(src_mem.tag, Tag::Memory);
        assert_eq!(target_slot.tag, Tag::Uninit);

        // create a new notification in the target slot
        target_slot.tag = Tag::Notification;
        target_slot.variant = Variant {
            notification: ManuallyDrop::new(Notification {}),
        };

        unsafe {
            src_mem.insert_derivation(target_slot);
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
                notification: ManuallyDrop::new(Notification {}),
            }
        }

        // insert the new copy into the derivation tree
        unsafe { src.insert_copy(dst) };
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
