use crate::markdown::{as_embedded_block_ref, BlockRef, Doc, Fragment, Heading};
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
pub enum Action {
    Literal(Fragment),
    Reference(BlockRef),
}

impl Action {
    pub fn from_fragment(fragment: Fragment) -> Self {
        match as_embedded_block_ref(fragment.as_events()) {
            Some(block_ref) => Self::Reference(block_ref),
            None => Self::Literal(fragment),
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
        let text = "# @computer\n\n- foo\n- ![[bar#^baz]]\n";
        let context = Context::parse("@computer", text).unwrap();
        assert_eq!(
            context.actions,
            vec![
                Action::Literal(Fragment::from_events(vec![Event::Text("foo".into())])),
                Action::Reference(BlockRef {
                    link: String::from("bar"),
                    id: String::from("baz")
                }),
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
