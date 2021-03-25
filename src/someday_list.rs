use crate::markdown::{as_obsidian_link, Doc, Fragment, Heading};
use crate::parser::ParseError;
use pulldown_cmark::CowStr;

#[derive(Debug, Clone)]
pub struct SomedayList {
    pub title: Heading,
    pub tags: Vec<String>,
    pub items: Vec<Item>,
}

impl SomedayList {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let Doc {
            title,
            tags,
            mut parser,
        } = Doc::parse(text)?;

        let items = parser
            .parse_list()?
            .into_iter()
            .map(|f| {
                as_obsidian_link(f.as_events())
                    .map(Item::Project)
                    .unwrap_or(Item::Simple(f))
            })
            .collect();

        Ok(Self { title, tags, items })
    }

    pub fn contains(&self, link: &str) -> bool {
        self.items
            .iter()
            .filter_map(|p| match p {
                Item::Project(p) => Some(p),
                _ => None,
            })
            .any(|p| &**p == link)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Project(CowStr<'static>),
    Simple(Fragment),
}

impl Item {
    pub fn link(&self) -> Option<&str> {
        match self {
            Item::Project(p) => Some(p),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::Event;
    use std::convert::TryInto;

    #[test]
    fn title_parses() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let someday_list = SomedayList::parse(text).unwrap();
        assert_eq!(
            someday_list.title,
            Fragment::from_events(vec![Event::Text("Someday".into())])
                .try_into()
                .unwrap()
        );
    }

    #[test]
    fn tags_parse() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let someday_list = SomedayList::parse(text).unwrap();
        assert_eq!(someday_list.tags, vec![String::from("gtd")]);
    }

    #[test]
    fn projects_parse() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let someday_list = SomedayList::parse(text).unwrap();
        assert_eq!(
            someday_list.items,
            vec![
                Item::Project(CowStr::Borrowed("Some project")),
                Item::Simple(Fragment::from_events(vec![Event::Text(
                    "Not a project".into()
                )])),
            ]
        );
    }
}
