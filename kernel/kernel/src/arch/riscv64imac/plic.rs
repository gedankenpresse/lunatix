use regs::RW;

#[repr(C)]
struct ContextEnable {
    enable_bits: [RW<u32>; 32],
}

#[repr(C)]
struct Context {
    priority_threshold: RW<u32>,
    claim_complete: RW<u32>,
    _reserved: [u32; 0x3fe],
}

#[repr(C)]
pub struct PLIC {
    priorites: [RW<u32>; 1024],
    pending: [RW<u32>; 32],
    _reserved0: [u32; 992],
    enable: [ContextEnable; 15872],
    _reserved1: [u32; 0x3800],
    context: [Context; 15872],
}

impl PLIC {
    pub fn enable_interrupt(&mut self, id: u32, context: usize) {
        // NOTE: if something with interrupts (id > 32) is broken, check this bit setting logic
        let reg = id / 32;
        let reg = &mut self.enable[context].enable_bits[reg as usize];
        unsafe {
            let bits = reg.read();
            reg.write(1 << (id % 32) | bits);
        }
    }

    pub fn set_priority(&mut self, id: u32, prio: u8) {
        assert!(prio < 8);
        unsafe {
            self.priorites[id as usize].write(prio as u32);
        }
    }

    pub fn set_threshold(&mut self, tsh: u8, context: usize) {
        log::debug!("setting plic threshold");
        assert!(tsh < 8);
        unsafe {
            self.context[context].priority_threshold.write(tsh as u32);
        }
    }

    pub fn claim_next(&mut self, context: usize) -> Option<u32> {
        let claim = unsafe { self.context[context].claim_complete.read() };
        if claim == 0 {
            None
        } else {
            Some(claim)
        }
    }

    pub fn complete(&mut self, context: usize, id: u32) {
        unsafe { self.context[context].claim_complete.write(id) }
    }
}

pub fn check_plic_offsets(mmio: &PLIC) {
    macro_rules! offset {
        ($offset:literal, $field:ident) => {
            log::debug!(
                "base {:p} {:>18} {:p} {:#04x}",
                &*mmio,
                stringify!($field),
                &mmio.$field,
                $offset
            );
        };
        ($offset:literal, $field:ident[$idx:literal]) => {
            log::debug!(
                "base {:p} {:>18} {:p} {:#04x}",
                &*mmio,
                stringify!($field[$idx]),
                &mmio.$field[$idx],
                $offset
            );
        };
    }

    offset!(0x0, priorites);
    offset!(0xffc, priorites[1023]);
    offset!(0x1000, pending);
    offset!(0x107c, pending[31]);
    offset!(0x2000, enable);
    offset!(0x2000, enable[0]);
    offset!(0x1f_1f80, enable[15871]);
    offset!(0x20_0000, context);
    offset!(0x20_0000, context[0]);
    offset!(0x20_1000, context[1]);
    offset!(0x3FF_e000, context[15871]);
}

pub unsafe fn init_plic<'a>(base_addr: *mut PLIC) -> &'a mut PLIC {
    log::debug!("Initialize PLIC...");
    let mmio = unsafe { &mut *base_addr };
    check_plic_offsets(mmio);
    log::debug!("Done.");
    mmio
}
