use crate::markdown::{parse_event, parse_heading, parse_tasklist, parse_until, Fragment};
use pulldown_cmark::{Event, Options, Parser, Tag};
use std::iter::Peekable;

const GTD_PROJECT_TAG: &'static str = "gtd-project";

#[derive(Debug, Clone, PartialEq)]
pub struct Project<'a> {
    pub title: Fragment<'a>,
    pub tags: Vec<String>,
    pub goal: Option<Fragment<'a>>,
    pub info: Option<Fragment<'a>>,
    pub actions: Option<Vec<(bool, Fragment<'a>)>>,
}

impl<'a> Project<'a> {
    // TODO: This should return a Result with errors.
    pub fn parse(text: &'a str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parse_heading(&mut parser, 1)?;
        let tags = parse_tags(&mut parser)?;

        let mut parser = parser.peekable();

        let mut goal = None;
        let mut info = None;
        let mut actions = None;

        while parser.peek().is_some() {
            let mut heading = parse_heading(&mut parser, 2)?.0;

            let title = match heading.pop()? {
                Event::Text(t) => t,
                _ => return None,
            };

            if !heading.is_empty() {
                return None;
            }

            match &*title {
                "Goal" => goal = Some(parse_until(&mut parser, Event::Start(Tag::Heading(2)))),
                "Info" => info = Some(parse_until(&mut parser, Event::Start(Tag::Heading(2)))),
                "Actions" | "Action Items" => actions = parse_tasklist(&mut parser),
                _ => return None,
            }
        }

        Some(Self {
            title,
            tags,
            goal,
            info,
            actions,
        })
    }
}

// TODO: Should return borrowed, and also error if gtd-project isn't found.
fn parse_tags(mut parser: &mut Parser) -> Option<Vec<String>> {
    use crate::markdown::parse_tags;

    let mut tags = parse_tags(&mut parser)?;

    tags.iter().position(|s| s == GTD_PROJECT_TAG).map(|idx| {
        tags.remove(idx);
        tags
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_project_parses() {
        let project_str = "# Project title\n#gtd-project\n";
        let project = Project::parse(project_str);
        assert!(project.is_some());
    }

    #[test]
    fn strings_without_gtd_project_tag_dont_parse() {
        let project_str = "# Project title\n#other #tags\n";
        let project = Project::parse(project_str);
        assert_eq!(project, None);
    }

    #[test]
    fn simple_title_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n";
        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.title,
            Fragment(vec![Event::Text("Project title".into())])
        );
    }

    #[test]
    fn complex_title_is_parsed() {
        let project_str = "# Title with `code`\n#gtd-project\n";
        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.title,
            Fragment(vec![
                Event::Text("Title with ".into()),
                Event::Code("code".into()),
            ])
        );
    }

    #[test]
    fn gtd_project_tag_is_not_in_tags() {
        let project_str = "# Project title\n#gtd-project #other #tags\n";
        let project = Project::parse(project_str).unwrap();
        assert!(!project.tags.contains(&String::from("gtd-project")));
    }

    #[test]
    fn tags_are_parsed() {
        let project_str = "# Project title\n#gtd-project #other #tags\n";
        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.tags,
            vec![String::from("other"), String::from("tags")]
        );
    }

    #[test]
    fn goal_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n## Goal\nGoal text\n";
        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.goal,
            Some(Fragment(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Goal text".into()),
                Event::End(Tag::Paragraph)
            ])),
        );
    }

    #[test]
    fn goal_is_parsed_after_other_sections() {
        let project_str = "# Project title\n#gtd-project\n## Info\nFoo\n## Goal\nGoal text\n";
        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.goal,
            Some(Fragment(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Goal text".into()),
                Event::End(Tag::Paragraph)
            ])),
        );
    }

    #[test]
    fn info_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n## Info\nFoo\n";
        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.info,
            Some(Fragment(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Foo".into()),
                Event::End(Tag::Paragraph)
            ])),
        );
    }

    #[test]
    fn actions_are_parsed() {
        let project_str =
            "# Project title\n#gtd-project\n## Actions\n- [x] First action\n- [ ] Second action\n";

        let project = Project::parse(project_str).unwrap();
        assert_eq!(
            project.actions,
            Some(vec![
                (true, Fragment(vec![Event::Text("First action".into())])),
                (false, Fragment(vec![Event::Text("Second action".into())])),
            ]),
        );
    }

    #[test]
    fn things_are_parsed_even_in_reverse_order() {
        let project_str = "# Project title\n#gtd-project\n## Actions\n- [x] First action\n- [ ] Second action\n\n## Info\n\nFoo\n\n## Goal\n\nGoal text\n";

        let project = Project::parse(project_str).unwrap();

        assert_eq!(
            project.goal,
            Some(Fragment(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Goal text".into()),
                Event::End(Tag::Paragraph)
            ])),
        );

        assert_eq!(
            project.info,
            Some(Fragment(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Foo".into()),
                Event::End(Tag::Paragraph)
            ])),
        );

        assert_eq!(
            project.actions,
            Some(vec![
                (true, Fragment(vec![Event::Text("First action".into())])),
                (false, Fragment(vec![Event::Text("Second action".into())])),
            ]),
        );
    }
}
