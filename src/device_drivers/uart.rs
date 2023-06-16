use crate::registers::{RO, RW};
use core::fmt::Write;

pub struct Uart {
    transceiver: RW<u8>,
    _interrupt_enable: RW<u8>,
    _interrupt_status___fifo_control: RW<u8>,
    _line_control: RW<u8>,
    _modem_control: RW<u8>,
    line_status: RO<u8>,
    _modem_status: RO<u8>,
    _scratch_pad: RW<u8>,
}

impl Uart {    
    /// Whether this UART device has data ready for reading
    pub fn has_rx(&self) -> bool {
        // this is safe because we know that UART does not perform side effects when reading this register
        unsafe { self.line_status.read() & 1 == 1 }
    }

    pub unsafe fn write_data(&self, data: u8) {
        self.transceiver.write(data)
    }

    pub unsafe fn read_data(&self) -> u8 {
        self.transceiver.read()
    }
}

impl Write for Uart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &char in s.as_bytes() {
            unsafe { self.write_data(char) }
        }
        Ok(())
    }
}
