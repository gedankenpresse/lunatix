#![no_std]
#![no_main]

mod commands;
mod elfloader;
mod read;
mod sifive_uart;

use crate::commands::Command;
use crate::read::{ByteReader, EchoingByteReader};
use crate::sifive_uart::SifiveUartMM;
use core::panic::PanicInfo;
use fdt::node::FdtNode;
use fdt::Fdt;
use librust::syscall_abi::identify::{CapabilityVariant, Identify};
use librust::syscall_abi::CAddr;
use librust::{print, println};
use sifive_uart::SifiveUart;
use uart_driver::{MmUart, Uart};

static HELLO_WORLD_BIN: &[u8] =
    include_bytes!("../../../target/riscv64imac-unknown-none-elf/release/hello_world");

#[no_mangle]
fn _start() {
    main();
}

const CADDR_MEM: CAddr = 1;
const _CADDR_CSPACE: CAddr = 2;
const CADDR_VSPACE: CAddr = 3;
const CADDR_IRQ_CONTROL: CAddr = 4;
const CADDR_DEVMEM: CAddr = 5;
const _CADDR_ASID_CONTROL: CAddr = 7;
const CADDR_UART_IRQ: CAddr = 7;
const CADDR_UART_NOTIFICATION: CAddr = 8;

const CADDR_CHILD_TASK: CAddr = 10;
const CADDR_CHILD_CSPACE: CAddr = 11;
const CADDR_CHILD_VSPACE: CAddr = 12;
const CADDR_CHILD_STACK_PAGE: CAddr = 13;
const CADDR_CHILD_PAGE_START: CAddr = 14;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}

fn init_uart<'a, 'dt>(node: &FdtNode<'a, 'dt>) -> Result<Uart<'static>, &'static str> {
    if let None = node
        .compatible()
        .expect("no comptible")
        .all()
        .find(|&a| a == "ns16550a")
    {
        return Err("not compatible");
    }
    let Some(mut reg) = node.reg() else { return Err("no reg")};
    let Some(region) = reg.next() else { return Err("no memory region") };
    let Some(mut interrupts) = node.interrupts() else { return Err("no interrupts") };
    let Some(interrupt) = interrupts.next() else { return Err("no interrupt") };

    librust::derive(
        CADDR_MEM,
        CADDR_UART_NOTIFICATION,
        CapabilityVariant::Notification,
        None,
    )
    .unwrap();
    librust::irq_control_claim(
        CADDR_IRQ_CONTROL,
        interrupt as usize,
        CADDR_UART_IRQ,
        CADDR_UART_NOTIFICATION,
    )
    .unwrap();
    assert_eq!(
        librust::identify(CADDR_UART_IRQ).unwrap(),
        CapabilityVariant::Irq
    );

    librust::devmem_map(
        CADDR_DEVMEM,
        CADDR_MEM,
        CADDR_VSPACE,
        region.starting_address as usize,
        region.size.unwrap() as usize,
    )
    .unwrap();
    let mut uart = unsafe { Uart::from_ptr(region.starting_address as *mut MmUart) };
    uart.enable_rx_interrupts();
    Ok(uart)
}

fn init_sifive_uart(node: &FdtNode<'_, '_>) -> Result<SifiveUart<'static>, &'static str> {
    let compatible = node.compatible().expect("no comptible").first();
    if compatible != "sifive,uart0" {
        return Err("not compatible");
    }
    let Some(mut reg) = node.reg() else { return Err("no reg")};
    let Some(region) = reg.next() else { return Err("no memory region") };
    let Some(mut interrupts) = node.interrupts() else { return Err("no interrupts") };
    let Some(interrupt) = interrupts.next() else { return Err("no interrupt") };

    librust::derive(
        CADDR_MEM,
        CADDR_UART_NOTIFICATION,
        CapabilityVariant::Notification,
        None,
    )
    .unwrap();
    librust::irq_control_claim(
        CADDR_IRQ_CONTROL,
        interrupt as usize,
        CADDR_UART_IRQ,
        CADDR_UART_NOTIFICATION,
    )
    .unwrap();
    assert_eq!(
        librust::identify(CADDR_UART_IRQ).unwrap(),
        CapabilityVariant::Irq
    );

    librust::devmem_map(
        CADDR_DEVMEM,
        CADDR_MEM,
        CADDR_VSPACE,
        region.starting_address as usize,
        region.size.unwrap() as usize,
    )
    .unwrap();
    let mut uart = unsafe { SifiveUart::from_ptr(region.starting_address as *mut SifiveUartMM) };
    uart.enable_rx_interrupts();
    Ok(uart)
}

fn main() {
    let dev_tree_address: usize = 0x20_0000_0000;
    let dt = unsafe { Fdt::from_ptr(dev_tree_address as *const u8).unwrap() };
    let stdout = dt.chosen().stdout().expect("no stdout found");
    if let Ok(uart) = init_uart(&stdout) {
        fn read_char_blocking(uart: &Uart) -> u8 {
            let _ = librust::wait_on(CADDR_UART_NOTIFICATION).unwrap();
            let c = unsafe { uart.read_data() };
            librust::irq_complete(CADDR_UART_IRQ).unwrap();
            return c;
        }
        struct R<'a> {
            uart: Uart<'a>,
        }
        impl ByteReader for R<'_> {
            fn read_byte(&mut self) -> Result<u8, ()> {
                Ok(read_char_blocking(&mut self.uart))
            }
        }
        shell(&mut EchoingByteReader(R { uart }));
    } else if let Ok(uart) = init_sifive_uart(&stdout) {
        fn read_char_blocking(uart: &mut SifiveUart) -> u8 {
            let _ = librust::wait_on(CADDR_UART_NOTIFICATION).unwrap();
            let c = uart.read_data();
            librust::irq_complete(CADDR_UART_IRQ).unwrap();
            return c;
        }
        struct R<'a> {
            uart: SifiveUart<'a>,
        }
        impl ByteReader for R<'_> {
            fn read_byte(&mut self) -> Result<u8, ()> {
                Ok(read_char_blocking(&mut self.uart))
            }
        }
        shell(&mut EchoingByteReader(R { uart }));
    }

    println!("Init task says good bye 👋");
}

fn shell(reader: &mut dyn ByteReader) {
    assert_eq!(
        librust::identify(CADDR_IRQ_CONTROL).unwrap(),
        CapabilityVariant::IrqControl
    );

    let mut buf = [0u8; 256];
    loop {
        let cmd = read_cmd(reader, &mut buf);
        process_cmd(cmd);
    }
}

fn read_cmd<'b>(reader: &mut dyn ByteReader, buf: &'b mut [u8]) -> &'b str {
    // reset buffer
    let mut pos: isize = 0;
    for c in buf.iter_mut() {
        *c = 0;
    }

    print!("> ");

    loop {
        let c = reader.read_byte().unwrap();
        match c as char {
            // handle backspace
            '\x7f' => {
                buf[pos as usize] = 0;
                pos = core::cmp::max(pos - 1, 0);
            }

            // handle carriage return
            '\x0d' => {
                return core::str::from_utf8(&buf[0..pos as usize])
                    .expect("could not interpret char buffer as string");
            }
            // append any other character to buffer
            _ => {
                buf[pos as usize] = c;
                pos = core::cmp::min(pos + 1, buf.len() as isize - 1);
            }
        }
    }
}

struct Help;

impl Command for Help {
    fn get_name(&self) -> &'static str {
        "help"
    }

    fn get_summary(&self) -> &'static str {
        "help for this command"
    }

    fn execute(&self, _args: &str) -> Result<(), &'static str> {
        println!("Known Commands: ");
        for cmd in KNOWN_COMMANDS {
            println!("\t {: <12} {}", cmd.get_name(), cmd.get_summary());
        }
        Ok(())
    }
}

const KNOWN_COMMANDS: &[&'static dyn Command] = &[
    &commands::SecondTask,
    &commands::Echo,
    &commands::Shutdown,
    &Help,
    &commands::Identify,
    &commands::Destroy,
    &commands::Copy,
];

fn process_cmd(input: &str) {
    print!("\n");

    let Some(cmd) = KNOWN_COMMANDS
        .iter()
        .find(|i| input.starts_with(i.get_name()))
    else {
        println!(
            "Unknown command {:?}. Enter 'help' for a list of commands",
            input);
            return };
    match cmd.execute(input.strip_prefix(cmd.get_name()).unwrap().trim_start()) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
