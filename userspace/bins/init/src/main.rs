#![no_std]
#![no_main]

extern crate alloc;

mod commands;
mod elfloader;
mod logger;
mod sched;
mod shell;
mod sifive_uart;
mod static_once_cell;
mod static_vec;

use crate::sifive_uart::SifiveUartMM;

use allocators::boundary_tag_alloc::{BoundaryTagAllocator, TagsU32};
use caddr_alloc::CAddrAlloc;
use core::{cell::RefCell, panic::PanicInfo, sync::atomic::AtomicUsize};
use fdt::{node::FdtNode, Fdt};
use io::read::{ByteReader, EchoingByteReader};
use librust::println;
use librust::syscall_abi::{identify::CapabilityVariant, system_reset::*, CAddr, MapFlags};
use log::Level;
use logger::Logger;
use sifive_uart::SifiveUart;
use static_once_cell::StaticOnceCell;
use uart_driver::{MmUart, Uart};
use virtio_p9::{init_9p_driver, P9Driver};

static LOGGER: Logger = Logger::new(Level::Info);

#[no_mangle]
fn _start() {
    LOGGER.install().expect("could not install logger");
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
    log::error!("panic {}", info);
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

pub unsafe fn alloc_init(pages: usize, addr: *mut u8) -> BoundaryTagAllocator<'static, TagsU32> {
    const PAGESIZE: usize = 4096;
    for i in 0..pages {
        let page = caddr_alloc::alloc_caddr();
        librust::derive(CADDR_MEM, page, CapabilityVariant::Page, None).unwrap();
        librust::map_page(
            page,
            CADDR_VSPACE,
            CADDR_MEM,
            addr as usize + i * PAGESIZE,
            MapFlags::READ | MapFlags::WRITE,
        )
        .unwrap();
    }

    let mem = unsafe { core::slice::from_raw_parts_mut(addr, pages * PAGESIZE) };
    mem.fill(0);
    BoundaryTagAllocator::new(mem)
}

#[global_allocator]
pub static ALLOC: StaticOnceCell<BoundaryTagAllocator<'static, TagsU32>> = StaticOnceCell::new();

pub static CADDR_ALLOC: CAddrAlloc = CAddrAlloc {
    max: AtomicUsize::new(128),
    cur: AtomicUsize::new(10),
};

fn main() {
    unsafe { caddr_alloc::set_global_caddr_allocator(&CADDR_ALLOC) };
    ALLOC.get_or_init(|| unsafe { alloc_init(32, 0x10_0000 as *mut u8) });
    let dev_tree_address: usize = 0x20_0000_0000;
    let dt = unsafe { Fdt::from_ptr(dev_tree_address as *const u8).unwrap() };
    let stdin = init_stdin(&dt.chosen().stdout().expect("no stdout found")).unwrap();

    let p9 = init_9p_driver(
        CADDR_MEM,
        CADDR_VSPACE,
        CADDR_DEVMEM,
        CADDR_IRQ_CONTROL,
        0x33_0000_0000 as *mut u8,
    );
    let _ = FS.0.borrow_mut().insert(p9);

    shell::shell(&mut EchoingByteReader(stdin));
    println!("Init task says good bye 👋");
}