use crate::{
    markdown::{BlockRef, Fragment, Heading},
    parser::{self, Doc},
    project::ActionRef,
};
use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub name: Name,
    pub title: Heading,
    actions: Vec<Action>,
}

impl Context {
    pub fn parse<S: Into<String>>(filename: S, text: &str) -> Result<Self, ParseError> {
        let name = Name(filename.into());

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
            name,
            title,
            actions,
        })
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions[..]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

impl Name {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Literal(Fragment),
    Reference(ActionRef),
}

impl Action {
    pub fn from_fragment(fragment: Fragment) -> Self {
        match BlockRef::from_fragment(&fragment) {
            Some(block_ref) => Self::Reference(ActionRef::from_block_ref(block_ref).unwrap()),
            None => Self::Literal(fragment),
        }
    }

    pub fn to_action_ref(&self) -> Option<&ActionRef> {
        match self {
            Action::Literal(_) => None,
            Action::Reference(action_ref) => Some(action_ref),
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
    use crate::markdown::BlockRef;
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
        let text = "# @computer\n\n- foo\n- ![[197001010000 bar#^abcdef]]\n";
        let context = Context::parse("@computer", text).unwrap();
        assert_eq!(
            context.actions,
            vec![
                Action::Literal(Fragment::from_events(vec![Event::Text("foo".into())])),
                Action::Reference(
                    ActionRef::from_block_ref(BlockRef {
                        link: String::from("197001010000 bar"),
                        id: String::from("abcdef"),
                        is_embedded: true,
                    })
                    .unwrap()
                ),
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
