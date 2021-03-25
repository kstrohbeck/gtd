use crate::markdown::{as_obsidian_link, Doc, Fragment, Heading};
use crate::parser::{ParseError, Parser};
use pulldown_cmark::{CowStr, Event};

#[derive(Debug, Clone, PartialEq)]
pub struct ActionList {
    pub title: Heading,
    pub tags: Vec<String>,
    pub contexts: Vec<Context>,
}

impl ActionList {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let Doc {
            title,
            tags,
            mut parser,
        } = Doc::parse(text)?;

        let mut contexts = vec![];
        while let Ok(context) = Context::parse(&mut parser) {
            contexts.push(context);
        }

        Ok(Self {
            title,
            tags,
            contexts,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub title: Heading,
    pub actions: Vec<Action>,
}

impl Context {
    pub fn parse<'a>(parser: &'a mut Parser) -> Result<Self, ParseError<'a>> {
        let title = parser.parse_heading(2)?;
        let actions = parser
            .parse_list()
            .ok()
            .unwrap_or_else(Vec::new)
            .into_iter()
            .map(Action::from_fragment)
            .collect();

        Ok(Self { title, actions })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    pub text: Fragment,
    pub project: Option<CowStr<'static>>,
}

impl Action {
    pub fn from_fragment(fragment: Fragment) -> Self {
        // Try to find the last soft break.
        let soft_break_idx = fragment
            .as_events()
            .iter()
            .rposition(|e| e == &Event::SoftBreak);

        let with_project = soft_break_idx.and_then(|idx| {
            let (text, link) = fragment.as_events().split_at(idx);
            as_obsidian_link(&link[1..]).map(|link| (text, link))
        });

        match with_project {
            Some((text, link)) => Action {
                text: Fragment::from_events(text.to_vec()),
                project: Some(link),
            },
            None => Action {
                text: fragment,
                project: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn title_parses() {
        let text = "# Next Actions\n#gtd\n\n## @foo\n\n- bar\n- baz\n  [[quux]]\n";
        let action_list = ActionList::parse(text).unwrap();
        assert_eq!(
            action_list.title,
            Fragment::from_events(vec![Event::Text("Next Actions".into())])
                .try_into()
                .unwrap()
        );
    }

    #[test]
    fn tags_parse() {
        let text = "# Next Actions\n#gtd\n\n## @foo\n\n- bar\n- baz\n  [[quux]]\n";
        let action_list = ActionList::parse(text).unwrap();
        assert_eq!(action_list.tags, vec![String::from("gtd")]);
    }

    #[test]
    fn context_parses() {
        let text = "# Next Actions\n#gtd\n\n## @foo\n\n- bar\n- baz\n  [[quux]]\n";
        let action_list = ActionList::parse(text).unwrap();
        assert_eq!(
            action_list.contexts,
            vec![Context {
                title: Fragment::from_events(vec![Event::Text("@foo".into())])
                    .try_into()
                    .unwrap(),
                actions: vec![
                    Action {
                        text: Fragment::from_events(vec![Event::Text("bar".into()),]),
                        project: None,
                    },
                    Action {
                        text: Fragment::from_events(vec![Event::Text("baz".into()),]),
                        project: Some(CowStr::Borrowed("quux")),
                    },
                ],
            }],
        );
    }

    #[test]
    fn multiple_contexts_parse() {
        let text =
            "# Next Actions\n#gtd\n\n## @foo\n\n- bar\n- baz\n  [[quux]]\n\n## @thing\n\n- stuff";
        let action_list = ActionList::parse(text).unwrap();
        assert_eq!(
            action_list.contexts,
            vec![
                Context {
                    title: Fragment::from_events(vec![Event::Text("@foo".into())])
                        .try_into()
                        .unwrap(),
                    actions: vec![
                        Action {
                            text: Fragment::from_events(vec![Event::Text("bar".into()),]),
                            project: None,
                        },
                        Action {
                            text: Fragment::from_events(vec![Event::Text("baz".into()),]),
                            project: Some(CowStr::Borrowed("quux")),
                        },
                    ],
                },
                Context {
                    title: Fragment::from_events(vec![Event::Text("@thing".into())])
                        .try_into()
                        .unwrap(),
                    actions: vec![Action {
                        text: Fragment::from_events(vec![Event::Text("stuff".into()),]),
                        project: None,
                    },],
                }
            ],
        );
    }

    #[test]
    fn empty_contexts_parse() {
        let text = "# Next Actions\n#gtd\n\n## @foo\n\n- bar\n- baz\n  [[quux]]\n\n## @empty\n\n## @thing\n\n- stuff\n";
        let action_list = ActionList::parse(text).unwrap();
        assert_eq!(
            action_list.contexts,
            vec![
                Context {
                    title: Fragment::from_events(vec![Event::Text("@foo".into())])
                        .try_into()
                        .unwrap(),
                    actions: vec![
                        Action {
                            text: Fragment::from_events(vec![Event::Text("bar".into()),]),
                            project: None,
                        },
                        Action {
                            text: Fragment::from_events(vec![Event::Text("baz".into()),]),
                            project: Some(CowStr::Borrowed("quux")),
                        },
                    ],
                },
                Context {
                    title: Fragment::from_events(vec![Event::Text("@empty".into())])
                        .try_into()
                        .unwrap(),
                    actions: vec![],
                },
                Context {
                    title: Fragment::from_events(vec![Event::Text("@thing".into())])
                        .try_into()
                        .unwrap(),
                    actions: vec![Action {
                        text: Fragment::from_events(vec![Event::Text("stuff".into()),]),
                        project: None,
                    },],
                }
            ],
        );
    }
}
