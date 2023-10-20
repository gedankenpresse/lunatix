use librust::print;

pub trait ByteReader {
    fn read_byte(&mut self) -> Result<u8, ()>;
}

pub struct EchoingByteReader<R: ByteReader>(pub R);

impl<R: ByteReader> ByteReader for EchoingByteReader<R> {
    fn read_byte(&mut self) -> Result<u8, ()> {
        let byte = self.0.read_byte()?;
        match byte as char {
            // handle backspace
            '\x7f' => print!("\x08 \x08"),
            c => print!("{}", c),
        };
        Ok(byte)
    }
}

pub trait Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()>;
}
