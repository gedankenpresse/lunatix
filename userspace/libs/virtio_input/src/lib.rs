#![no_std]

use io::read::ByteReader;

pub mod input;
pub mod keyboards;

pub use keyboards::Keyboard;

pub use input::{init_input_driver, Event, EventType, InputDriver};

pub struct VirtioByteReader<K: Keyboard> {
    pub input: InputDriver,
    pub keyboard: K,
}

impl<K: Keyboard> ByteReader for VirtioByteReader<K> {
    #[rustfmt::skip]
    fn read_byte(&mut self) -> Result<u8, ()> {
        loop {
            let event = unsafe { self.input.read_event() };
            let char = self.keyboard.process_evdev_event(event);
            if let Some(c) = char {
                return Ok(c as u8);
            }
        }
    }
}
