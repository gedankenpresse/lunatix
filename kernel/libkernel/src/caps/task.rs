use crate::arch::trap::TrapFrame;
use crate::caps::cspace::CSpace;
use crate::caps::CapHolder;
use core::mem::MaybeUninit;
use core::ptr;

pub struct Task {
    pub frame: TrapFrame,
    pub cspace: CapHolder<CSpace>,
    pub vspace: CapHolder<()>,
}

impl Task {
    pub unsafe fn init(ptr: *mut MaybeUninit<Task>) {
        let ptr = ptr.cast::<Task>();
        ptr::addr_of_mut!((*ptr).frame).write(TrapFrame::null());
        ptr::addr_of_mut!((*ptr).cspace).write(CapHolder::new(CSpace::new()));
        ptr::addr_of_mut!((*ptr).vspace).write(CapHolder::new(()));
    }
}
