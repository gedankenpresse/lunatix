use core::mem;
use core::str::FromStr;
use ini_core::{Item, Parser};

#[derive(Debug, Eq, PartialEq)]
pub struct CapSpec<'src> {
    pub caddr: usize,
    pub typ: &'src str,
    pub args: CapArgs<'src>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CapArgs<'src> {
    raw: &'src str,
}

pub struct Capabilities<'src> {
    pub(super) parser: Parser<'src>,
    pub(super) is_done: bool,
}

impl<'src> Iterator for Capabilities<'src> {
    type Item = CapSpec<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        match self.parser.next() {
            None => None,
            Some(item) => match item {
                Item::SectionEnd => {
                    self.is_done = true;
                    None
                }
                Item::Property(key, Some(value)) => match CapSpec::try_from((key, value)) {
                    Err(_) => None,
                    Ok(result) => Some(result),
                },
                _ => None,
            },
        }
    }
}

impl<'src> CapArgs<'src> {
    pub fn get(&self, key: &str) -> Option<&'src str> {
        self.iter()
            .find_map(|(i_key, i_value)| if i_key == key { Some(i_value) } else { None })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&'src str, &'src str)> {
        self.raw.split(",").filter_map(|i| i.split_once("="))
    }
}

impl<'src> TryFrom<(&'src str, &'src str)> for CapSpec<'src> {
    type Error = ();

    fn try_from(value: (&'src str, &'src str)) -> Result<Self, Self::Error> {
        let (key, value) = value;
        let caddr = usize::from_str(key).map_err(|_| ())?;
        let (typ, args) = match value.split_once(",") {
            Some((typ, args)) => (typ, args),
            None => (value, ""),
        };
        Ok(Self {
            caddr,
            typ,
            args: CapArgs { raw: args },
        })
    }
}

fn get_arg<'a, 'b>(src: &'a str, arg: &'b str) -> Option<&'a str> {
    src.split(",")
        .filter_map(|part| part.split_once("="))
        .find_map(|(key, value)| if key == arg { Some(value) } else { None })
}
