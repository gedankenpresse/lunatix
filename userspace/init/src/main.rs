#![no_std]
#![no_main]

mod commands;
mod elfloader;
mod second_task;

use crate::commands::KNOWN_COMMANDS;
use core::panic::PanicInfo;
use fdt_rs::base::{DevTree, DevTreeNode, DevTreeProp};
use fdt_rs::prelude::*;
use librust::syscall_abi::identify::CapabilityVariant;
use librust::syscall_abi::CAddr;
use librust::{print, println};
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
const CADDR_DEVMEM: CAddr = 5;
const CADDR_UART_IRQ: CAddr = 6;
const CADDR_UART_NOTIFICATION: CAddr = 7;

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

fn devtree_prop<'a, 'dt: 'a>(
    node: &'a DevTreeNode<'a, 'dt>,
    name: &str,
) -> Option<DevTreeProp<'a, 'dt>> {
    let mut props = node.props();
    while let Ok(Some(prop)) = props.next() {
        if prop.name() != Ok(name) {
            continue;
        }
        return Some(prop);
    }
    None
}

fn init_uart<'dt>(dt: &DevTree<'dt>) -> Result<Uart<'static>, &'static str> {
    let Ok(Some(node)) = dt.compatible_nodes("ns16550a").next() else { return Err("no compatible node found")};
    let Some(reg) = devtree_prop(&node, "reg") else { return Err("no reg prop")};
    let base = reg.u64(0).unwrap();
    let len = reg.u64(1).unwrap();
    let Some(interrupts) = devtree_prop(&node, "interrupts") else { return Err("no interrupt prop") };
    let interrupt = interrupts.u32(0).unwrap();

    librust::derive_from_mem(
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

    librust::map_devmem(CADDR_DEVMEM, CADDR_MEM, base as usize, len as usize).unwrap();
    let mut uart = unsafe { Uart::from_ptr(base as *mut MmUart) };
    uart.enable_rx_interrupts();
    Ok(uart)
}

fn main() {
    let dev_tree_address: usize = 0x20_0000_0000;
    let dev_tree = unsafe { DevTree::from_raw_pointer(dev_tree_address as *const u8).unwrap() };

    handle_interrupts(&dev_tree);
    println!("Init task says good bye 👋");
}

fn read_char_blocking(uart: &Uart) -> u8 {
    let _ = librust::wait_on(CADDR_UART_NOTIFICATION).unwrap();
    let c = unsafe { uart.read_data() };
    librust::irq_complete(CADDR_UART_IRQ).unwrap();
    return c;
}

fn handle_interrupts<'dt>(dt: &DevTree<'dt>) {
    assert_eq!(
        librust::identify(CADDR_IRQ_CONTROL).unwrap(),
        CapabilityVariant::IrqControl
    );

    let uart = match init_uart(dt) {
        Ok(uart) => uart,
        Err(err) => panic!("{}", err),
    };
    let mut buf = [0u8; 256];
    loop {
        let cmd = read_cmd(&uart, &mut buf);
        process_cmd(cmd);
    }
}

fn read_cmd<'a, 'b>(uart: &'a Uart, buf: &'b mut [u8]) -> &'b str {
    // reset buffer
    let mut pos: isize = 0;
    for c in buf.iter_mut() {
        *c = 0;
    }

    print!("> ");

    loop {
        let c = read_char_blocking(&uart);
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
                return core::str::from_utf8(&buf[0..pos as usize])
                    .expect("could not interpret char buffer as string");
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

fn process_cmd(input: &str) {
    print!("\n");
    match KNOWN_COMMANDS
        .iter()
        .find(|i| input.starts_with(i.get_name()))
    {
        None => println!(
            "Unknown command {:?}. Enter 'help' for a list of commands",
            input
        ),
        Some(cmd) => cmd
            .execute(input.strip_prefix(cmd.get_name()).unwrap().trim_start())
            .expect("Could not execute command"),
    };
}
