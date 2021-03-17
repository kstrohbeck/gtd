use crate::markdown::{as_obsidian_link, parse_heading, parse_list, parse_tags, Fragment};
use pulldown_cmark::{CowStr, Options, Parser};

pub struct ProjectList<'a> {
    pub title: Fragment<'a>,
    pub tags: Vec<String>,
    pub items: Vec<CowStr<'a>>,
}

impl<'a> ProjectList<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parse_heading(&mut parser, 1)?;
        let tags = parse_tags(&mut parser)?;

        let l = parse_list(&mut parser)?;
        let items = l
            .into_iter()
            .map(|f| as_obsidian_link(f.as_events()))
            .flatten()
            .collect();

        Some(Self { title, tags, items })
    }
}
