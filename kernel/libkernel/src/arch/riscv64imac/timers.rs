use crate::arch::cpu;
use crate::println;
use sbi::SbiError;

pub fn set_next_timer(offset: u64) -> Result<(), SbiError> {
    println!("enabling timer interrupt in {} time units", offset);
    sbi::timer::set_timer(cpu::Time::read() + offset)
}
