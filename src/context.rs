use crate::{
    markdown::{as_embedded_block_ref, BlockRef, Doc, Fragment, Heading},
    parser,
};
use std::{error::Error, fmt};

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

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError<'a> {
    ParseError(parser::ParseError<'a>),
}

impl<'a> ParseError<'a> {
    pub fn into_static(self) -> ParseError<'static> {
        match self {
            Self::ParseError(e) => ParseError::ParseError(e.into_static()),
        }
    }
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "{}", e),
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
    use pulldown_cmark::Event;
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
