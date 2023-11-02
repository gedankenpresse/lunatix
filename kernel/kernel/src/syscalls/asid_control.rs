use crate::caps::{asid::asid_control_assign, AsidControl, CSpace, SyscallError};
use syscall_abi::send::SendArgs;

pub fn asid_control_send(
    cspace: &CSpace,
    asid_control: &AsidControl,
    args: &SendArgs,
) -> Result<(), SyscallError> {
    const ASSIGN: usize = 1234;
    match args.label() {
        ASSIGN => asid_control_assign(asid_control, unsafe {
            cspace
                .resolve_caddr(args.cap_args()[0])
                .ok_or(SyscallError::InvalidCAddr)?
                .as_mut()
                .unwrap()
                .get_inner_vspace_mut()
                .unwrap()
        }),
        _ => Err(SyscallError::Unsupported),
    }
}
