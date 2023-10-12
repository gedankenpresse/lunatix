use crate::caps::{asid::asid_control_assign, AsidControl, CSpace, Capability, Error, VSpace};

pub fn asid_control_send(
    cspace: &CSpace,
    asid_control: &AsidControl,
    args: &[usize],
) -> Result<(), Error> {
    const ASSIGN: usize = 1234;
    match args[0] {
        ASSIGN => asid_control_assign(asid_control, unsafe {
            cspace
                .lookup_raw(args[1])
                .ok_or(Error::InvalidCAddr)?
                .as_mut()
                .unwrap()
                .get_inner_vspace_mut()
                .unwrap()
        }),
        _ => Err(Error::Unsupported),
    }
}
