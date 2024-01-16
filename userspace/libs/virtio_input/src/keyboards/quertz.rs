use liblunatix::println;

use crate::{Event, EventType};

use super::{CreateKeyboard, Keyboard};

pub struct QwertzKeyboard {
    shift: bool,
}

impl QwertzKeyboard {
    pub fn new() -> Self {
        Self { shift: false }
    }
}

impl CreateKeyboard for QwertzKeyboard {
    fn create() -> Self {
        Self { shift: false }
    }
}

impl QwertzKeyboard {
    fn ascii_alpha(&self, c: u8) -> u8 {
        if self.shift {
            return (c as char).to_ascii_uppercase() as u8;
        } else {
            return c;
        }
    }

    fn ascii_num(&self, c: u8) -> u8 {
        return c;
    }
}

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h
impl Keyboard for QwertzKeyboard {
    #[rustfmt::skip]
    fn process_evdev_event(&mut self, event: Event) -> Option<char> {
        match (self.shift, event) {
            (_, Event { event_type: EventType::Key, value: 1, code: 2 }) => Some(self.ascii_num(b'1') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 4 }) => Some(self.ascii_num(b'3') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 5 }) => Some(self.ascii_num(b'4') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 6 }) => Some(self.ascii_num(b'5') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 3 }) => Some(self.ascii_num(b'2') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 7 }) => Some(self.ascii_num(b'6') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 8 }) => Some(self.ascii_num(b'7') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 9 }) => Some(self.ascii_num(b'8') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 10 }) => Some(self.ascii_num(b'9') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 11 }) => Some(self.ascii_num(b'0') as char),

            (_, Event { event_type: EventType::Key, value: 1, code: 16 }) => Some(self.ascii_alpha(b'q') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 17 }) => Some(self.ascii_alpha(b'w') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 18 }) => Some(self.ascii_alpha(b'e') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 19 }) => Some(self.ascii_alpha(b'r') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 20 }) => Some(self.ascii_alpha(b't') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 21 }) => Some(self.ascii_alpha(b'z') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 22 }) => Some(self.ascii_alpha(b'u') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 23 }) => Some(self.ascii_alpha(b'i') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 24 }) => Some(self.ascii_alpha(b'o') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 25 }) => Some(self.ascii_alpha(b'p') as char),

            (_, Event { event_type: EventType::Key, value: 1, code: 28 }) => Some(b'\n' as char),

            (_, Event { event_type: EventType::Key, value: 1, code: 30 }) => Some(self.ascii_alpha(b'a') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 31 }) => Some(self.ascii_alpha(b's') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 32 }) => Some(self.ascii_alpha(b'd') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 33 }) => Some(self.ascii_alpha(b'f') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 34 }) => Some(self.ascii_alpha(b'g') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 35 }) => Some(self.ascii_alpha(b'h') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 36 }) => Some(self.ascii_alpha(b'j') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 37 }) => Some(self.ascii_alpha(b'k') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 38 }) => Some(self.ascii_alpha(b'l') as char),


            (_ , Event { event_type: EventType::Key, value: 1, code: 42 }) => {
                self.shift = true;
                return None;
            }
            (_ , Event { event_type: EventType::Key, value: 0, code: 42 }) => {
                self.shift = false;
                return None;
            }

            (_, Event { event_type: EventType::Key, value: 1, code: 44 }) => Some(self.ascii_alpha(b'y') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 45 }) => Some(self.ascii_alpha(b'x') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 46 }) => Some(self.ascii_alpha(b'c') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 47 }) => Some(self.ascii_alpha(b'v') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 48 }) => Some(self.ascii_alpha(b'b') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 49 }) => Some(self.ascii_alpha(b'n') as char),
            (_, Event { event_type: EventType::Key, value: 1, code: 50 }) => Some(self.ascii_alpha(b'm') as char),

            (_, Event { event_type: EventType::Key, value: 1, code: 52 }) => Some(self.ascii_alpha(b'.') as char),

            (true, Event { event_type: EventType::Key, value: 1, code: 53 }) => Some(self.ascii_alpha(b'_') as char),
            (false, Event { event_type: EventType::Key, value: 1, code: 53 }) => Some(self.ascii_alpha(b'-') as char),

            (_, Event { event_type: EventType::Key, value: 1, code: 57 }) => Some(b' ' as char),
            _ => {
                if event.value == 1 {
                    println!("{:?}", event);
                }
                return None;
            }
        }
    }
}
