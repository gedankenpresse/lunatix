use crate::caps::{asid::asid_control_assign, AsidControl, CSpace, Error};

pub fn asid_control_send(
    cspace: &CSpace,
    asid_control: &AsidControl,
    op: u16,
    args: &[usize],
) -> Result<(), Error> {
    const ASSIGN: u16 = 1234;
    match op {
        ASSIGN => asid_control_assign(asid_control, unsafe {
            cspace
                .lookup_raw(args[0])
                .ok_or(Error::InvalidCAddr)?
                .as_mut()
                .unwrap()
                .get_inner_vspace_mut()
                .unwrap()
        }),
        _ => Err(Error::Unsupported),
    }
}
