use crate::caps::{AsidControl, CSpace, Capability, Error};

pub fn asid_control_send(
    cspace: &CSpace,
    asid_control: &AsidControl,
    args: &[usize],
) -> Result<(), Error> {
    const ASSIGN: usize = 0;

    match args[0] {
        ASSIGN => asid_control_assign(asid_control, unsafe {
            cspace
                .lookup_raw(args[1])
                .ok_or(Error::InvalidCAddr)?
                .as_ref()
                .unwrap()
        }),
        _ => Err(Error::Unsupported),
    }
}

fn asid_control_assign(asid_control: &AsidControl, args: &Capability) -> Result<(), Error> {
    todo!()
}
