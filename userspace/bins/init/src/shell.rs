use io::read::ByteReader;
use librust::{print, println};

use crate::commands::{self, Command};

pub fn shell(reader: &mut dyn ByteReader) {
    let mut buf = [0u8; 256];
    loop {
        let cmd = read_cmd(reader, &mut buf);
        process_cmd(cmd);
    }
}

fn read_cmd<'b>(reader: &mut dyn ByteReader, buf: &'b mut [u8]) -> &'b str {
    // reset buffer
    let mut pos: isize = 0;
    for c in buf.iter_mut() {
        *c = 0;
    }

    print!("> ");

    loop {
        let c = reader.read_byte().unwrap();
        match c as char {
            // handle backspace
            '\x7f' => {
                buf[pos as usize] = 0;
                pos = core::cmp::max(pos - 1, 0);
            }

            // handle carriage return
            '\x0d' => {
                return core::str::from_utf8(&buf[0..pos as usize])
                    .expect("could not interpret char buffer as string");
            }
            // append any other character to buffer
            _ => {
                buf[pos as usize] = c;
                pos = core::cmp::min(pos + 1, buf.len() as isize - 1);
            }
        }
    }
}

struct Help;

impl Command for Help {
    fn get_name(&self) -> &'static str {
        "help"
    }

    fn get_summary(&self) -> &'static str {
        "help for this command"
    }

    fn execute(&self, _args: &str) -> Result<(), &'static str> {
        println!("Known Commands: ");
        for cmd in KNOWN_COMMANDS {
            println!("\t {: <12} {}", cmd.get_name(), cmd.get_summary());
        }
        Ok(())
    }
}

const KNOWN_COMMANDS: &[&'static dyn Command] = &[
    &commands::Echo,
    &commands::Shutdown,
    &Help,
    &commands::Identify,
    &commands::Destroy,
    &commands::Copy,
    &commands::Cat,
    &commands::Ls,
    &commands::Exec,
];

fn process_cmd(input: &str) {
    print!("\n");

    let Some(cmd) = KNOWN_COMMANDS
        .iter()
        .find(|i| input.starts_with(i.get_name()))
    else {
        println!(
            "Unknown command {:?}. Enter 'help' for a list of commands",
            input
        );
        return;
    };
    match cmd.execute(input.strip_prefix(cmd.get_name()).unwrap().trim_start()) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
