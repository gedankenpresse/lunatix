use liblunatix::println;

use crate::event_codes::*;
use crate::{Event, Keyboard};

#[derive(Default)]
pub struct Neo2Keyboard {
    shift: bool,
    alt: bool,
    ctrl: bool,
    mod3: bool,
    mod4: bool,
}

impl Neo2Keyboard {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn layer(&self) -> Layer {
        match (self.shift, self.mod3, self.mod4) {
            (false, false, false) => Layer::Lowercase,
            (true, false, _) => Layer::Uppercase,
            (false, true, false) => Layer::Programming,
            (false, false, true) => Layer::Meta,
            (true, true, _) => Layer::Greek,
            (false, true, true) => Layer::Math,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Layer {
    Lowercase,
    Uppercase,
    Programming,
    Meta,
    Greek,
    Math,
}

impl Keyboard for Neo2Keyboard {
    fn process_evdev_event(&mut self, event: Event) -> Option<char> {
        // Determine Layer changes
        match event.code {
            KEY_LEFTSHIFT => {
                self.shift = event.value != 0;
                return None;
            }
            KEY_CAPSLOCK => {
                self.mod3 = event.value != 0;
                return None;
            }
            KEY_LEFTCTRL => {
                self.ctrl = event.value != 0;
                return None;
            }
            _ => {}
        }

        // Skip keyup events
        if event.value == 0 {
            return None;
        }

        let current_layer = self.layer();
        use Layer::*;
        let char = match (current_layer, event.code) {
            (_, KEY_SPACE) => ' ',
            (_, KEY_ENTER) => '\n',
            (_, KEY_BACKSPACE) => '\x7f',
            (Lowercase, KEY_1) => '1',
            (Lowercase, KEY_2) => '2',
            (Lowercase, KEY_3) => '3',
            (Lowercase, KEY_4) => '4',
            (Lowercase, KEY_5) => '5',
            (Lowercase, KEY_6) => '6',
            (Lowercase, KEY_7) => '7',
            (Lowercase, KEY_8) => '8',
            (Lowercase, KEY_9) => '9',
            (Lowercase, KEY_0) => '0',
            (Lowercase, KEY_MINUS) => '-',
            (Lowercase, KEY_EQUAL) => '`',

            (Lowercase, KEY_Q) => 'x',
            (Lowercase, KEY_W) => 'v',
            (Lowercase, KEY_E) => 'l',
            (Lowercase, KEY_R) => 'c',
            (Lowercase, KEY_T) => 'w',
            (Lowercase, KEY_Y) => 'k',
            (Lowercase, KEY_U) => 'h',
            (Lowercase, KEY_I) => 'g',
            (Lowercase, KEY_O) => 'f',
            (Lowercase, KEY_P) => 'q',
            (Lowercase, KEY_LEFTBRACE) => 'ß',
            (Lowercase, KEY_A) => 'u',
            (Lowercase, KEY_S) => 'i',
            (Lowercase, KEY_D) => 'a',
            (Lowercase, KEY_F) => 'e',
            (Lowercase, KEY_G) => 'o',
            (Lowercase, KEY_H) => 's',
            (Lowercase, KEY_J) => 'n',
            (Lowercase, KEY_K) => 'r',
            (Lowercase, KEY_L) => 't',
            (Lowercase, KEY_SEMICOLON) => 'd',
            (Lowercase, KEY_APOSTROPHE) => 'y',
            (Lowercase, KEY_Z) => 'ü',
            (Lowercase, KEY_X) => 'ö',
            (Lowercase, KEY_C) => 'ä',
            (Lowercase, KEY_V) => 'p',
            (Lowercase, KEY_B) => 'z',
            (Lowercase, KEY_N) => 'b',
            (Lowercase, KEY_M) => 'm',
            (Lowercase, KEY_COMMA) => ',',
            (Lowercase, KEY_DOT) => '.',
            (Lowercase, KEY_SLASH) => 'j',

            (Uppercase, KEY_1) => '°',
            (Uppercase, KEY_2) => '§',
            (Uppercase, KEY_3) => 'ℓ',
            (Uppercase, KEY_4) => '»',
            (Uppercase, KEY_5) => '«',
            (Uppercase, KEY_6) => '$',
            (Uppercase, KEY_7) => '€',
            (Uppercase, KEY_8) => '„',
            (Uppercase, KEY_9) => '“',
            (Uppercase, KEY_0) => '”',
            (Uppercase, KEY_MINUS) => '—',
            (Uppercase, KEY_EQUAL) => '¸',

            (Uppercase, KEY_Q) => 'X',
            (Uppercase, KEY_W) => 'V',
            (Uppercase, KEY_E) => 'L',
            (Uppercase, KEY_R) => 'C',
            (Uppercase, KEY_T) => 'W',
            (Uppercase, KEY_Y) => 'K',
            (Uppercase, KEY_U) => 'H',
            (Uppercase, KEY_I) => 'G',
            (Uppercase, KEY_O) => 'F',
            (Uppercase, KEY_P) => 'Q',
            (Uppercase, KEY_LEFTBRACE) => 'ẞ',
            (Uppercase, KEY_A) => 'U',
            (Uppercase, KEY_S) => 'I',
            (Uppercase, KEY_D) => 'A',
            (Uppercase, KEY_F) => 'E',
            (Uppercase, KEY_G) => 'O',
            (Uppercase, KEY_H) => 'S',
            (Uppercase, KEY_J) => 'N',
            (Uppercase, KEY_K) => 'R',
            (Uppercase, KEY_L) => 'T',
            (Uppercase, KEY_SEMICOLON) => 'D',
            (Uppercase, KEY_APOSTROPHE) => 'Y',
            (Uppercase, KEY_Z) => 'Ü',
            (Uppercase, KEY_X) => 'Ö',
            (Uppercase, KEY_C) => 'Ä',
            (Uppercase, KEY_V) => 'P',
            (Uppercase, KEY_B) => 'Z',
            (Uppercase, KEY_N) => 'Z',
            (Uppercase, KEY_M) => 'B',
            (Uppercase, KEY_COMMA) => '–',
            (Uppercase, KEY_DOT) => '•',
            (Uppercase, KEY_SLASH) => 'J',

            (Programming, KEY_W) => '_',
            _ => {
                println!("{:?} {}", current_layer, event.code);
                return None;
            }
        };
        return Some(char);
    }
}
