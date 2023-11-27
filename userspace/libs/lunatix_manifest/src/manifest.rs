use crate::capabilities::{CapPlacement, Capabilities};
use crate::environment::Environment;
use crate::metadata::Metadata;
use ini_core::{Item, Parser};

pub struct LunatixManifest<'src> {
    pub src: &'src str,
}

impl<'src> LunatixManifest<'src> {
    pub fn from(src: &'src str) -> Self {
        Self { src }
    }

    fn parser(&self) -> Parser<'src> {
        Parser::new(self.src).auto_trim(true)
    }

    pub fn metadata(&self) -> Option<Metadata> {
        let mut parser = self.parser();
        let _ = parser.find(|item| matches!(item, Item::Section("metadata")))?;
        Some(Metadata { parser })
    }

    pub fn environment(&self) -> Option<Environment> {
        let mut parser = self.parser();
        let _ = parser.find(|item| matches!(item, Item::Section("environment")))?;
        Some(Environment { parser })
    }

    pub fn capabilities(&self) -> Option<Capabilities> {
        let mut parser = self.parser();
        let _ = parser.find(|item| matches!(item, Item::Section("capabilities")))?;
        Some(Capabilities {
            parser,
            is_done: false,
        })
    }
}
