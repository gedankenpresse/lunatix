use crate::println;
use crate::registers::{RO, RW};
use core::fmt::{Debug, Formatter, Write};
use fdt_rs::base::DevTree;
use fdt_rs::prelude::{FallibleIterator, PropReader};

/// A controller for UART (ns16550) devices
pub struct Uart<'a> {
    /// A handle to the memory mapped Uart controller
    regs: &'a MmUart,
    /// The baud rate of the controller (if known)
    pub baud_rate: Option<u32>,
}

/// Memory mapped UART controller
///
/// This struct models how the ns16550a registers are layed out in memory.
///
/// For more details on how the controller works see the [ns16550 spec](http://caro.su/msx/ocm_de1/16550.pdf).
pub struct MmUart {
    transceiver: RW<u8>,
    _interrupt_enable: RW<u8>,
    _interrupt_status___fifo_control: RW<u8>,
    _line_control: RW<u8>,
    _modem_control: RW<u8>,
    line_status: RO<u8>,
    _modem_status: RO<u8>,
    _scratch_pad: RW<u8>,
}

impl<'a> Uart<'a> {
    /// Create a Uart controller from the description given in a device tree.
    ///
    /// The device tree should model the Uart device as specified by the [Device Tree Specification v0.3 Section 4.2](https://devicetree-specification.readthedocs.io/en/v0.3/device-bindings.html#serial-devices).
    pub unsafe fn from_device_tree(dev_tree: &DevTree) -> Result<Self, ()> {
        match dev_tree.compatible_nodes("ns16550a").next() {
            Err(_) => Err(()),
            Ok(None) => Err(()),
            Ok(Some(node)) => {
                println!("Using device {} as UART device", node.name().unwrap());
                // TODO Handle registers better. Right now we completely ignore that reg is "in the address space of the parent bus" and what virtual-reg means
                let addr_prop = node
                    .props()
                    .find(|prop| prop.name().map(|name| name == "reg"))
                    .unwrap()
                    .unwrap()
                    .raw();

                // TODO Fetch #address-cells and #size-cells from parent node (although 2 and 2 are usually the default on 64 bit systems) and use it in reg interpretation
                let mm_addr = u64::from_be_bytes((&addr_prop[0..8]).try_into().unwrap());
                let mm_len = u64::from_be_bytes((&addr_prop[8..16]).try_into().unwrap());
                println!(
                    "UART device {} is memory mapped at 0x{:x}",
                    node.name().unwrap(),
                    mm_addr
                );

                // fetch baud rate property from device tree
                let baud_rate = u32::from_be_bytes(
                    node.props()
                        .find(|prop| prop.name().map(|name| name == "clock-frequency"))
                        .unwrap()
                        .unwrap()
                        .raw()
                        .try_into()
                        .unwrap(),
                );

                // construct an instance
                Ok(Self {
                    regs: &*(mm_addr as *mut MmUart),
                    baud_rate: Some(baud_rate),
                })
            }
        }
    }

    pub unsafe fn from_ptr(pointer: *mut MmUart) -> Self {
        Self {
            regs: &*pointer,
            baud_rate: None,
        }
    }

    /// Whether this UART device has data ready for reading
    pub fn has_rx(&self) -> bool {
        // this is safe because we know that UART does not perform side effects when reading this register
        unsafe { self.regs.line_status.read() & 1 == 1 }
    }

    /// Write a single byte into the Uart device
    pub unsafe fn write_data(&self, data: u8) {
        self.regs.transceiver.write(data)
    }

    /// Read a single byte from the Uart device.
    ///
    /// **Caution**: If no data is ready to be read, this will block until new data becomes available.
    /// Use [`has_rx()`](Uart::has_rx) to check if there is data to read before actually reading it.
    pub unsafe fn read_data(&self) -> u8 {
        self.regs.transceiver.read()
    }
}

impl<'a> Write for Uart<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &char in s.as_bytes() {
            unsafe { self.write_data(char) }
        }
        Ok(())
    }
}

impl<'a> Debug for Uart<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Uart")
            .field(
                "memory_map",
                &format_args!("0x{:x}", self.regs as *const MmUart as usize),
            )
            .field("baud_rate", &self.baud_rate)
            .finish()
    }
}
