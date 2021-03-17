use crate::markdown::{as_obsidian_link, parse_heading, parse_list, parse_tags, Fragment};
use pulldown_cmark::{CowStr, Event, Options, Parser};

#[derive(Debug, Clone, PartialEq)]
pub struct ActionList {
    pub title: Fragment,
    pub tags: Vec<String>,
    pub contexts: Vec<Context>,
}

impl ActionList {
    pub fn parse(text: &str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parse_heading(&mut parser, 1)?;
        let tags = parse_tags(&mut parser)?;

        let mut contexts = vec![];
        while let Some(context) = Context::parse(&mut parser) {
            contexts.push(context);
        }

        Some(Self {
            title,
            tags,
            contexts,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub title: Fragment,
    pub actions: Vec<Action>,
}

impl Context {
    pub fn parse<'a, I>(mut parser: I) -> Option<Self>
    where
        I: Iterator<Item = Event<'a>>,
    {
        let title = parse_heading(&mut parser, 2)?;
        let list = parse_list(&mut parser).unwrap_or(vec![]);
        let actions = list.into_iter().map(Action::from_fragment).collect();

        Some(Self { title, actions })
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
        if let Some(idx) = soft_break_idx {
            let (text, link) = fragment.as_events().split_at(idx);
            if let Some(link) = as_obsidian_link(&link[1..]) {
                Action {
                    text: Fragment::from_events(text.to_vec()),
                    project: Some(link),
                }
            } else {
                Action {
                    text: fragment,
                    project: None,
                }
            }
        } else {
            Action {
                text: fragment,
                project: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_parses() {
        let text = "# Next Actions\n#gtd\n\n## @foo\n\n- bar\n- baz\n  [[quux]]\n";
        let action_list = ActionList::parse(text).unwrap();
        assert_eq!(
            action_list.title,
            Fragment::from_events(vec![Event::Text("Next Actions".into())])
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
                title: Fragment::from_events(vec![Event::Text("@foo".into())]),
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
                    title: Fragment::from_events(vec![Event::Text("@foo".into())]),
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
                    title: Fragment::from_events(vec![Event::Text("@thing".into())]),
                    actions: vec![Action {
                        text: Fragment::from_events(vec![Event::Text("stuff".into()),]),
                        project: None,
                    },],
                }
            ],
        );
    }
}
