use crate::markdown::{as_obsidian_link, Heading, Parser};
use pulldown_cmark::{CowStr, Options};

#[derive(Debug, Clone)]
pub struct ProjectList {
    pub title: Heading,
    pub tags: Vec<String>,
    pub items: Vec<CowStr<'static>>,
}

impl ProjectList {
    pub fn parse(text: &str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parser.parse_heading(1)?;
        let tags = parser.parse_tags()?;
        let items = parser
            .parse_list()?
            .into_iter()
            .flat_map(|f| as_obsidian_link(f.as_events()))
            .collect();

        Some(Self { title, tags, items })
    }

    pub fn contains(&self, link: &str) -> bool {
        self.items.iter().any(|p| &**p == link)
    }
}
