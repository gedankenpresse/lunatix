use crate::caps::{asid::asid_control_assign, AsidControl, CSpace, Error};
use syscall_abi::send::SendArgs;

pub fn asid_control_send(
    cspace: &CSpace,
    asid_control: &AsidControl,
    args: &SendArgs,
) -> Result<(), Error> {
    const ASSIGN: usize = 1234;
    match args.label() {
        ASSIGN => asid_control_assign(asid_control, unsafe {
            cspace
                .resolve_caddr(args.cap_args()[0])
                .ok_or(Error::InvalidCAddr)?
                .as_mut()
                .unwrap()
                .get_inner_vspace_mut()
                .unwrap()
        }),
        _ => Err(Error::Unsupported),
    }
}
