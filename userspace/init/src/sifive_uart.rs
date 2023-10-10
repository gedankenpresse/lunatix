use librust::println;
use regs::RW;

pub struct SifiveUartMM {
    txdata: RW<u32>,
    rxdata: RW<u32>,
    txctrl: RW<u32>,
    rxctrl: RW<u32>,
    ie: RW<u32>,
    ip: RW<u32>,
    div: RW<u32>,
}

pub struct SifiveUart<'a> {
    mm: &'a mut SifiveUartMM,
}

impl<'a> SifiveUart<'a> {
    pub fn log_settings(&self) {
        unsafe {
            let txdata = self.mm.txctrl.read();
            let rxdata = self.mm.rxctrl.read();
            let ie = self.mm.ie.read();
            let ip = self.mm.ip.read();
            let div = self.mm.div.read();
            println!("{txdata:x} {rxdata:x} {ie:x} {ip:x} {div:x}")
        }
    }

    pub unsafe fn from_ptr(ptr: *mut SifiveUartMM) -> Self {
        Self {
            mm: unsafe { ptr.as_mut().unwrap() },
        }
    }

    pub fn enable_rx_interrupts(&mut self) {
        let ie = unsafe { self.mm.ie.read() };
        unsafe { self.mm.ie.write(ie | 2) }
        let rxctl = unsafe { self.mm.rxctrl.read() };
        unsafe { self.mm.rxctrl.write((1 << 16) | rxctl | 1) };
        unsafe { self.mm.div.write(64) }
    }
    pub fn read_data(&mut self) -> u8 {
        unsafe { self.mm.rxdata.read() as u8 }
    }

    pub fn write_data(&mut self, byte: u8) {
        unsafe {
            self.mm.txdata.write(byte as u32);
        }
    }
}
