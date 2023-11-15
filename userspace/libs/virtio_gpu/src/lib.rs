#![no_std]

use core::fmt::Write;

use alloc::{boxed::Box, vec};
use liblunatix::{prelude::CAddr, println};
use vga::FramebufferWriter;

use crate::vga::{VGABuffer, VGAChar};

extern crate alloc;

pub mod draw;
pub mod gpu;
pub mod vga;

pub fn create_gpu_writer(
    mem: CAddr,
    vspace: CAddr,
    devmem: CAddr,
    irq_control: CAddr,
    cspace_bits: usize,
) -> FramebufferWriter {
    let mut gpu = gpu::init_gpu_driver(mem, vspace, devmem, irq_control);
    let display = gpu.get_displays()[0].clone();
    let width = display.rect.width.get();
    let height = display.rect.height.get();
    println!("width: {width}, height: {height}");
    let fb = gpu.create_resource(mem, vspace, 0xdeadbeef, 0, width, height, cspace_bits);

    let vga_width = fb.width / 7;
    let vga_height = (fb.height / 14) - 2;
    let vga_buf = vec![VGAChar::default(); (vga_width * vga_height) as usize];
    let vga = VGABuffer {
        buf: vga_buf,
        width: vga_width,
        height: vga_height,
    };
    let vga_writer = vga::VGAWriter {
        vga,
        pos_x: 0,
        pos_y: 0,
    };
    let fb_writer = vga::FramebufferWriter {
        gpu,
        fb,
        vga: vga_writer,
    };

    return fb_writer;
}
