use derivation_tree::caps::CapabilityIface;
use syscall_abi::send::SendArgs;

use crate::{
    caps::{CSpace, CSpaceIface, Error, Tag, Task, VSpaceIface},
    syscalls::utils,
};

pub fn task_send(cspace: &CSpace, task: &Task, args: &SendArgs) -> Result<(), Error> {
    const ASSIGN_REGS: u16 = 1;
    const ASSIGN_VSPACE: u16 = 2;
    const ASSIGN_CSPACE: u16 = 3;
    match args.op {
        ASSIGN_REGS => task_assign_control_registers(task, args.data_args()),
        ASSIGN_VSPACE => task_assign_vspace(cspace, task, args.cap_args()[0]),
        ASSIGN_CSPACE => task_assign_cspace(cspace, task, args.cap_args()[0]),
        _ => Err(Error::Unsupported),
    }
}

fn task_assign_cspace(cspace: &CSpace, task: &Task, cspace_addr: usize) -> Result<(), Error> {
    // get valid cspace cap from current tasks cspace
    let source = unsafe { utils::lookup_cap(cspace, cspace_addr, Tag::CSpace) }?;

    // assign cspace to target task
    log::debug!("copy cspace: {}", cspace_addr);
    let mut task = task.state.borrow_mut();
    log::debug!("{:?}", task.cspace.get_tag());
    CSpaceIface.copy(&source, &mut task.cspace);
    log::trace!("cspace copied");
    Ok(())
}

fn task_assign_vspace(cspace: &CSpace, task: &Task, vspace_addr: usize) -> Result<(), Error> {
    // get valid cspace cap from current tasks cspace
    let source = unsafe { utils::lookup_cap(cspace, vspace_addr, Tag::VSpace) }?;

    // assign cspace to target task
    log::trace!(
        "copy vspace (asid = {}):",
        source.get_inner_vspace().unwrap().asid
    );
    let mut task = task.state.borrow_mut();
    VSpaceIface.copy(&source, &mut task.vspace);
    log::trace!("vspace copied");
    Ok(())
}

fn task_assign_control_registers(task: &Task, args: &[usize]) -> Result<(), Error> {
    // TODO Ensure that the task is not currently executing

    // assign control registers as specified by the syscall
    let mut task_state = task.state.borrow_mut();
    let [pc, sp, gp, tp] = args[..4] else {
        panic!("wrong count")
    };
    task_state.frame.start_pc = pc;
    task_state.frame.general_purpose_regs[2] = sp;

    // NOTE: check that this order is correct
    task_state.frame.general_purpose_regs[3] = gp;
    task_state.frame.general_purpose_regs[8] = tp;

    Ok(())
}
