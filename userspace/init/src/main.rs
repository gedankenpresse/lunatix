#![no_std]
#![no_main]

mod caddr_alloc;
mod commands;
mod drivers;
mod elfloader;
mod read;
mod sifive_uart;
mod static_vec;

use crate::commands::Command;
use crate::drivers::virtio_9p::init_9p_driver;
use crate::read::{ByteReader, EchoingByteReader, Reader};
use crate::sifive_uart::SifiveUartMM;
use align_data::{include_aligned, Align16};
use allocators::boundary_tag_alloc::{BoundaryTagAllocator, TagsU16};
use core::cell::RefCell;
use core::panic::PanicInfo;
use drivers::virtio_9p::P9Driver;
use fdt::node::FdtNode;
use fdt::Fdt;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::syscall_abi::system_reset::{ResetReason, ResetType};
use librust::syscall_abi::CAddr;
use librust::{print, println};
use sifive_uart::SifiveUart;
use uart_driver::{MmUart, Uart};

static HELLO_WORLD_BIN: &[u8] = include_aligned!(
    Align16,
    "../../../target/riscv64imac-unknown-none-elf/release/hello_world"
);

#[no_mangle]
fn _start() {
    main();
}

const CADDR_MEM: CAddr = 1;
const _CADDR_CSPACE: CAddr = 2;
const CADDR_VSPACE: CAddr = 3;
const CADDR_IRQ_CONTROL: CAddr = 4;
const CADDR_DEVMEM: CAddr = 5;
const CADDR_ASID_CONTROL: CAddr = 6;
const CADDR_UART_IRQ: CAddr = 7;
const CADDR_UART_NOTIFICATION: CAddr = 8;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    librust::system_reset(ResetType::Shutdown, ResetReason::SystemFailure);
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
    let Some(mut reg) = node.reg() else {
        return Err("no reg");
    };
    let Some(region) = reg.next() else {
        return Err("no memory region");
    };
    let Some(mut interrupts) = node.interrupts() else {
        return Err("no interrupts");
    };
    let Some(interrupt) = interrupts.next() else {
        return Err("no interrupt");
    };

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
    let Some(mut reg) = node.reg() else {
        return Err("no reg");
    };
    let Some(region) = reg.next() else {
        return Err("no memory region");
    };
    let Some(mut interrupts) = node.interrupts() else {
        return Err("no interrupts");
    };
    let Some(interrupt) = interrupts.next() else {
        return Err("no interrupt");
    };

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

fn init_stdin(stdio: &FdtNode) -> Result<impl ByteReader, &'static str> {
    enum Reader<'a> {
        Uart(Uart<'a>),
        Sifive(SifiveUart<'a>),
    }

    impl ByteReader for Reader<'_> {
        fn read_byte(&mut self) -> Result<u8, ()> {
            match self {
                Reader::Uart(uart) => {
                    let _ = librust::wait_on(CADDR_UART_NOTIFICATION).unwrap();
                    let c = unsafe { uart.read_data() };
                    librust::irq_complete(CADDR_UART_IRQ).unwrap();
                    return Ok(c);
                }
                Reader::Sifive(uart) => {
                    let _ = librust::wait_on(CADDR_UART_NOTIFICATION).unwrap();
                    let c = uart.read_data();
                    librust::irq_complete(CADDR_UART_IRQ).unwrap();
                    return Ok(c);
                }
            }
        }
    }
    if let Ok(uart) = init_uart(stdio) {
        return Ok(Reader::Uart(uart));
    }

    if let Ok(uart) = init_sifive_uart(&stdio) {
        return Ok(Reader::Sifive(uart));
    }
    return Err("could not init uart");
}

unsafe impl Send for FileSystem {}
unsafe impl Sync for FileSystem {}
pub struct FileSystem(RefCell<Option<P9Driver<'static>>>);
pub static FS: FileSystem = FileSystem(RefCell::new(None));

fn main() {
    //drivers::virtio_9p::test();
    //panic!();

    let dev_tree_address: usize = 0x20_0000_0000;
    let dt = unsafe { Fdt::from_ptr(dev_tree_address as *const u8).unwrap() };
    let stdin = init_stdin(&dt.chosen().stdout().expect("no stdout found")).unwrap();

    let p9 = init_9p_driver();
    let _ = FS.0.borrow_mut().insert(p9);

    shell(&mut EchoingByteReader(stdin));
    println!("Init task says good bye 👋");
}

fn shell(reader: &mut dyn ByteReader) {
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
    &commands::Cat,
    &commands::Ls,
];

fn process_cmd(input: &str) {
    print!("\n");

    let Some(cmd) = KNOWN_COMMANDS
        .iter()
        .find(|i| input.starts_with(i.get_name()))
    else {
        println!(
            "Unknown command {:?}. Enter 'help' for a list of commands",
            input
        );
        return;
    };
    match cmd.execute(input.strip_prefix(cmd.get_name()).unwrap().trim_start()) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
