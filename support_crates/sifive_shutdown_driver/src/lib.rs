//! Driver implementation for Sifive test devices (which are used for shutdown and reboot)
#![no_std]

use regs::WO;

/// The exact shutdown code to pass to the device
pub enum ShutdownCode {
    /// Indication that everything is well and that a normal shutdown should be performed.
    Pass,
    /// Indication that something is wrong and the device had to shut down because of it.
    /// An additional exit code may be set which may be passed on to other tools related to the running device (e.g. QEMU).
    Fail(u16),
    /// Instead of shutting the device down, a reboot should be performed instead.
    Reset,
}

/// Controller for a memory mapped Sifive test device
pub struct SifiveShutdown<'a> {
    reg: &'a WO<u32>,
}

impl<'a> SifiveShutdown<'a> {
    /// Create a controller of a Sifive test device that is memory mapped at `ptr`.
    ///
    /// # Safety
    /// - This function is safe to use iff `ptr` points to the memory mapped register of an
    /// attached Sifive test device.
    ///
    /// - For correct *mut* semantics the caller must also ensure that no two instances are created
    /// for the same memory mapped device.
    pub unsafe fn from_ptr(ptr: *mut u32) -> Self {
        Self {
            reg: &*(ptr as *mut WO<u32>),
        }
    }

    /// Shut down the device with the given code
    ///
    /// # Safety
    /// This is always unsafe because it will literally stop the CPU and terminate everything.
    pub unsafe fn shutdown(&self, code: ShutdownCode) -> ! {
        self.reg.write(match code {
            ShutdownCode::Pass => 0x5555,
            ShutdownCode::Fail(exit_code) => 0x3333 | (exit_code as u32) << 16,
            ShutdownCode::Reset => 0x00007777,
        });
        panic!("device did not shutdown");
    }
}
