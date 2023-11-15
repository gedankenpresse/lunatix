use crate::Event;

pub trait Keyboard {
    fn process_evdev_event(&mut self, event: Event) -> Option<char>;
}

pub trait CreateKeyboard {
    fn create() -> Self;
}

mod quertz;
pub use quertz::QuertzKeyboard;
