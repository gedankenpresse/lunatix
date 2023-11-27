use core::str::FromStr;
use ini_core::{Item, Parser};

#[derive(Debug, Eq, PartialEq)]
pub struct CapPlacement {
    pub caddr: usize,
    pub spec: CapSpec,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CapSpec {
    CSpace { source: CSpaceSource },
    Irq { line: usize },
    Memory { min_size_bytes: usize },
}

#[derive(Debug, Eq, PartialEq)]
pub enum CSpaceSource {
    OwnCSpace,
}

pub struct Capabilities<'src> {
    pub(super) parser: Parser<'src>,
    pub(super) is_done: bool,
}

impl<'src> Iterator for Capabilities<'src> {
    type Item = CapPlacement;

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
                Item::Property(key, Some(value)) => match CapPlacement::try_from((key, value)) {
                    Err(_) => None,
                    Ok(result) => Some(result),
                },
                _ => None,
            },
        }
    }
}

impl<'src> TryFrom<(&'src str, &'src str)> for CapPlacement {
    type Error = ();

    fn try_from(value: (&'src str, &'src str)) -> Result<Self, Self::Error> {
        let (key, value) = value;
        let caddr = usize::from_str(key).map_err(|_| ())?;
        let spec = CapSpec::try_from(value)?;
        Ok(CapPlacement { caddr, spec })
    }
}

impl<'src> TryFrom<&'src str> for CapSpec {
    type Error = ();

    fn try_from(value: &'src str) -> Result<Self, Self::Error> {
        let (typ, args) = value.split_once(",").unwrap_or((value, ""));
        match typ {
            "cspace" => {
                let source = get_arg(args, "source").ok_or(())?;
                Ok(CapSpec::CSpace {
                    source: CSpaceSource::try_from(source)?,
                })
            }
            "irq" => {
                let line = get_arg(args, "line").ok_or(())?;
                let line = usize::from_str(line).map_err(|_| ())?;
                Ok(CapSpec::Irq { line })
            }
            "memory" => {
                let min_size = get_arg(args, "min_size_bytes").ok_or(())?;
                let min_size = usize::from_str(min_size).map_err(|_| ())?;
                Ok(CapSpec::Memory {
                    min_size_bytes: min_size,
                })
            }
            _ => Err(()),
        }
    }
}

impl TryFrom<&'_ str> for CSpaceSource {
    type Error = ();

    fn try_from(value: &'_ str) -> Result<Self, Self::Error> {
        match value {
            "self" => Ok(CSpaceSource::OwnCSpace),
            _ => Err(()),
        }
    }
}

fn get_arg<'a, 'b>(src: &'a str, arg: &'b str) -> Option<&'a str> {
    src.split(",")
        .filter_map(|part| part.split_once("="))
        .find_map(|(key, value)| if key == arg { Some(value) } else { None })
}
