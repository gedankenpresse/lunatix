use aligned_vec::AVec;
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

    fn read_to_vec(&mut self, align: usize) -> Result<AVec<u8>, ()> {
        let mut result = AVec::new(align);

        loop {
            let mut read_buf = vec![0u8; 4096];
            let read = self.read(&mut read_buf)?;
            if read == 0 {
                break;
            } else {
                if result.capacity() < read {
                    result.reserve(read - result.capacity());
                }
                for b in &read_buf[..read] {
                    result.push(*b);
                }
            }
        }

        Ok(result)
    }
}
