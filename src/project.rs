use crate::{
    markdown::{BlockRef, Fragment, Heading},
    parser::{self, Doc, Parser},
};
use pulldown_cmark::{CowStr, Event, Tag};
use std::{convert::TryFrom, error::Error, fmt};

const SOMEDAY_TAG: &str = "someday";
const IN_PROGRESS_TAG: &str = "in-progress";
const COMPLETE_TAG: &str = "complete";

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: Name,
    // TODO: Rename title.
    pub title: Heading,
    pub tags: Vec<String>,
    pub status: Status,
    pub goal: Option<Fragment>,
    pub info: Option<Fragment>,
    pub actions: Actions,
}

impl Project {
    pub fn parse<S: Into<String>>(filename: S, text: &str) -> Result<Self, ParseError> {
        let name = Name::new(filename.into()).ok_or(ParseError::InvalidProjectName)?;

        let Doc {
            title,
            mut tags,
            mut parser,
        } = Doc::parse(text).map_err(ParseError::ParseError)?;

        let (status_idx, status) = tags
            .iter()
            .enumerate()
            .find_map(|(i, t)| Status::try_from(t.as_str()).ok().map(|s| (i, s)))
            .ok_or(ParseError::MissingStatus)?;

        tags.remove(status_idx);

        let mut goal = None;
        let mut info = None;
        let mut actions = None;

        while parser.peek().is_some() {
            let section_heading = parser.parse_heading(2).map_err(ParseError::ParseError)?;
            let section_title = section_heading
                .try_to_text()
                .ok_or_else(|| ParseError::HasSectionWithNonStringTitle(section_heading.clone()))?;

            match &*section_title {
                "Goal" => goal = Some(parser.parse_until(Event::Start(Tag::Heading(2)))),
                "Info" => info = Some(parser.parse_until(Event::Start(Tag::Heading(2)))),
                "Actions" => actions = Actions::parse(&mut parser).ok(),
                "Action Items" => {
                    let title_string = title.try_to_title_string().unwrap();
                    println!("Warning: Project \"{}\" uses deprecated \"Action Items\" section; rename to \"Actions\".", title_string);
                    actions = Actions::parse(&mut parser).ok();
                }
                _ => {
                    return Err(ParseError::HasUnexpectedSection(section_heading));
                }
            }
        }

        Ok(Self {
            name,
            title,
            tags,
            status,
            goal,
            info,
            actions: actions.unwrap_or_else(Actions::default),
        })
    }

    pub fn id(&self) -> &str {
        self.name.id()
    }

    pub fn title(&self) -> &str {
        self.name.title()
    }
}

impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name {
    name: String,
    split_idx: usize,
}

impl Name {
    pub fn new(name: String) -> Option<Self> {
        let split_idx = name
            .char_indices()
            .find_map(|(i, c)| if c == ' ' { Some(i) } else { None })?;

        // Validate the ID.
        let id = &name[..split_idx];
        if id.len() != 12 || id.chars().any(|c| !c.is_digit(10)) {
            return None;
        }

        Some(Self { name, split_idx })
    }

    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &str {
        &self.name[..self.split_idx]
    }

    pub fn title(&self) -> &str {
        &self.name[self.split_idx + 1..]
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Someday,
    InProgress,
    Complete,
}

impl TryFrom<&str> for Status {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            SOMEDAY_TAG => Ok(Self::Someday),
            IN_PROGRESS_TAG => Ok(Self::InProgress),
            COMPLETE_TAG => Ok(Self::Complete),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Actions {
    active: Vec<Action>,
    upcoming: Vec<Action>,
    complete: Vec<Action>,
}

impl Actions {
    fn parse<'a>(parser: &mut Parser<'a>) -> Result<Self, ParseError<'a>> {
        let mut active = Vec::new();
        let mut upcoming = Vec::new();
        let mut complete = Vec::new();

        while let Some(Event::Start(Tag::Heading(3))) = parser.peek() {
            let section_heading = parser.parse_heading(3)?;
            let section_title = section_heading
                .try_to_text()
                .ok_or_else(|| ParseError::HasSectionWithNonStringTitle(section_heading.clone()))?;

            let actions_type = match &*section_title {
                "Active" => ActionStatus::Active,
                "Upcoming" => ActionStatus::Upcoming,
                "Complete" => ActionStatus::Complete,
                _ => {
                    return Err(ParseError::HasUnexpectedSection(section_heading));
                }
            };

            let actions = parser
                .parse_list_opt()?
                .into_iter()
                .map(Action::from_fragment)
                .collect();

            match actions_type {
                ActionStatus::Active => active = actions,
                ActionStatus::Upcoming => upcoming = actions,
                ActionStatus::Complete => complete = actions,
            }
        }

        Ok(Self {
            active,
            upcoming,
            complete,
        })
    }

    pub fn actions(&self) -> impl Iterator<Item = (&Action, ActionStatus)> {
        let active = self.active.iter().map(|a| (a, ActionStatus::Active));
        let upcoming = self.upcoming.iter().map(|a| (a, ActionStatus::Upcoming));
        let complete = self.complete.iter().map(|a| (a, ActionStatus::Complete));
        active.chain(upcoming).chain(complete)
    }

    pub fn get_action(&self, id: &ActionId) -> Option<(&Action, ActionStatus)> {
        self.actions()
            .find(|(a, _)| matches!(&a.id, Some(x) if x == id))
    }
}

impl Default for Actions {
    fn default() -> Self {
        Self {
            active: Vec::new(),
            upcoming: Vec::new(),
            complete: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Active,
    Upcoming,
    Complete,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    text: Fragment,
    id: Option<ActionId>,
}

impl Action {
    fn from_fragment(frag: Fragment) -> Self {
        // For the action to have a reference, we need the last event of the fragment to be a Text
        // with it as a suffix.

        fn split_id<'a>(ev: &Event<'a>) -> Option<(Option<Event<'a>>, String)> {
            let text = match ev {
                Event::Text(t) => t,
                _ => return None,
            };

            let idx = text.rfind('^')?;
            let id = &text[idx + 1..];
            if id.len() != 6 {
                return None;
            }

            let rest = match text[..idx].trim_end() {
                "" => None,
                s => Some(Event::Text(CowStr::Boxed(s.to_string().into_boxed_str()))),
            };

            let id = id.to_string();

            Some((rest, id))
        }

        let mut evs = frag.into_events();

        let last_ev = match evs.pop() {
            Some(ev) => ev,
            None => {
                return Action {
                    text: Fragment::from_events(evs),
                    id: None,
                }
            }
        };

        let id = match split_id(&last_ev) {
            Some((ev, id)) => {
                if let Some(ev) = ev {
                    evs.push(ev);
                }
                Some(ActionId(id))
            }
            None => {
                evs.push(last_ev);
                None
            }
        };

        Action {
            text: Fragment::from_events(evs),
            id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRef {
    pub project_name: Name,
    pub action_id: ActionId,
}

impl ActionRef {
    pub fn from_block_ref(block_ref: BlockRef) -> Option<Self> {
        let project_name = Name::new(block_ref.link)?;
        let action_id = ActionId(block_ref.id);
        Some(Self {
            project_name,
            action_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError<'a> {
    InvalidProjectName,
    MissingStatus,
    HasSectionWithNonStringTitle(Heading),
    HasUnexpectedSection(Heading),
    ParseError(parser::ParseError<'a>),
}

impl<'a> ParseError<'a> {
    pub fn into_static(self) -> ParseError<'static> {
        match self {
            Self::InvalidProjectName => ParseError::InvalidProjectName,
            Self::MissingStatus => ParseError::MissingStatus,
            Self::HasSectionWithNonStringTitle(h) => ParseError::HasSectionWithNonStringTitle(h),
            Self::HasUnexpectedSection(h) => ParseError::HasUnexpectedSection(h),
            Self::ParseError(p) => ParseError::ParseError(p.into_static()),
        }
    }
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidProjectName => write!(f, "Project has invalid name"),
            Self::MissingStatus => write!(f, "Project is missing status"),
            Self::HasSectionWithNonStringTitle(_) => {
                write!(f, "Project has section with non-string title")
            }
            Self::HasUnexpectedSection(_) => write!(f, "Project has unexpected section"),
            Self::ParseError(p) => write!(f, "{}", p),
        }
    }
}

impl<'a> Error for ParseError<'a> {}

impl<'a> From<parser::ParseError<'a>> for ParseError<'a> {
    fn from(error: parser::ParseError<'a>) -> Self {
        Self::ParseError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    mod action {
        use super::*;

        #[test]
        fn text_action_with_id_has_correct_id() {
            let frag = Fragment::from_events(vec![Event::Text("action text ^abcdef".into())]);
            let action = Action::from_fragment(frag);
            assert_eq!(action.id, Some(ActionId(String::from("abcdef"))));
        }

        #[test]
        fn text_action_with_id_has_correct_text() {
            let frag = Fragment::from_events(vec![Event::Text("action text ^abcdef".into())]);
            let action = Action::from_fragment(frag);
            assert_eq!(
                action.text,
                Fragment::from_events(vec![Event::Text("action text".into())])
            );
        }

        #[test]
        fn text_action_without_id_has_no_id() {
            let frag = Fragment::from_events(vec![Event::Text("action text".into())]);
            let action = Action::from_fragment(frag);
            assert_eq!(action.id, None);
        }

        #[test]
        fn text_action_without_id_has_correct_text() {
            let frag = Fragment::from_events(vec![Event::Text("action text".into())]);
            let action = Action::from_fragment(frag);
            assert_eq!(
                action.text,
                Fragment::from_events(vec![Event::Text("action text".into())])
            );
        }

        #[test]
        fn complex_action_with_id_has_correct_id() {
            let frag = Fragment::from_events(vec![
                Event::Text("Something with ".into()),
                Event::Start(Tag::Emphasis),
                Event::Text("emphasis".into()),
                Event::End(Tag::Emphasis),
                Event::Text(" ^abcdef".into()),
            ]);
            let action = Action::from_fragment(frag);
            assert_eq!(action.id, Some(ActionId(String::from("abcdef"))));
        }

        #[test]
        fn complex_action_with_id_has_correct_text() {
            let frag = Fragment::from_events(vec![
                Event::Text("Something with ".into()),
                Event::Start(Tag::Emphasis),
                Event::Text("emphasis".into()),
                Event::End(Tag::Emphasis),
                Event::Text(" ^abcdef".into()),
            ]);
            let action = Action::from_fragment(frag);
            assert_eq!(
                action.text,
                Fragment::from_events(vec![
                    Event::Text("Something with ".into()),
                    Event::Start(Tag::Emphasis),
                    Event::Text("emphasis".into()),
                    Event::End(Tag::Emphasis),
                ])
            );
        }

        #[test]
        fn complex_action_without_id_has_no_id() {
            let frag = Fragment::from_events(vec![
                Event::Text("Something with ".into()),
                Event::Start(Tag::Emphasis),
                Event::Text("emphasis".into()),
                Event::End(Tag::Emphasis),
                Event::Text(" ".into()),
            ]);
            let action = Action::from_fragment(frag);
            assert_eq!(action.id, None);
        }

        #[test]
        fn complex_action_without_id_has_correct_text() {
            let frag = Fragment::from_events(vec![
                Event::Text("Something with ".into()),
                Event::Start(Tag::Emphasis),
                Event::Text("emphasis".into()),
                Event::End(Tag::Emphasis),
                Event::Text(" ".into()),
            ]);
            let action = Action::from_fragment(frag);
            assert_eq!(
                action.text,
                Fragment::from_events(vec![
                    Event::Text("Something with ".into()),
                    Event::Start(Tag::Emphasis),
                    Event::Text("emphasis".into()),
                    Event::End(Tag::Emphasis),
                    Event::Text(" ".into()),
                ])
            );
        }
    }

    #[test]
    fn basic_project_parses() {
        let project_str = "# Project title\n#in-progress\n";
        let project = Project::parse("197001010000 Project title", project_str);
        assert!(project.is_ok());
    }

    #[test]
    fn simple_title_is_parsed() {
        let project_str = "# Project title\n#in-progress\n";
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
        let project_str = "# Title with `code`\n#in-progress\n";
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
        let project_str = "# Project title\n#in-progress #other #tags\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(
            project.tags,
            vec![String::from("other"), String::from("tags")]
        );
    }

    #[test]
    fn someday_status_is_parsed() {
        let project_str = "# Project title\n#someday\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(project.status, Status::Someday);
    }

    #[test]
    fn in_progress_status_is_parsed() {
        let project_str = "# Project title\n#in-progress\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(project.status, Status::InProgress);
    }

    #[test]
    fn complete_status_is_parsed() {
        let project_str = "# Project title\n#complete\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(project.status, Status::Complete);
    }

    #[test]
    fn status_is_not_in_tags() {
        let project_str = "# Project title\n#in-progress #other #tags\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert!(!project.tags.contains(&String::from("in-progress")));
    }

    #[test]
    fn parsing_fails_without_status() {
        let project_str = "# Project title\n#other #tags\n";
        let project = Project::parse("197001010000 Project title", project_str);
        assert_eq!(project, Err(ParseError::MissingStatus));
    }

    #[test]
    fn goal_is_parsed() {
        let project_str = "# Project title\n#in-progress\n## Goal\nGoal text\n";
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
        let project_str = "# Project title\n#in-progress\n## Info\nFoo\n## Goal\nGoal text\n";
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
        let project_str = "# Project title\n#in-progress\n## Info\nFoo\n";
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
        let project_str =
            "# Project title\n#in-progress\n## Actions\n\n### Active\n\n- First action\n\n### Upcoming\n\n- Second action ^abcdef\n- Third action `with code` ^fedcba\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(
            project.actions,
            Actions {
                active: vec![Action {
                    text: Fragment::from_events(vec![Event::Text("First action".into())]),
                    id: None
                }],
                upcoming: vec![
                    Action {
                        text: Fragment::from_events(vec![Event::Text("Second action".into())]),
                        id: Some(ActionId(String::from("abcdef"))),
                    },
                    Action {
                        text: Fragment::from_events(vec![
                            Event::Text("Third action ".into()),
                            Event::Code("with code".into())
                        ]),
                        id: Some(ActionId(String::from("fedcba"))),
                    }
                ],
                complete: vec![],
            }
        );
    }

    #[test]
    fn things_are_parsed_even_in_reverse_order() {
        let project_str =
            "# Project title\n#in-progress\n## Actions\n\n### Active\n\n- First action\n\n### Upcoming\n\n- Second action ^abcdef\n- Third action `with code` ^fedcba\n\n## Info\n\nFoo\n\n## Goal\n\nGoal text\n";
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
            Actions {
                active: vec![Action {
                    text: Fragment::from_events(vec![Event::Text("First action".into())]),
                    id: None
                }],
                upcoming: vec![
                    Action {
                        text: Fragment::from_events(vec![Event::Text("Second action".into())]),
                        id: Some(ActionId(String::from("abcdef"))),
                    },
                    Action {
                        text: Fragment::from_events(vec![
                            Event::Text("Third action ".into()),
                            Event::Code("with code".into())
                        ]),
                        id: Some(ActionId(String::from("fedcba"))),
                    }
                ],
                complete: vec![],
            }
        );
    }

    #[test]
    fn empty_action_section_is_allowed() {
        let project_str = "# Project title\n#in-progress\n## Actions\n\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(project.actions, Actions::default());
    }

    #[test]
    fn empty_action_subsection_is_allowed() {
        let project_str = "# Project title\n#in-progress\n## Actions\n\n### Active\n\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(project.actions, Actions::default());
    }

    #[test]
    fn empty_action_subsection_is_allowed_followed_by_nonempty_section() {
        let project_str =
            "# Project title\n#in-progress\n## Actions\n\n### Active\n\n### Upcoming\n\n- foo\n\n";
        let project = Project::parse("197001010000 Project title", project_str).unwrap();
        assert_eq!(
            project.actions,
            Actions {
                active: vec![],
                upcoming: vec![Action {
                    text: Fragment::from_events(vec![Event::Text("foo".into())]),
                    id: None,
                }],
                complete: vec![],
            }
        );
    }

    mod id {
        use super::*;

        #[test]
        fn id_is_parsed() {
            let project_str = "# Project title\n#in-progress\n";
            let project = Project::parse("197001010000 Project title", project_str).unwrap();

            assert_eq!(project.id(), "197001010000");
        }
    }

    mod name {
        use super::*;

        #[test]
        fn name_is_returned_if_it_exists() {
            let project_str = "# Project title\n#in-progress\n";
            let project = Project::parse("197001010000 Project title", project_str).unwrap();

            assert_eq!(project.title(), "Project title");
        }
    }
}
