use alloc::vec::Vec;
use embedded_graphics::{pixelcolor, prelude::DrawTarget};

use crate::{
    draw::DrawBuffer,
    gpu::{GpuDriver, GpuFramebuffer},
};

pub struct FramebufferWriter {
    pub gpu: GpuDriver,
    pub fb: GpuFramebuffer,
    pub vga: VGAWriter,
}

impl FramebufferWriter {
    fn flush(&mut self) {
        let mut target = DrawBuffer {
            buf: &mut self.fb.buf,
            width: self.fb.width,
            height: self.fb.height,
        };
        render_vga_buffer(&mut target, &mut self.vga.vga).unwrap();
        self.gpu.draw_resource(&self.fb);
    }
}

impl core::fmt::Write for FramebufferWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.vga.write_str(s)?;
        self.flush();
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.vga.write_str(c.encode_utf8(&mut [0; 4]))
    }

    fn write_fmt(&mut self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        core::fmt::write(&mut self.vga, args)?;
        self.flush();
        Ok(())
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct VGAChar {
    pub char: u8,
    pub damaged: bool,
}

pub struct VGABuffer {
    pub buf: Vec<VGAChar>,
    pub width: u32,
    pub height: u32,
}

pub struct VGAWriter {
    pub vga: VGABuffer,
    pub pos_x: u32,
    pub pos_y: u32,
}

impl VGAWriter {
    fn scroll(&mut self) {
        for line in 1..self.vga.height {
            for col in 0..self.vga.width {
                let this = line * self.vga.width + col;
                let prev = (line - 1) * self.vga.width + col;
                self.vga.buf[prev as usize] = self.vga.buf[this as usize];
                self.vga.buf[prev as usize].damaged = true;
            }
        }
        for col in 0..self.vga.width {
            let pos = (self.vga.height - 1) * self.vga.width + col;
            self.vga.buf[pos as usize].char = b' ';
            self.vga.buf[pos as usize].damaged = true;
        }
        self.pos_y = self.vga.height - 1;
    }
}

impl core::fmt::Write for VGAWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for &b in s.as_bytes() {
            match b {
                b'\n' => {
                    self.pos_x = 0;
                    self.pos_y += 1;
                    if self.pos_y >= self.vga.height {
                        self.scroll();
                    }
                }
                other => {
                    let idx = self.pos_y * self.vga.width + self.pos_x;
                    self.vga.buf[idx as usize].char = other;
                    self.vga.buf[idx as usize].damaged = true;
                    self.pos_x = (self.pos_x + 1) % self.vga.width;
                    if self.pos_x == 0 {
                        self.pos_y += 1;
                    }
                    if self.pos_y >= self.vga.height {
                        self.scroll();
                    }
                }
            }
        }
        Ok(())
    }
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
