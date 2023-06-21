#![no_std]
#![no_main]

#[path = "arch/riscv64imac/mod.rs"]
mod arch;

mod caps;
mod device_drivers;
mod mem;
mod registers;

use crate::arch::trap::TrapFrame;
use crate::device_drivers::shutdown::{ShutdownCode, SifiveShutdown};
use crate::device_drivers::uart::Uart;
use core::fmt;
use core::fmt::Write;
use core::ops::DerefMut;
use core::panic::PanicInfo;
use device_drivers::uart::MmUart;
use fdt_rs::base::DevTree;
use ksync::SpinLock;
use thiserror_no_std::private::DisplayAsDisplay;

static UART_DEVICE: SpinLock<Option<Uart>> = SpinLock::new(None);
static SHUTDOWN_DEVICE: SpinLock<Option<SifiveShutdown>> = SpinLock::new(None);

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    // print panic message
    println!("!!! Kernel Panic !!!");
    println!("  {}", info.as_display());

    // shutdown the device
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Fail(1)) }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
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

fn get_memory(dev_tree: &DevTree) -> fdt_rs::error::Result<Option<(u64, u64)>> {
    use fdt_rs::base::DevTree;
    use fdt_rs::prelude::{FallibleIterator, PropReader};
    let mut nodes = dev_tree.nodes();
    let mut memory = None;
    while let Some(item) = nodes.next()? {
        if item.name()?.starts_with("memory") {
            memory = Some(item);
            break;
        }
    }
    let memory = match memory {
        Some(node) => node,
        None => panic!("no memory"),
    };

    println!("{:?}", memory.name()?);
    let mut props = memory.props();
    while let Some(prop) = props.next()? {
        if prop.name().unwrap() == "reg" {
            let start = prop.u64(0)?;
            let size = prop.u64(1)?;
            return Ok(Some((start, size)));
        }
    }
    return Ok(None);
}

fn init_heap(dev_tree: &DevTree) -> memory::Arena<'static, crate::mem::Page> {
    extern "C" {
        static mut _heap_start: u64;
    }

    let heap_start: *mut u8 = unsafe { &mut _heap_start as *mut u64 as *mut u8 };
    let (start, size) = get_memory(dev_tree).unwrap().unwrap();
    assert!(heap_start >= start as *mut u8);
    assert!(heap_start < (start + size) as *mut u8);
    let heap_size = size - (heap_start as u64 - start);
    let pages = heap_size as usize / crate::mem::PAGESIZE;
    assert!(pages * crate::mem::PAGESIZE <= (heap_size as usize));

    let pages =
        unsafe { core::slice::from_raw_parts_mut(heap_start as *mut crate::mem::Page, pages) };
    let mem = memory::Arena::new(pages);
    println!("{:?}", &mem);
    mem
}

fn init_caps(mem: memory::Arena<'static, crate::mem::Page>) -> Result<(), caps::Error> {
    let mut init_memcap = {
        let content = caps::Memory { inner: mem };
        caps::Cap::from_content(content)
    };

    let mut init_cspace = caps::CSpace::init_sz(&mut init_memcap, 8)?;
    let slot = init_cspace.get_slot_mut(0)?;
    slot.set(caps::Memory::init_sz(&mut init_memcap, 10)?)?;
    Ok(())
}

#[no_mangle]
extern "C" fn kernel_main(_hartid: usize, _unused: usize, dtb: *mut u8) {
    // parse device tree from bootloader
    let device_tree = unsafe { DevTree::from_raw_pointer(dtb).unwrap() };

    // setup uart
    let uart = unsafe { Uart::from_device_tree(&device_tree).unwrap() };
    {
        (*UART_DEVICE.spin_lock()) = Some(uart);
    }

    // setup page heap
    // after this operation, the device tree was overwritten
    let mut mem = init_heap(&device_tree);
    drop(device_tree);
    drop(dtb);

    // setup context switching
    let stack_pages = mem.alloc_many_raw(10).unwrap();
    let trap_frame = unsafe {
        TrapFrame::null_from_stack(
            stack_pages.cast::<usize>(),
            crate::mem::PAGESIZE / core::mem::size_of::<usize>(),
        )
    };
    unsafe {
        arch::asm_utils::write_sscratch(&trap_frame as *const TrapFrame as usize);
    }
    arch::trap::enable_interrupts();

    // do the actual kernel logic
    for i in 0..5 {
        println!("Hello World from Kernel Land Nr {}", i);
    }

    init_caps(mem).unwrap();
    unsafe {
        let null_deref = *(0 as *mut u8);
        println!("{null_deref}");
    };

    // shut down the machine
    let shutdown_device: &mut SifiveShutdown = unsafe { &mut *(0x100_000 as *mut SifiveShutdown) };
    unsafe { shutdown_device.shutdown(ShutdownCode::Pass) };
}
