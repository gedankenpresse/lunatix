use alloc::vec::Vec;
use embedded_graphics::{
    pixelcolor,
    prelude::{DrawTarget, OriginDimensions, RgbColor, Size},
    Pixel,
};

pub struct DrawBuffer<'b> {
    pub buf: &'b mut [u32],
    pub width: u32,
    pub height: u32,
}

impl<'b> OriginDimensions for DrawBuffer<'b> {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}

impl<'b> DrawTarget for DrawBuffer<'b> {
    type Color = pixelcolor::Rgb888;

    type Error = ();

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (63,63)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if coord.x < 0 || coord.x >= self.width as i32 {
                panic!("x out of range: x is {}, width is {}", coord.x, self.width);
            }
            if coord.y < 0 || coord.y >= self.height as i32 {
                panic!(
                    "y out of range: y is {}, height is {}",
                    coord.y, self.height
                );
            }
            let x = coord.x as u32;
            let y = coord.y as u32;
            // Calculate the index in the framebuffer.
            let index: u32 = x + y * self.width;
            let px = (color.r() as u32) << 8 | (color.g() as u32) << 16 | (color.b() as u32) << 24;
            self.buf[index as usize] = px;
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Default)]
pub struct VGAChar {
    pub char: u8,
}

pub struct VGABuffer {
    pub buf: Vec<VGAChar>,
    pub width: u32,
    pub height: u32,
}

pub fn render_vga_buffer<'b, E, T: DrawTarget<Color = pixelcolor::Rgb888, Error = E>>(
    target: &mut T,
    vga: &VGABuffer,
) -> Result<(), E> {
    use embedded_graphics::prelude::*;
    use embedded_graphics::{
        mono_font::{ascii, MonoTextStyleBuilder},
        text::*,
    };

    // Create a new character style
    let style = MonoTextStyleBuilder::new()
        .font(&ascii::FONT_7X14)
        .text_color(RgbColor::RED)
        .build();

    // Create a new text style.
    let text_style = TextStyleBuilder::new().alignment(Alignment::Left).build();

    for line in 0..vga.height {
        for col in 0..vga.width {
            let x_off = Point::new(col as i32, 0) * 7;
            let y_off = Point::new(0, line as i32) * 14;
            let point = Point::new(1, 13) + x_off + y_off;
            let vga_char = vga.buf[(line * vga.width + col) as usize];
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
