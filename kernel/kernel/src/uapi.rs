use crate::caps;
use crate::ipc;
use crate::sched;

use libkernel::arch::trap::TrapFrame;
use libkernel::print;

use core::cell::RefCell;

const SYS_DEBUG_LOG: usize = 0;
const SYS_DEBUG_PUTC: usize = 1;
const SYS_SEND: usize = 2;


fn send(cspace: &mut caps::CNode, cap: usize, tag: ipc::Tag, args: &[usize]) -> Result<usize, caps::Error> {
    let raw = ipc::RawMessage::from_args(tag, args);
    let cspaceref = cspace.get_cspace_mut().unwrap();
    // TODO check if object has send rights


    // TODO: resolve cap references
    assert!(tag.ncaps() <= 8, "too many caps");
    let mut resolved: [Option<&RefCell<caps::CSlot>>; 8] = [None, None, None, None, None, None, None, None]; 
    for (i, &addr) in raw.cap_addresses.iter().enumerate() {
        resolved[i] = Some(cspaceref.elem.lookup(addr)?);
    }

    let object = cspaceref.elem.lookup(cap).unwrap();
    let res = object.try_borrow_mut()?.cap.send(tag.label(), &resolved[..tag.ncaps() as usize], raw.params)?;
    Ok(res)
}

#[inline(always)]
pub (crate) fn handle_syscall(tf: &mut TrapFrame) -> &mut TrapFrame {
    let args = &mut tf.general_purpose_regs[10..=17];
    let res = match args[0] {
        SYS_DEBUG_LOG => {
            let bytes = args[1];
            let ptr = args[2..].as_ptr().cast::<u8>();
            let length = args[2..].len() * core::mem::size_of::<usize>();
            let str_slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, length) };
            let str = core::str::from_utf8(&str_slice[..bytes]).unwrap();
            print!("{}", str);
            Ok(0)
        },
        SYS_DEBUG_PUTC => {
            print!("{}", args[1] as u8 as char);
            Ok(0)
        },
        SYS_SEND => {
            let cspace = sched::cspace();
            send(cspace, args[1], ipc::Tag(args[2]), &args[3..])
        },
        no => { panic!("unsupported syscall: {}", no); }
    };

    // write result back to userspace
    let (a0, a1) = ipc::result_to_raw(res);
    tf.general_purpose_regs[10] = a0;
    tf.general_purpose_regs[11] = a1;
    return tf;
}