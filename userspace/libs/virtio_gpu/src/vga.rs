use core::{cell::RefCell, fmt::Write};

use alloc::{rc::Rc, vec::Vec};
use embedded_graphics::{pixelcolor, prelude::DrawTarget};

use crate::{
    draw::DrawBuffer,
    gpu::{GpuDriver, GpuFramebuffer},
};

pub struct FramebufferFlushWriter {
    pub gpu: Rc<RefCell<GpuDriver>>,
    pub fb_writer: FramebufferWriter,
}

impl FramebufferFlushWriter {
    fn flush(&mut self) {
        self.gpu.borrow_mut().draw_resource(&self.fb_writer.fb);
    }
}

impl Write for FramebufferFlushWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.fb_writer.write_str(s)?;
        self.flush();
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.fb_writer.write_char(c)?;
        self.flush();
        Ok(())
    }

    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        self.fb_writer.write_fmt(args)?;
        self.flush();
        Ok(())
    }
}

#[derive(Default)]
pub struct Pos {
    x: u32,
    y: u32,
}

pub struct VGABuffer {
    pub buf: Vec<VGAChar>,
    pub width: u32,
    pub height: u32,
}

pub struct FramebufferWriter {
    pub fb: GpuFramebuffer,
    pub vga: VGABuffer,
    pub pos: Pos,
}

impl FramebufferWriter {
    fn scroll(&mut self) {
        let range = self.vga.width as usize..self.vga.buf.len();
        self.vga.buf.copy_within(range, 0);

        let lineheight = 14;
        let range = self.fb.width as usize * lineheight..self.fb.buf.len();
        self.fb.buf.copy_within(range, 0);
        /*
        for line in 1..self.vga.height {
            for col in 0..self.vga.width {
                let this = line * self.vga.width + col;
                let prev = (line - 1) * self.vga.width + col;
                self.vga.buf[prev as usize] = self.vga.buf[this as usize];
                self.vga.buf[prev as usize].damaged = true;
            }
        }
        */
        for col in 0..self.vga.width {
            let pos = (self.vga.height - 1) * self.vga.width + col;
            self.vga.buf[pos as usize].char = b' ';
            self.vga.buf[pos as usize].damaged = true;
        }
        self.pos.y = self.vga.height - 1;
    }
}

impl core::fmt::Write for FramebufferWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &b in s.as_bytes() {
            match b {
                b'\n' => {
                    self.pos.x = 0;
                    self.pos.y += 1;
                    if self.pos.y >= self.vga.height {
                        self.scroll();
                    }
                }
                other => {
                    let idx = self.pos.y * self.vga.width + self.pos.x;
                    self.vga.buf[idx as usize].char = other;
                    self.vga.buf[idx as usize].damaged = true;
                    self.pos.x = (self.pos.x + 1) % self.vga.width;
                    if self.pos.x == 0 {
                        self.pos.y += 1;
                    }
                    if self.pos.y >= self.vga.height {
                        self.scroll();
                    }
                }
            }
        }
        let mut draw = DrawBuffer {
            buf: self.fb.buf,
            width: self.fb.width,
            height: self.fb.height,
        };
        render_vga_buffer(&mut draw, &mut self.vga).unwrap();
        Ok(())
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct VGAChar {
    pub char: u8,
    pub damaged: bool,
}

pub fn render_vga_buffer<'b, E, T: DrawTarget<Color = pixelcolor::Rgb888, Error = E>>(
    target: &mut T,
    vga: &mut VGABuffer,
) -> Result<(), E> {
    use embedded_graphics::prelude::*;
    use embedded_graphics::{
        mono_font::{ascii, MonoTextStyleBuilder},
        text::*,
    };

    // Create a new character style
    let style = MonoTextStyleBuilder::new()
        .font(&ascii::FONT_7X14)
        .text_color(RgbColor::WHITE)
        .background_color(RgbColor::BLACK)
        .build();

    // Create a new text style.
    let text_style = TextStyleBuilder::new().alignment(Alignment::Left).build();

    for line in 0..vga.height {
        for col in 0..vga.width {
            let vga_char = &mut vga.buf[(line * vga.width + col) as usize];
            if !vga_char.damaged {
                continue;
            }
            vga_char.damaged = false;
            let x_off = Point::new(col as i32, 0) * 7;
            let y_off = Point::new(0, line as i32) * 14;
            let point = Point::new(1, 11) + x_off + y_off;
            let char = if (vga_char.char as char).is_ascii_graphic() {
                vga_char.char
            } else {
                b' '
            };
            let mut tmp = [0u8; 4];
            let your_string = (char as char).encode_utf8(&mut tmp);
            Text::with_text_style(your_string, point, style, text_style).draw(target)?;
        }
    }

    Ok(())
}
