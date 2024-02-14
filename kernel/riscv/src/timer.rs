use crate::cpu;
use sbi::SbiError;

/// Schedule an interrupt to trigger in `t` time units.
///
/// The time unit is hardware implementation dependent and its determination is out of scope here.
/// How you determine the number of time each tick represents is platform-dependent, and the frequency of the clock should be expressed in the timebase-frequency property of the CPU nodes in the devicetree, if you have one available.
pub fn set_timeout(t: u64) -> Result<(), SbiError> {
    log::trace!("scheduling timer interrupt in {} time units", t);
    sbi::timer::set_timer(cpu::Time::read() + t)
}
