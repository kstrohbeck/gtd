use crate::markdown::{Fragment, Heading, Parser};
use pulldown_cmark::{Event, Options, Tag};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub filename: String,
    filename_split_idx: Option<usize>,
    pub title: Heading,
    pub tags: Vec<String>,
    pub goal: Option<Fragment>,
    pub info: Option<Fragment>,
    pub actions: Vec<(bool, Fragment)>,
}

impl Project {
    const GTD_PROJECT_TAG: &'static str = "gtd-project";
    const COMPLETE_TAG: &'static str = "complete";

    // TODO: This should return a Result with errors.
    pub fn parse(filename: String, text: &str) -> Option<Self> {
        let filename_split_idx =
            filename
                .char_indices()
                .find_map(|(i, c)| if c == ' ' { Some(i) } else { None });

        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parser.parse_heading(1)?;

        let mut tags = parser.parse_tags()?;
        let idx = tags.iter().position(|s| s == Self::GTD_PROJECT_TAG)?;
        tags.remove(idx);

        let mut goal = None;
        let mut info = None;
        let mut actions = None;

        while parser.peek().is_some() {
            let heading = parser.parse_heading(2)?;
            let title = heading.try_as_str()?;

            match &*title {
                "Goal" => goal = Some(parser.parse_until(Event::Start(Tag::Heading(2)))),
                "Info" => info = Some(parser.parse_until(Event::Start(Tag::Heading(2)))),
                "Actions" | "Action Items" => actions = parser.parse_tasklist(),
                _ => return None,
            }
        }

        Some(Self {
            filename,
            filename_split_idx,
            title,
            tags,
            goal,
            info,
            actions: actions.unwrap_or_else(Vec::new),
        })
    }

    pub fn id(&self) -> Option<&str> {
        let idx = self.filename_split_idx?;
        let id = self.filename.get(..idx)?;
        if id.len() != 12 || id.chars().any(|c| !c.is_digit(10)) {
            return None;
        }
        Some(id)
    }

    pub fn name(&self) -> Option<&str> {
        self.filename.get(self.filename_split_idx? + 1..)
    }

    pub fn is_complete(&self) -> bool {
        self.tags.iter().any(|s| s == Self::COMPLETE_TAG)
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn basic_project_parses() {
        let project_str = "# Project title\n#gtd-project\n";
        let project = Project::parse("197001010000 Project title".into(), project_str);
        assert!(project.is_some());
    }

    #[test]
    fn strings_without_gtd_project_tag_dont_parse() {
        let project_str = "# Project title\n#other #tags\n";
        let project = Project::parse("197001010000 Project title".into(), project_str);
        assert_eq!(project, None);
    }

    #[test]
    fn simple_title_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert_eq!(
            project.title,
            Fragment::from_events(vec![Event::Text("Project title".into())])
                .try_into()
                .unwrap()
        );
    }

    #[test]
    fn complex_title_is_parsed() {
        let project_str = "# Title with `code`\n#gtd-project\n";
        let project = Project::parse("197001010000 Title with code".into(), project_str).unwrap();
        assert_eq!(
            project.title,
            Fragment::from_events(vec![
                Event::Text("Title with ".into()),
                Event::Code("code".into()),
            ])
            .try_into()
            .unwrap()
        );
    }

    #[test]
    fn gtd_project_tag_is_not_in_tags() {
        let project_str = "# Project title\n#gtd-project #other #tags\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert!(!project.tags.contains(&String::from("gtd-project")));
    }

    #[test]
    fn tags_are_parsed() {
        let project_str = "# Project title\n#gtd-project #other #tags\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert_eq!(
            project.tags,
            vec![String::from("other"), String::from("tags")]
        );
    }

    #[test]
    fn goal_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n## Goal\nGoal text\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert_eq!(
            project.goal,
            Some(Fragment::from_events(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Goal text".into()),
                Event::End(Tag::Paragraph)
            ])),
        );
    }

    #[test]
    fn goal_is_parsed_after_other_sections() {
        let project_str = "# Project title\n#gtd-project\n## Info\nFoo\n## Goal\nGoal text\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert_eq!(
            project.goal,
            Some(Fragment::from_events(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Goal text".into()),
                Event::End(Tag::Paragraph)
            ])),
        );
    }

    #[test]
    fn info_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n## Info\nFoo\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert_eq!(
            project.info,
            Some(Fragment::from_events(vec![
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
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();
        assert_eq!(
            project.actions,
            vec![
                (
                    true,
                    Fragment::from_events(vec![Event::Text("First action".into())])
                ),
                (
                    false,
                    Fragment::from_events(vec![Event::Text("Second action".into())])
                ),
            ],
        );
    }

    #[test]
    fn things_are_parsed_even_in_reverse_order() {
        let project_str = "# Project title\n#gtd-project\n## Actions\n- [x] First action\n- [ ] Second action\n\n## Info\n\nFoo\n\n## Goal\n\nGoal text\n";
        let project = Project::parse("197001010000 Project title".into(), project_str).unwrap();

        assert_eq!(
            project.goal,
            Some(Fragment::from_events(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Goal text".into()),
                Event::End(Tag::Paragraph)
            ])),
        );

        assert_eq!(
            project.info,
            Some(Fragment::from_events(vec![
                Event::Start(Tag::Paragraph),
                Event::Text("Foo".into()),
                Event::End(Tag::Paragraph)
            ])),
        );

        assert_eq!(
            project.actions,
            vec![
                (
                    true,
                    Fragment::from_events(vec![Event::Text("First action".into())])
                ),
                (
                    false,
                    Fragment::from_events(vec![Event::Text("Second action".into())])
                ),
            ],
        );
    }
}
