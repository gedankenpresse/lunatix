use crate::UART_DEVICE;
use core::fmt;
use core::fmt::Write;
use core::ops::DerefMut;

use uart_driver::{MmUart, Uart};

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::printk::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    if let Some(uart) = UART_DEVICE.spin_lock().deref_mut() {
        uart.write_fmt(args).unwrap();
    } else {
        let mut uart = unsafe { Uart::from_ptr(0x1000_0000 as *mut MmUart) };
        uart.write_str(
            "Warning: UART device has not been set up. Using hardcoded qemu device pointer.\n",
        )
        .unwrap();
        uart.write_fmt(args).unwrap();
    }
}
