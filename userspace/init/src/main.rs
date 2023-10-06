#![no_std]
#![no_main]

mod elfloader;

use crate::elfloader::LunatixElfLoader;
use ::elfloader::ElfBinary;
use core::arch::asm;
use core::panic::PanicInfo;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::syscall_abi::map_page::MapPageFlag;
use librust::syscall_abi::CAddr;
use librust::{print, println, put_c};
use uart_driver::{MmUart, Uart};

static HELLO_WORLD_BIN: &[u8] =
    include_bytes!("../../../target/riscv64imac-unknown-none-elf/release/hello_world");

#[no_mangle]
fn _start() {
    main();
}

const CADDR_MEM: CAddr = 1;
const CADDR_CSPACE: CAddr = 2;
const CADDR_VSPACE: CAddr = 3;
const CADDR_IRQ_CONTROL: CAddr = 4;

#[panic_handler]
fn panic_handler(info: &PanicInfo) -> ! {
    println!("panic {}", info);
    loop {}
}

fn main() {
    //run_second_task();
    handle_interrupts();
    println!("Init task says good bye 👋");
}

fn run_second_task() {
    const CADDR_CHILD_TASK: CAddr = 4;
    librust::derive_from_mem(CADDR_MEM, CADDR_CHILD_TASK, CapabilityVariant::Task, None).unwrap();

    const CADDR_CHILD_CSPACE: CAddr = 5;
    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_CHILD_CSPACE,
        CapabilityVariant::CSpace,
        Some(8),
    )
    .unwrap();
    librust::task_assign_cspace(CADDR_CHILD_CSPACE, CADDR_CHILD_TASK).unwrap();

    const CADDR_CHILD_VSPACE: CAddr = 6;
    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_CHILD_VSPACE,
        CapabilityVariant::VSpace,
        None,
    )
    .unwrap();
    assert_eq!(
        librust::identify(CADDR_CHILD_VSPACE).unwrap(),
        CapabilityVariant::VSpace
    );
    librust::task_assign_vspace(CADDR_CHILD_VSPACE, CADDR_CHILD_TASK).unwrap();

    println!("loading HelloWorld binary");
    // load a stack for the child task
    const CADDR_CHILD_STACK_PAGE: CAddr = 7;
    const CHILD_STACK_LOW: usize = 0x5_0000_0000;
    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_CHILD_STACK_PAGE,
        CapabilityVariant::Page,
        None,
    )
    .unwrap();
    librust::map_page(
        CADDR_CHILD_STACK_PAGE,
        CADDR_CHILD_VSPACE,
        CADDR_MEM,
        CHILD_STACK_LOW,
        MapPageFlag::READ | MapPageFlag::WRITE,
    )
    .unwrap();
    // load binary elf code
    const CADDR_CHILD_PAGE_START: CAddr = 8;
    let elf_binary = ElfBinary::new(HELLO_WORLD_BIN).unwrap();
    let mut elf_loader = LunatixElfLoader::<8>::new(
        CADDR_MEM,
        CADDR_VSPACE,
        CADDR_CHILD_VSPACE,
        CADDR_CHILD_PAGE_START,
        0x0000003000000000,
    );
    elf_binary.load(&mut elf_loader).unwrap();
    librust::task_assign_control_registers(
        CADDR_CHILD_TASK,
        elf_binary.entry_point() as usize,
        CHILD_STACK_LOW + 4096,
        0x0,
        0x0,
    )
    .unwrap();
    println!("Yielding to Hello World Task");
    librust::yield_to(CADDR_CHILD_TASK).unwrap();
}

fn read_char_blocking(uart: &Uart, noti: CAddr, irq: CAddr) -> u8 {
    let _ = librust::wait_on(noti).unwrap();
    let c = unsafe { uart.read_data() };
    librust::irq_complete(irq).unwrap();
    return c;
}

fn handle_interrupts() {
    assert_eq!(
        librust::identify(CADDR_IRQ_CONTROL).unwrap(),
        CapabilityVariant::IrqControl
    );

    const CADDR_NOTIFICATION: CAddr = 6;
    librust::derive_from_mem(
        CADDR_MEM,
        CADDR_NOTIFICATION,
        CapabilityVariant::Notification,
        None,
    )
    .unwrap();

    const CADDR_CLAIMED_IRQ: CAddr = 5;
    const UART_INTERRUPT_LINE: usize = 0xa;
    librust::irq_control_claim(
        CADDR_IRQ_CONTROL,
        UART_INTERRUPT_LINE,
        CADDR_CLAIMED_IRQ,
        CADDR_NOTIFICATION,
    )
    .unwrap();
    assert_eq!(
        librust::identify(CADDR_CLAIMED_IRQ).unwrap(),
        CapabilityVariant::Irq
    );

    // TODO: allocate pages for this memory map yourself
    let uart = unsafe { Uart::from_ptr(0x10000000 as *mut MmUart) };
    let mut buf = [0u8; 256];
    let mut pos: isize = 0;
    print!("> ");
    loop {
        let c = read_char_blocking(&uart, CADDR_NOTIFICATION, CADDR_CLAIMED_IRQ);
        //print!("{}", c);
        match c as char {
            // handle backspace
            '\x7f' => {
                buf[pos as usize] = 0;
                if pos > 0 {
                    print!("\x08 \x08");
                }
                pos = core::cmp::max(pos - 1, 0);
            }

            // handle carriage return
            '\x0d' => {
                // process command
                let cmd = core::str::from_utf8(&buf)
                    .expect("could not interpret char buffer as string")
                    .trim_end_matches('\0')
                    .trim_end();
                process_command(cmd);

                // reset buffer
                pos = 0;
                for c in buf.iter_mut() {
                    *c = 0;
                }
                print!("> ");
            }

            // append any other character to buffer
            _ => {
                buf[pos as usize] = c;
                print!("{}", c as char);
                pos = core::cmp::min(pos + 1, buf.len() as isize - 1);
            }
        }
    }
}

fn process_command(cmd: &str) {
    print!("\n");
    match cmd {
        "help" => {
            println!("Available commands: help, shutdown");
        }
        "shutdown" => {
            println!("Shutting down system");
            panic!("implement shutdown syscall");
        }
        _ => {
            println!("Unknown command. Enter 'help' for a list of commands");
        }
    }
}
