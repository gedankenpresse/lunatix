use crate::registers::WO;

pub enum ShutdownCode {
    Pass,
    Fail(u16),
    Reset,
}

pub struct SifiveShutdown {
    reg: WO<u32>,
}

impl SifiveShutdown {
    pub unsafe fn shutdown(&self, code: ShutdownCode) -> ! {
        self.reg.write(match code {
            ShutdownCode::Pass => 0x5555,
            ShutdownCode::Fail(exit_code) => 0x3333 | (exit_code as u32) << 16,
            ShutdownCode::Reset => 0x00007777,
        });
        panic!("device did not shutdown");
    }
}
