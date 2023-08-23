pub use derivation_tree::caps::CapabilityIface;

use super::Capability;

pub type CSpace = derivation_tree::caps::CSpace<'static, 'static, Capability>;

#[derive(Copy, Clone)]
pub struct CSpaceIface;
impl CSpaceIface {
    pub fn derive(&self, mem: &Capability, cspace: &mut Capability) {
        todo!()
    }
}

impl CapabilityIface<Capability> for CSpaceIface {
    type InitArgs = usize;

    fn init(
        &self,
        target: &mut impl derivation_tree::AsStaticMut<Capability>,
        args: Self::InitArgs,
    ) {
        todo!()
    }

    fn copy(
        &self,
        src: &impl derivation_tree::AsStaticRef<Capability>,
        dst: &mut impl derivation_tree::AsStaticMut<Capability>,
    ) {
        todo!()
    }

    fn destroy(&self, target: &mut Capability) {
        todo!()
    }
}
