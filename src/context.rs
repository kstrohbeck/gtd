use crate::markdown::{as_obsidian_link, Doc, Fragment, Heading};
use crate::parser::ParseError;
use pulldown_cmark::{CowStr, Event};

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub filename: String,
    pub title: Heading,
    pub actions: Vec<Action>,
}

impl Context {
    pub fn parse<S: Into<String>>(filename: S, text: &str) -> Result<Self, ParseError> {
        let filename = filename.into();

        let Doc {
            title,
            tags: _tags,
            mut parser,
        } = Doc::parse(text)?;

        let actions = parser
            .parse_list()
            .ok()
            .unwrap_or_else(Vec::new)
            .into_iter()
            .map(Action::from_fragment)
            .collect();

        Ok(Self {
            filename,
            title,
            actions,
        })
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
        let text = "# @computer\n\n- foo\n- bar\n  [[baz]]";
        let context = Context::parse("@computer", text).unwrap();
        assert_eq!(
            context.title,
            Fragment::from_events(vec![Event::Text("@computer".into())])
                .try_into()
                .unwrap()
        );
    }

    #[test]
    fn actions_parse() {
        let text = "# @computer\n\n- foo\n- bar\n  [[baz]]";
        let context = Context::parse("@computer", text).unwrap();
        assert_eq!(
            context.actions,
            vec![
                Action {
                    text: Fragment::from_events(vec![Event::Text("foo".into())]),
                    project: None,
                },
                Action {
                    text: Fragment::from_events(vec![Event::Text("bar".into())]),
                    project: Some(CowStr::Borrowed("baz")),
                }
            ]
        );
    }

    #[test]
    fn context_without_actions_parses() {
        let text = "# @computer\n";
        let context = Context::parse("@computer", text).unwrap();
        assert_eq!(context.actions, vec![]);
    }
}
