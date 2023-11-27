use ini_core::{Item, Parser};

pub struct Metadata<'src> {
    pub(super) parser: Parser<'src>,
}

impl<'src> Metadata<'src> {
    pub fn name(&self) -> Option<&'src str> {
        self.parser
            .clone()
            .take_while(|item| !matches!(item, Item::SectionEnd))
            .find_map(|item| match item {
                Item::Property("name", value) => value,
                _ => None,
            })
    }

    pub fn description(&self) -> Option<&'src str> {
        self.parser
            .clone()
            .take_while(|item| !matches!(item, Item::SectionEnd))
            .find_map(|item| match item {
                Item::Property("description", value) => value,
                _ => None,
            })
    }
}
