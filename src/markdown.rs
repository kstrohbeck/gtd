use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser as MarkdownParser, Tag};
use std::iter::Peekable;

pub fn cow_str_static<'a>(cow: CowStr<'a>) -> CowStr<'static> {
    match cow {
        CowStr::Borrowed(s) => CowStr::Boxed(s.into()),
        CowStr::Boxed(s) => CowStr::Boxed(s),
        CowStr::Inlined(s) => CowStr::Inlined(s),
    }
}

fn code_block_kind_static<'a>(kind: CodeBlockKind<'a>) -> CodeBlockKind<'static> {
    match kind {
        CodeBlockKind::Fenced(f) => CodeBlockKind::Fenced(cow_str_static(f)),
        CodeBlockKind::Indented => CodeBlockKind::Indented,
    }
}

fn tag_static<'a>(tag: Tag<'a>) -> Tag<'static> {
    match tag {
        Tag::Paragraph => Tag::Paragraph,
        Tag::Heading(h) => Tag::Heading(h),
        Tag::BlockQuote => Tag::BlockQuote,
        Tag::CodeBlock(kind) => Tag::CodeBlock(code_block_kind_static(kind)),
        Tag::List(n) => Tag::List(n),
        Tag::Item => Tag::Item,
        Tag::FootnoteDefinition(s) => Tag::FootnoteDefinition(cow_str_static(s)),
        Tag::Table(align) => Tag::Table(align),
        Tag::TableHead => Tag::TableHead,
        Tag::TableRow => Tag::TableRow,
        Tag::TableCell => Tag::TableCell,
        Tag::Emphasis => Tag::Emphasis,
        Tag::Strong => Tag::Strong,
        Tag::Strikethrough => Tag::Strikethrough,
        Tag::Link(ty, a, b) => Tag::Link(ty, cow_str_static(a), cow_str_static(b)),
        Tag::Image(ty, a, b) => Tag::Image(ty, cow_str_static(a), cow_str_static(b)),
    }
}

fn event_static<'a>(event: Event<'a>) -> Event<'static> {
    match event {
        Event::Start(t) => Event::Start(tag_static(t)),
        Event::End(t) => Event::End(tag_static(t)),
        Event::Text(s) => Event::Text(cow_str_static(s)),
        Event::Code(s) => Event::Code(cow_str_static(s)),
        Event::Html(s) => Event::Html(cow_str_static(s)),
        Event::FootnoteReference(s) => Event::FootnoteReference(cow_str_static(s)),
        Event::SoftBreak => Event::SoftBreak,
        Event::HardBreak => Event::HardBreak,
        Event::Rule => Event::Rule,
        Event::TaskListMarker(b) => Event::TaskListMarker(b),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fragment(Vec<Event<'static>>);

impl Fragment {
    pub fn from_events(events: Vec<Event>) -> Self {
        Self(events.into_iter().map(event_static).collect())
    }

    pub fn as_events(&self) -> &[Event<'static>] {
        &self.0[..]
    }

    pub fn try_as_str(&self) -> Option<&str> {
        if self.0.len() == 1 {
            match &self.0[0] {
                Event::Text(t) => Some(&*t),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn try_as_title_string(&self) -> Option<String> {
        let mut s = String::new();

        for ev in &self.0 {
            match ev {
                Event::Text(t) | Event::Code(t) => s.push_str(t),
                _ => return None,
            }
        }

        Some(s)
    }
}

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

    pub fn parse_event(&mut self, req: Event<'a>) -> Option<Event<'a>> {
        if let Some(ev) = self.peek() {
            if ev == &req {
                self.next()
            } else {
                None
            }
        } else {
            None
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

    pub fn parse_heading(&mut self, heading: u32) -> Option<Fragment> {
        self.parse_event(Event::Start(Tag::Heading(heading)))?;
        let frag = self.parse_until(Event::End(Tag::Heading(heading)));
        self.parse_event(Event::End(Tag::Heading(heading)))?;
        Some(frag)
    }

    pub fn parse_list(&mut self) -> Option<Vec<Fragment>> {
        self.parse_event(Event::Start(Tag::List(None)))?;
        let items = std::iter::from_fn(|| self.parse_item()).collect();
        self.parse_event(Event::End(Tag::List(None)))?;
        Some(items)
    }

    pub fn parse_item(&mut self) -> Option<Fragment> {
        self.parse_event(Event::Start(Tag::Item))?;
        let frag = self.parse_until(Event::End(Tag::Item));
        self.parse_event(Event::End(Tag::Item))?;
        Some(frag)
    }

    pub fn parse_tasklist(&mut self) -> Option<Vec<(bool, Fragment)>> {
        self.parse_event(Event::Start(Tag::List(None)))?;
        let tasks = std::iter::from_fn(|| self.parse_task()).collect();
        self.parse_event(Event::End(Tag::List(None)))?;
        Some(tasks)
    }

    pub fn parse_task(&mut self) -> Option<(bool, Fragment)> {
        self.parse_event(Event::Start(Tag::Item))?;

        let b = match self.parser.next()? {
            Event::TaskListMarker(b) => b,
            _ => return None,
        };

        let text = self.parse_until(Event::End(Tag::Item));
        self.parse_event(Event::End(Tag::Item))?;

        Some((b, text))
    }

    pub fn parse_tags(&mut self) -> Option<Vec<String>> {
        self.parse_event(Event::Start(Tag::Paragraph))?;
        let tag_line = match self.parser.next()? {
            Event::Text(t) => t,
            _ => return None,
        };
        self.parse_event(Event::End(Tag::Paragraph))?;

        let tags = tag_line
            .split(' ')
            .flat_map(|s| s.strip_prefix('#'))
            .map(|s| s.to_string())
            .collect();

        Some(tags)
    }
}

impl<'a> Iterator for Parser<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next()
    }
}

pub fn as_obsidian_link<'a>(v: &[Event<'a>]) -> Option<CowStr<'a>> {
    if v.len() != 5 {
        return None;
    }

    // Check for brackets.
    for i in [0, 1].iter() {
        match &v[*i] {
            Event::Text(s) if &**s == "[" => {}
            _ => return None,
        }
    }

    let text = match &v[2] {
        Event::Text(s) => s.clone(),
        _ => return None,
    };

    for i in [3, 4].iter() {
        match &v[*i] {
            Event::Text(s) if &**s == "]" => {}
            _ => return None,
        }
    }

    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fragment {
        use super::*;

        mod try_as_title_string {
            use super::*;

            #[test]
            fn code_text_is_concatenated() {
                let fragment = Fragment::from_events(vec![
                    Event::Text("Foo ".into()),
                    Event::Code("bar".into()),
                    Event::Text(" baz".into()),
                ]);
                let title = fragment.try_as_title_string();
                assert_eq!(title, Some(String::from("Foo bar baz")));
            }
        }
    }

    mod parser {
        use super::*;

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
                    Fragment(vec![
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
                    Fragment(vec![
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
                    Some(Fragment(vec![Event::Text("Heading text".into())]))
                );
            }

            #[test]
            fn complex_heading_is_parsed() {
                let text = "# Heading `complex` text";
                let mut parser = Parser::new(text);
                let heading = parser.parse_heading(1);
                assert_eq!(
                    heading,
                    Some(Fragment(vec![
                        Event::Text("Heading ".into()),
                        Event::Code("complex".into()),
                        Event::Text(" text".into()),
                    ]))
                );
            }

            #[test]
            fn heading_2_is_parsed() {
                let text = "## Heading text";
                let mut parser = Parser::new(text);
                let heading = parser.parse_heading(2);
                assert_eq!(
                    heading,
                    Some(Fragment(vec![Event::Text("Heading text".into())]))
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
                assert_eq!(tags, Some(vec!["foo".into(), "bar".into()]),);
            }
        }
    }
}
