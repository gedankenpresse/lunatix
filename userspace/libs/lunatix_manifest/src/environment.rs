use ini_core::{Item, Parser};

pub struct Environment<'src> {
    pub(super) parser: Parser<'src>,
}

impl<'src> Environment<'src> {
    pub fn cspace_radix(&self) -> Option<usize> {
        self.parser
            .clone()
            .take_while(|item| !matches!(item, Item::SectionEnd))
            .find_map(|item| match item {
                Item::Property("cspace_radix", Some(value_str)) => {
                    usize::from_str_radix(value_str, 10).ok()
                }
                _ => None,
            })
    }

    pub fn stack_size_bytes(&self) -> Option<usize> {
        self.parser
            .clone()
            .take_while(|item| !matches!(item, Item::SectionEnd))
            .find_map(|item| match item {
                Item::Property("stack_size_bytes", Some(value_str)) => {
                    usize::from_str_radix(value_str, 10).ok()
                }
                _ => None,
            })
    }
}
