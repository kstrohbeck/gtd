use crate::markdown::{Doc, Fragment, Heading};
use crate::parser;
use pulldown_cmark::{Event, Tag};
use std::{error::Error, fmt};

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
    const COMPLETE_TAG: &'static str = "complete";

    pub fn parse<S: Into<String>>(filename: S, text: &str) -> Result<Self, ParseError> {
        let filename = filename.into();

        let filename_split_idx =
            filename
                .char_indices()
                .find_map(|(i, c)| if c == ' ' { Some(i) } else { None });

        let Doc {
            title,
            tags,
            mut parser,
        } = Doc::parse(text).map_err(ParseError::ParseError)?;

        let mut goal = None;
        let mut info = None;
        let mut actions = None;

        while parser.peek().is_some() {
            let section_heading = parser.parse_heading(2).map_err(ParseError::ParseError)?;
            let section_title = section_heading
                .try_as_str()
                .ok_or_else(|| ParseError::HasSectionWithNonStringTitle(section_heading.clone()))?;

            match &*section_title {
                "Goal" => goal = Some(parser.parse_until(Event::Start(Tag::Heading(2)))),
                "Info" => info = Some(parser.parse_until(Event::Start(Tag::Heading(2)))),
                "Actions" => actions = parser.parse_tasklist().ok(),
                "Action Items" => {
                    let title_string = title.try_as_title_string().unwrap();
                    println!("Warning: Project \"{}\" uses deprecated \"Action Items\" section; rename to \"Actions\".", title_string);
                    actions = parser.parse_tasklist().ok();
                }
                _ => {
                    return Err(ParseError::HasUnexpectedSection(section_heading));
                }
            }
        }

        Ok(Self {
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

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError<'a> {
    HasSectionWithNonStringTitle(Heading),
    HasUnexpectedSection(Heading),
    ParseError(parser::ParseError<'a>),
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::HasSectionWithNonStringTitle(_) => write!(f, "has section with non-string title"),
            Self::HasUnexpectedSection(_) => write!(f, "has unexpected section"),
            Self::ParseError(p) => write!(f, "{}", p),
        }
    }
}

impl<'a> Error for ParseError<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn basic_project_parses() {
        let project_str = "# Project title\n";
        let project = Project::parse("197001010000 Project title", project_str);
        assert!(project.is_ok());
    }

    #[test]
    fn simple_title_is_parsed() {
        let project_str = "# Project title\n#gtd-project\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
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
        let project = Project::parse("197001010000 Title with code", project_str).unwrap();
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
    fn tags_are_parsed() {
        let project_str = "# Project title\n#other #tags\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(
            project.tags,
            vec![String::from("other"), String::from("tags")]
        );
    }

    #[test]
    fn goal_is_parsed() {
        let project_str = "# Project title\n## Goal\nGoal text\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
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
        let project_str = "# Project title\n## Info\nFoo\n## Goal\nGoal text\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
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
        let project_str = "# Project title\n## Info\nFoo\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
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
        let project_str = "# Project title\n## Actions\n- [x] First action\n- [ ] Second action\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
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
        let project_str = "# Project title\n## Actions\n- [x] First action\n- [ ] Second action\n\n## Info\n\nFoo\n\n## Goal\n\nGoal text\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();

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

    mod id {
        use super::*;

        #[test]
        fn id_is_returned_if_it_exists() {
            let project_str = "# Project title\n";
            let project = Project::parse("197001010000 Project title", project_str).unwrap();

            assert_eq!(project.id(), Some("197001010000"));
        }

        #[test]
        fn id_is_not_returned_if_in_wrong_format() {
            let project_str = "# Project title\n";
            let project = Project::parse("19700101 Project title", project_str).unwrap();

            assert!(project.id().is_none());
        }

        #[test]
        fn id_is_not_returned_if_it_doesnt_exist() {
            let project_str = "# Project title\n";
            let project = Project::parse("Project title", project_str).unwrap();

            assert!(project.id().is_none());
        }
    }

    mod name {
        use super::*;

        #[test]
        fn name_is_returned_if_it_exists() {
            let project_str = "# Project title\n";
            let project = Project::parse("197001010000 Project title", project_str).unwrap();

            assert_eq!(project.name(), Some("Project title"));
        }

        #[test]
        fn name_is_not_returned_if_it_doesnt_exist() {
            let project_str = "# Project title\n";
            let project = Project::parse("197001010000", project_str).unwrap();

            assert!(project.name().is_none());
        }
    }

    mod is_complete {
        use super::*;

        #[test]
        fn project_is_complete_if_it_has_tag() {
            let project_str = "# Project title\n#complete\n";
            let project = Project::parse("197001010000 Project title", project_str).unwrap();

            assert!(project.is_complete());
        }

        #[test]
        fn project_is_not_complete_if_it_doesnt_have_tag() {
            let project_str = "# Project title\n";
            let project = Project::parse("197001010000 Project title", project_str).unwrap();

            assert!(!project.is_complete());
        }
    }
}
