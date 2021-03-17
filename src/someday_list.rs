use crate::markdown::{as_obsidian_link, parse_heading, parse_list, parse_tags, Fragment};
use pulldown_cmark::{CowStr, Options, Parser};

pub struct ProjectList<'a> {
    pub title: Fragment<'a>,
    pub tags: Vec<String>,
    pub projects: Vec<Item<'a>>,
}

impl<'a> ProjectList<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parse_heading(&mut parser, 1)?;
        let tags = parse_tags(&mut parser)?;

        let l = parse_list(&mut parser)?;
        let projects = l
            .into_iter()
            .map(|f| {
                as_obsidian_link(f.as_events())
                    .map(Item::Project)
                    .unwrap_or(Item::Simple(f))
            })
            .collect();

        Some(Self {
            title,
            tags,
            projects,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item<'a> {
    Project(CowStr<'a>),
    Simple(Fragment<'a>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::Event;

    #[test]
    fn title_parses() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let project_list = ProjectList::parse(text).unwrap();
        assert_eq!(
            project_list.title,
            Fragment(vec![Event::Text("Someday".into())])
        );
    }

    #[test]
    fn tags_parse() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let project_list = ProjectList::parse(text).unwrap();
        assert_eq!(project_list.tags, vec![String::from("gtd")]);
    }

    #[test]
    fn projects_parse() {
        let text = "# Someday\n#gtd\n\n- [[Some project]]\n- Not a project\n";
        let project_list = ProjectList::parse(text).unwrap();
        assert_eq!(
            project_list.projects,
            vec![
                Item::Project(CowStr::Borrowed("Some project")),
                Item::Simple(Fragment(vec![Event::Text("Not a project".into())])),
            ]
        );
    }
}
