use crate::caps;
use crate::ipc;
use crate::sched;

use libkernel::print;
use riscv::trap::TrapFrame;

const SYS_DEBUG_LOG: usize = 0;
const SYS_DEBUG_PUTC: usize = 1;
const SYS_SEND: usize = 2;
const SYS_IDENTIFY: usize = 3;
const SYS_DESTROY: usize = 4;

fn send(cspace: &caps::CSpace, cap: usize, tag: ipc::Tag, args: &[usize]) -> ipc::IpcResult {
    log::debug!("cap: {cap}, tag: {tag:?}, args: {args:?}");
    let raw = ipc::RawMessage::from_args(tag, args);
    log::debug!(
        "raw: caps: {:?}, params: {:?}",
        raw.cap_addresses,
        raw.params
    );
    // TODO check if object has send rights

    // TODO: remove this
    assert!(tag.ncaps() <= 8, "too many caps");
    let mut resolved: [Option<&caps::CSlot>; 8] = [None, None, None, None, None, None, None, None];
    for (i, &addr) in raw.cap_addresses.iter().enumerate() {
        resolved[i] = Some(cspace.lookup(addr)?);
    }

    let object = cspace.lookup(cap).unwrap();
    let res = object.send(tag.label(), &resolved[..tag.ncaps() as usize], raw.params)?;
    Ok(res)
}

fn identify(cspace: &caps::CSpace, cap: usize) -> ipc::IpcResult {
    log::debug!("identifiying: cap: {cap}");
    let capslot = cspace.lookup(cap)?;
    let variant = capslot.get_variant();
    return Ok(variant.discriminant());
}

fn destroy(cspace: &caps::CSpace, cap: usize) -> ipc::IpcResult {
    log::debug!("destory: cap: {cap}");
    let capslot = cspace.lookup(cap)?;
    let variant = capslot.get_variant();
    variant.as_iface().destroy(capslot);
    Ok(0)
}

#[inline(always)]
pub(crate) fn handle_syscall(tf: &mut TrapFrame) -> &mut TrapFrame {
    let args = tf.get_ipc_args();
    let res = match args[0] {
        SYS_DEBUG_LOG => {
            let bytes = args[1];
            let ptr = args[2..].as_ptr().cast::<u8>();
            let length = args[2..].len() * core::mem::size_of::<usize>();
            let str_slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, length) };
            let str = core::str::from_utf8(&str_slice[..bytes]).unwrap();
            print!("{}", str);
            Ok(0)
        }
        SYS_DEBUG_PUTC => {
            print!("{}", args[1] as u8 as char);
            Ok(0)
        }
        SYS_SEND => {
            log::debug!("SEND: {:?}", args);
            let cspace = sched::cspace().get_cspace().unwrap();
            send(&cspace, args[1], ipc::Tag::from_raw(args[2]), &args[3..])
        }
        SYS_IDENTIFY => {
            log::debug!("IDENTIFY: {:?}", args);
            let cspace = sched::cspace().get_cspace().unwrap();
            identify(&cspace, args[1])
        }
        SYS_DESTROY => {
            log::debug!("DESTORY: {:?}", args);
            let cspace = sched::cspace().get_cspace().unwrap();
            destroy(&cspace, args[1])
        }
        no => {
            panic!("unsupported syscall: {}", no);
        }
    };

    // write result back to userspace
    let (a0, a1) = ipc::result_to_raw(res);
    tf.write_syscall_result(a0, a1);
    return tf;
}
