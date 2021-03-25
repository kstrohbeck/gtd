use crate::markdown::{as_obsidian_link, Doc, Heading};
use crate::parser::ParseError;
use pulldown_cmark::CowStr;

#[derive(Debug, Clone)]
pub struct ProjectList {
    pub title: Heading,
    pub tags: Vec<String>,
    pub items: Vec<CowStr<'static>>,
}

impl ProjectList {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let Doc {
            title,
            tags,
            mut parser,
        } = Doc::parse(text)?;

        let items = parser
            .parse_list()?
            .into_iter()
            .flat_map(|f| as_obsidian_link(f.as_events()))
            .collect();

        Ok(Self { title, tags, items })
    }

    pub fn contains(&self, link: &str) -> bool {
        self.items.iter().any(|p| &**p == link)
    }
}
