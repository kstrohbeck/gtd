//! Markdown parser and helpers.

use crate::markdown::{Fragment, Heading};
use pulldown_cmark::{CowStr, Event, Options, Parser as MarkdownParser, Tag};
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    fmt,
    iter::Peekable,
};

/// A Markdown parser.
///
/// `Parser` has single event lookahead, meaning that as long as you only need one event to
/// determine what to parse (which its internal parsing methods do,) you don't need to care about
/// backtracking.
pub struct Parser<'a> {
    parser: Peekable<MarkdownParser<'a>>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser from `text`.
    pub fn new(text: &'a str) -> Self {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let parser = MarkdownParser::new_ext(text, options).peekable();
        Self { parser }
    }

    /// Peeks at the next event in the parser without consuming it.
    pub fn peek(&mut self) -> Option<&Event<'a>> {
        self.parser.peek()
    }

    /// Parses an arbitrary event.
    ///
    /// The `matches` predicate determines if an event should be parsed at all - if it returns
    /// `true`, parsing continues; if `false`, parsing fails. In the success case, `extract` is used
    /// to extract a value from the event, which is returned. In the failure case, `expected`
    /// generates an example of the event type that the function is looking for that is used in the
    /// returned error.
    fn parse_general<F, G, H, T>(
        &mut self,
        matches: F,
        extract: G,
        expected: H,
    ) -> Result<T, ParseError<'a>>
    where
        F: Fn(&Event<'a>) -> bool,
        G: Fn(Event<'a>) -> Option<T>,
        H: Fn() -> Event<'a>,
    {
        if let Some(ev) = self.peek() {
            if matches(ev) {
                Ok(self
                    .next()
                    .and_then(extract)
                    .expect("peek not the same as next"))
            } else {
                Err(ParseError::Unexpected {
                    expected: expected(),
                    actual: Actual::Event(ev.clone()),
                })
            }
        } else {
            Err(ParseError::Unexpected {
                expected: expected(),
                actual: Actual::Eof,
            })
        }
    }

    /// Parses an unbroken chunk of text.
    fn parse_text(&mut self) -> Result<CowStr<'a>, ParseError<'a>> {
        self.parse_general(
            |ev| matches!(ev, &Event::Text(_)),
            |ev| match ev {
                Event::Text(t) => Some(t),
                _ => None,
            },
            || Event::Text(CowStr::Inlined(' '.into())),
        )
    }

    /// Parses a start `tag`.
    fn parse_start(&mut self, tag: &Tag<'a>) -> Result<Event<'a>, ParseError<'a>> {
        self.parse_general(
            |ev| matches!(ev, Event::Start(t) if t == tag),
            Some,
            || Event::Start(tag.clone()),
        )
    }

    /// Parses an end `tag`.
    fn parse_end(&mut self, tag: &Tag<'a>) -> Result<Event<'a>, ParseError<'a>> {
        self.parse_general(
            |ev| matches!(ev, Event::End(t) if t == tag),
            Some,
            || Event::End(tag.clone()),
        )
    }

    /// Parses all events until the `until` event occurs, returning the consumed events as a
    /// `Fragment`.
    pub fn parse_until(&mut self, until: Event<'a>) -> Fragment {
        let mut frag = Vec::new();

        loop {
            if self.peek().is_none() || self.peek() == Some(&until) {
                break;
            }

            frag.push(self.next().unwrap());
        }

        Fragment::from_events(frag)
    }

    /// Parses an element surrounded by start and end `tag`s given the infallible function `func`.
    fn parse_element<F, T>(&mut self, tag: &Tag<'a>, func: F) -> Result<T, ParseError<'a>>
    where
        F: Fn(&mut Self) -> T,
    {
        self.parse_element_res(tag, |p| Ok(func(p)))
    }

    /// Parses an element surrounded by start and end `tag`s given the fallible parsing function
    /// `func`.
    fn parse_element_res<F, T>(&mut self, tag: &Tag<'a>, func: F) -> Result<T, ParseError<'a>>
    where
        F: Fn(&mut Self) -> Result<T, ParseError<'a>>,
    {
        self.parse_start(tag)?;
        let output = func(self)?;
        self.parse_end(tag)?;
        Ok(output)
    }

    /// Parses a heading of the given `level`.
    pub fn parse_heading(&mut self, level: u32) -> Result<Heading, ParseError<'a>> {
        self.parse_element(&Tag::Heading(level), |p| {
            p.parse_until(Event::End(Tag::Heading(level)))
        })?
        .try_into()
        .map_err(ParseError::CouldntParseHeading)
    }

    fn parse_general_list<F, T>(
        &mut self,
        ordered: Option<u64>,
        item_parser: F,
    ) -> Result<Vec<T>, ParseError<'a>>
    where
        F: Fn(&mut Self) -> Result<T, ParseError<'a>>,
    {
        self.parse_start(&Tag::List(ordered))?;
        let mut items = Vec::new();
        while self.parse_end(&Tag::List(ordered)).is_err() {
            items.push(item_parser(self)?);
        }
        Ok(items)
    }

    /// Parses an unordered list.
    pub fn parse_list(&mut self) -> Result<Vec<Fragment>, ParseError<'a>> {
        //self.parse_general_list(None, |p| p.parse_item())
        self.parse_general_list(None, Self::parse_item)
    }

    /// Parses a single item in a list.
    fn parse_item(&mut self) -> Result<Fragment, ParseError<'a>> {
        self.parse_element(&Tag::Item, |p| p.parse_until(Event::End(Tag::Item)))
    }

    /// Parses a list of hashtags.
    pub fn parse_tags(&mut self) -> Result<Vec<String>, ParseError<'a>> {
        self.parse_element_res(&Tag::Paragraph, |p| {
            Ok(p.parse_text()?
                .split(' ')
                .flat_map(|s| s.strip_prefix('#').map(|s| s.to_string()))
                .collect())
        })
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next()
    }
}

/// An Error that happens while parsing Markdown.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError<'a> {
    /// Error when the parser expects one event but gets another.
    Unexpected {
        expected: Event<'a>,
        actual: Actual<'a>,
    },

    /// Error when the parser tries to parse a heading that contains invalid events.
    CouldntParseHeading(<Heading as TryFrom<Fragment>>::Error),
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unexpected { expected, actual } => {
                write!(f, "expected {}, got {}", DisplayableEvent(expected), actual)
            }
            Self::CouldntParseHeading(actual) => {
                write!(f, "expected heading event, got {}", actual)
            }
        }
    }
}

impl<'a> Error for ParseError<'a> {}

/// Real event received by the parser.
#[derive(Debug, Clone, PartialEq)]
pub enum Actual<'a> {
    /// Event triggered when the parser has reached the end of file.
    Eof,

    /// A standard `pulldown_cmark` event.
    Event(Event<'a>),
}

impl<'a> fmt::Display for Actual<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "end of file"),
            Self::Event(e) => write!(f, "{}", DisplayableEvent(e)),
        }
    }
}

/// Wrapper for `Event`s that allows them to be displayed.
pub struct DisplayableEvent<'a>(pub &'a Event<'a>);

impl<'a> fmt::Display for DisplayableEvent<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Event::Start(tag) => write!(f, "start of {}", DisplayableTag(tag)),
            Event::End(tag) => write!(f, "end of {}", DisplayableTag(tag)),
            Event::Text(_) => write!(f, "text"),
            Event::Code(_) => write!(f, "code"),
            Event::Html(_) => write!(f, "html"),
            Event::FootnoteReference(_) => write!(f, "footnote reference"),
            Event::SoftBreak => write!(f, "soft break"),
            Event::HardBreak => write!(f, "hard break"),
            Event::Rule => write!(f, "rule"),
            Event::TaskListMarker(_) => write!(f, "task list marker"),
        }
    }
}

/// Wrapper for `Tag`s that allows them to be displayed.
pub struct DisplayableTag<'a>(pub &'a Tag<'a>);

impl<'a> fmt::Display for DisplayableTag<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Tag::Paragraph => write!(f, "paragraph"),
            Tag::Heading(level) => write!(f, "level {} heading", level),
            Tag::BlockQuote => write!(f, "block quote"),
            Tag::CodeBlock(_) => write!(f, "code block"),
            Tag::List(None) => write!(f, "unordered list"),
            Tag::List(Some(_)) => write!(f, "ordered list"),
            Tag::Item => write!(f, "list item"),
            Tag::FootnoteDefinition(_) => write!(f, "footnote definition"),
            Tag::Table(_) => write!(f, "table"),
            Tag::TableHead => write!(f, "table head"),
            Tag::TableRow => write!(f, "table row"),
            Tag::TableCell => write!(f, "table cell"),
            Tag::Emphasis => write!(f, "emphasis"),
            Tag::Strong => write!(f, "strong"),
            Tag::Strikethrough => write!(f, "strikethrough"),
            Tag::Link(_, _, _) => write!(f, "link"),
            Tag::Image(_, _, _) => write!(f, "image"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Fragment;

    mod parse_until {
        use super::*;

        #[test]
        fn text_up_to_until_is_parsed() {
            let text = "- foo\n- bar\n";
            let mut parser = Parser::new(text);
            assert_eq!(parser.next().unwrap(), Event::Start(Tag::List(None)));
            let list = parser.parse_until(Event::End(Tag::List(None)));
            assert_eq!(
                list,
                Fragment::from_events(vec![
                    Event::Start(Tag::Item),
                    Event::Text("foo".into()),
                    Event::End(Tag::Item),
                    Event::Start(Tag::Item),
                    Event::Text("bar".into()),
                    Event::End(Tag::Item),
                ]),
            );
        }

        #[test]
        fn until_is_not_parsed() {
            let text = "- foo\n- bar\n";
            let mut parser = Parser::new(text);
            assert_eq!(parser.next().unwrap(), Event::Start(Tag::List(None)));
            let until = Event::End(Tag::List(None));
            let _list = parser.parse_until(until.clone());
            let next = parser.next();
            assert_eq!(next, Some(until));
        }

        #[test]
        fn rest_of_text_is_parsed_if_until_not_found() {
            let text = "Remaining `stuff`";
            let mut parser = Parser::new(text);
            let stuff = parser.parse_until(Event::Start(Tag::List(None)));
            assert_eq!(
                stuff,
                Fragment::from_events(vec![
                    Event::Start(Tag::Paragraph),
                    Event::Text("Remaining ".into()),
                    Event::Code("stuff".into()),
                    Event::End(Tag::Paragraph),
                ]),
            );
        }
    }

    mod parse_heading {
        use super::*;

        #[test]
        fn simple_heading_is_parsed() {
            let text = "# Heading text";
            let mut parser = Parser::new(text);
            let heading = parser.parse_heading(1);
            assert_eq!(
                heading,
                Ok(
                    Fragment::from_events(vec![Event::Text("Heading text".into())])
                        .try_into()
                        .unwrap()
                )
            );
        }

        #[test]
        fn complex_heading_is_parsed() {
            let text = "# Heading `complex` text";
            let mut parser = Parser::new(text);
            let heading = parser.parse_heading(1);
            assert_eq!(
                heading,
                Ok(Fragment::from_events(vec![
                    Event::Text("Heading ".into()),
                    Event::Code("complex".into()),
                    Event::Text(" text".into()),
                ])
                .try_into()
                .unwrap())
            );
        }

        #[test]
        fn heading_2_is_parsed() {
            let text = "## Heading text";
            let mut parser = Parser::new(text);
            let heading = parser.parse_heading(2);
            assert_eq!(
                heading,
                Ok(
                    Fragment::from_events(vec![Event::Text("Heading text".into())])
                        .try_into()
                        .unwrap()
                )
            );
        }
    }

    mod parse_list {
        use super::*;

        #[test]
        fn single_element_list_is_parsed() {
            let text = "- one";
            let mut parser = Parser::new(text);
            let list = parser.parse_list();
            assert_eq!(
                list,
                Ok(vec![Fragment::from_events(vec![Event::Text("one".into())])])
            );
        }

        #[test]
        fn multi_element_list_is_parsed() {
            let text = "- one\n- two\n  `three`";
            let mut parser = Parser::new(text);
            let list = parser.parse_list();
            assert_eq!(
                list,
                Ok(vec![
                    Fragment::from_events(vec![Event::Text("one".into())]),
                    Fragment::from_events(vec![
                        Event::Text("two".into()),
                        Event::SoftBreak,
                        Event::Code("three".into()),
                    ])
                ])
            );
        }

        #[test]
        fn element_after_list_is_preserved() {
            let text = "- one\n- two\n  `three`\n\n---";
            let mut parser = Parser::new(text);
            let _list = parser.parse_list();
            let next = parser.next();
            assert_eq!(next, Some(Event::Rule));
        }
    }

    mod parse_tags {
        use super::*;

        #[test]
        fn tags_are_parsed() {
            let text = "#foo #bar";
            let mut parser = Parser::new(text);
            let tags = parser.parse_tags();
            assert_eq!(tags, Ok(vec!["foo".into(), "bar".into()]),);
        }
    }
}
