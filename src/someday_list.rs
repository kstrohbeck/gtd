use crate::markdown::{as_obsidian_link, parse_heading, parse_list, parse_tags, Fragment};
use pulldown_cmark::{CowStr, Options, Parser};

#[derive(Debug, Clone)]
pub struct SomedayList {
    pub title: Fragment,
    pub tags: Vec<String>,
    pub items: Vec<Item>,
}

impl SomedayList {
    pub fn parse(text: &str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parse_heading(&mut parser, 1)?;
        let tags = parse_tags(&mut parser)?;

        let l = parse_list(&mut parser)?;
        let items = l
            .into_iter()
            .map(|f| {
                as_obsidian_link(f.as_events())
                    .map(Item::Project)
                    .unwrap_or(Item::Simple(f))
            })
            .collect();

        Some(Self { title, tags, items })
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

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::Event;

    #[test]
    fn title_parses() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let someday_list = SomedayList::parse(text).unwrap();
        assert_eq!(
            someday_list.title,
            Fragment::from_events(vec![Event::Text("Someday".into())])
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
