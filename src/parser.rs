use crate::markdown::{Fragment, Heading};
use pulldown_cmark::{CowStr, Event, Options, Parser as MarkdownParser, Tag};
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    fmt,
    iter::Peekable,
};

pub struct Parser<'a> {
    parser: Peekable<MarkdownParser<'a>>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        let parser = MarkdownParser::new(text).peekable();
        Self { parser }
    }

    pub fn new_ext(text: &'a str, options: Options) -> Self {
        let parser = MarkdownParser::new_ext(text, options).peekable();
        Self { parser }
    }

    pub fn peek(&mut self) -> Option<&Event<'a>> {
        self.parser.peek()
    }

    // TODO: Combine parse_event, parse_text, and parse_task_marker.

    pub fn parse_event(&mut self, req: &Event<'a>) -> Result<Event<'a>, ParseError<'a>> {
        if let Some(ev) = self.peek() {
            if ev == req {
                let value = match self.next() {
                    Some(v) => v,
                    _ => panic!("peek not the same as next"),
                };

                Ok(value)
            } else {
                Err(ParseError::Unexpected {
                    expected: req.clone(),
                    actual: Actual::Event(ev.clone()),
                })
            }
        } else {
            Err(ParseError::Unexpected {
                expected: req.clone(),
                actual: Actual::Eof,
            })
        }
    }

    fn parse_text(&mut self) -> Result<CowStr<'a>, ParseError<'a>> {
        if let Some(ev) = self.peek() {
            if let Event::Text(_) = ev {
                let text = match self.next() {
                    Some(Event::Text(t)) => t,
                    _ => panic!("peek not the same as next"),
                };
                Ok(text)
            } else {
                Err(ParseError::Unexpected {
                    expected: Event::Text(CowStr::Inlined(' '.into())),
                    actual: Actual::Event(ev.clone()),
                })
            }
        } else {
            Err(ParseError::Unexpected {
                expected: Event::TaskListMarker(false),
                actual: Actual::Eof,
            })
        }
    }

    fn parse_task_marker(&mut self) -> Result<bool, ParseError<'a>> {
        if let Some(ev) = self.peek() {
            if let Event::TaskListMarker(_) = ev {
                let b = match self.next() {
                    Some(Event::TaskListMarker(b)) => b,
                    _ => panic!("peek not the same as next"),
                };
                Ok(b)
            } else {
                Err(ParseError::Unexpected {
                    expected: Event::TaskListMarker(false),
                    actual: Actual::Event(ev.clone()),
                })
            }
        } else {
            Err(ParseError::Unexpected {
                expected: Event::TaskListMarker(false),
                actual: Actual::Eof,
            })
        }
    }

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

    fn parse_start(&mut self, tag: Tag<'a>) -> Result<Event<'a>, ParseError<'a>> {
        self.parse_event(&Event::Start(tag))
    }

    fn parse_end(&mut self, tag: Tag<'a>) -> Result<Event<'a>, ParseError<'a>> {
        self.parse_event(&Event::End(tag))
    }

    fn parse_element<F, T>(&mut self, tag: Tag<'a>, func: F) -> Result<T, ParseError<'a>>
    where
        F: Fn(&mut Self) -> T,
    {
        self.parse_start(tag.clone())?;
        let output = func(self);
        self.parse_end(tag)?;
        Ok(output)
    }

    pub fn parse_heading(&mut self, heading: u32) -> Result<Heading, ParseError<'a>> {
        let frag = self.parse_element(Tag::Heading(heading), |p| {
            p.parse_until(Event::End(Tag::Heading(heading)))
        })?;

        frag.try_into().map_err(ParseError::CouldntParseHeading)
    }

    pub fn parse_list(&mut self) -> Result<Vec<Fragment>, ParseError<'a>> {
        self.parse_element(Tag::List(None), |p| {
            std::iter::from_fn(|| p.parse_item().ok()).collect()
        })
    }

    fn parse_item(&mut self) -> Result<Fragment, ParseError<'a>> {
        self.parse_element(Tag::Item, |p| p.parse_until(Event::End(Tag::Item)))
    }

    pub fn parse_tasklist(&mut self) -> Result<Vec<(bool, Fragment)>, ParseError<'a>> {
        self.parse_element(Tag::List(None), |p| {
            std::iter::from_fn(|| p.parse_task().ok()).collect()
        })
    }

    fn parse_task(&mut self) -> Result<(bool, Fragment), ParseError<'a>> {
        self.parse_start(Tag::Item)?;
        let b = self.parse_task_marker()?;
        let text = self.parse_until(Event::End(Tag::Item));
        self.parse_end(Tag::Item)?;
        Ok((b, text))
    }

    pub fn parse_tags(&mut self) -> Result<Vec<String>, ParseError<'a>> {
        self.parse_start(Tag::Paragraph)?;
        let text = self.parse_text()?;
        self.parse_end(Tag::Paragraph)?;

        let tags = text
            .split(' ')
            .flat_map(|s| s.strip_prefix('#').map(|s| s.to_string()))
            .collect();

        Ok(tags)
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError<'a> {
    Unexpected {
        expected: Event<'a>,
        actual: Actual<'a>,
    },
    CouldntParseHeading(<Heading as TryFrom<Fragment>>::Error),
    Custom(String),
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Unexpected { expected, actual } => {
                write!(f, "expected ")?;
                fmt_event(expected, f)?;
                write!(f, ", got {}", actual)
            }
            Self::CouldntParseHeading(actual) => {
                write!(f, "expected heading event, got {}", actual)
            }
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl<'a> Error for ParseError<'a> {}

#[derive(Debug, Clone, PartialEq)]
pub enum Actual<'a> {
    Eof,
    Event(Event<'a>),
}

impl<'a> fmt::Display for Actual<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eof => write!(f, "end of file"),
            Self::Event(e) => fmt_event(e, f),
        }
    }
}

pub fn fmt_event<'a>(event: &Event<'a>, f: &mut fmt::Formatter) -> fmt::Result {
    match event {
        Event::Start(tag) => {
            write!(f, "start of ")?;
            fmt_tag(tag, f)
        }
        Event::End(tag) => {
            write!(f, "end of ")?;
            fmt_tag(tag, f)
        }
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

pub fn fmt_tag<'a>(tag: &Tag<'a>, f: &mut fmt::Formatter) -> fmt::Result {
    match tag {
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
