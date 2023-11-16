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
