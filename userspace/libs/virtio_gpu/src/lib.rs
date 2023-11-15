#![no_std]

use core::cell::RefCell;

use alloc::{rc::Rc, vec};
use gpu::GpuDriver;
use liblunatix::{prelude::CAddr, println};
use vga::FramebufferFlushWriter;

use crate::vga::{Pos, VGABuffer, VGAChar};

extern crate alloc;

pub mod draw;
pub mod gpu;
pub mod vga;

pub fn create_gpu_writer(
    gpu: Rc<RefCell<GpuDriver>>,
    mem: CAddr,
    vspace: CAddr,
    cspace_bits: usize,
) -> FramebufferFlushWriter {
    let mut driver = gpu.borrow_mut();
    let display = driver.get_displays()[0].clone();
    let width = display.rect.width.get();
    let height = display.rect.height.get();
    println!("width: {width}, height: {height}");
    let fb = driver.create_resource(mem, vspace, 0xdeadbeef, 0, width, height, cspace_bits);
    drop(driver);

    let vga_width = fb.width / 7;
    let vga_height = (fb.height / 14) - 2;
    let vga_buf = vec![VGAChar::default(); (vga_width * vga_height) as usize];
    let vga = VGABuffer {
        buf: vga_buf,
        width: vga_width,
        height: vga_height,
    };
    let fb_writer = vga::FramebufferFlushWriter {
        gpu,
        fb_writer: vga::FramebufferWriter {
            pos: Pos::default(),
            fb,
            vga,
        },
    };

    return fb_writer;
}
