use derivation_tree::caps::CapabilityIface;

use super::{
    AsidControlIface, CSpaceIface, Capability, DevmemIface, IrqControlIface, IrqIface, MemoryIface,
    NotificationIface, PageIface, TaskIface, VSpaceIface,
};

pub type CapCounted<T> = derivation_tree::CapCounted<'static, 'static, T>;
pub type KernelAlloc = allocators::bump_allocator::ForwardBumpingAllocator<'static>;

pub unsafe fn destroy(target: &mut Capability) {
    match target.get_tag() {
        crate::caps::Tag::Uninit => {}
        crate::caps::Tag::Memory => MemoryIface.destroy(target),
        crate::caps::Tag::CSpace => CSpaceIface.destroy(target),
        crate::caps::Tag::VSpace => VSpaceIface.destroy(target),
        crate::caps::Tag::Task => TaskIface.destroy(target),
        crate::caps::Tag::Page => PageIface.destroy(target),
        crate::caps::Tag::IrqControl => IrqControlIface.destroy(target),
        crate::caps::Tag::Irq => IrqIface.destroy(target),
        crate::caps::Tag::Notification => NotificationIface.destroy(target),
        crate::caps::Tag::Devmem => DevmemIface.destroy(target),
        crate::caps::Tag::AsidControl => AsidControlIface.destroy(target),
    };
}

pub unsafe fn copy(src: &Capability, dst: &mut Capability) {
    match src.get_tag() {
        crate::caps::Tag::Uninit => {}
        crate::caps::Tag::Memory => MemoryIface.copy(src, dst),
        crate::caps::Tag::CSpace => CSpaceIface.copy(src, dst),
        crate::caps::Tag::VSpace => VSpaceIface.copy(src, dst),
        crate::caps::Tag::Task => TaskIface.copy(src, dst),
        crate::caps::Tag::Page => PageIface.copy(src, dst),
        crate::caps::Tag::IrqControl => IrqControlIface.copy(src, dst),
        crate::caps::Tag::Irq => IrqIface.copy(src, dst),
        crate::caps::Tag::Notification => NotificationIface.copy(src, dst),
        crate::caps::Tag::Devmem => DevmemIface.copy(src, dst),
        crate::caps::Tag::AsidControl => AsidControlIface.copy(src, dst),
    };
}
