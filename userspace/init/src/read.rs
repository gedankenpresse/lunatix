use alloc::vec;
use alloc::vec::Vec;
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

    fn read_to_vec(&mut self) -> Result<Vec<u8>, ()> {
        let mut result = Vec::new();

        loop {
            let mut read_buf = vec![0u8; 4096];
            let read = self.read(&mut read_buf)?;
            if read == 0 {
                break;
            } else {
                result.extend_from_slice(&read_buf[..read])
            }
        }

        Ok(result)
    }
}
