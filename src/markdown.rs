use pulldown_cmark::{
    CodeBlockKind, CowStr, Event, LinkType, Options, Parser as MarkdownParser, Tag,
};
use std::{
    convert::{TryFrom, TryInto},
    iter::Peekable,
};

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Heading(Vec<HeadingEvent<'static>>);

impl Heading {
    pub fn try_as_str(&self) -> Option<&str> {
        if self.0.len() == 1 {
            match &self.0[0] {
                HeadingEvent::Text(t) => Some(&*t),
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
                HeadingEvent::Text(t) | HeadingEvent::Code(t) => s.push_str(t),
                _ => return None,
            }
        }

        Some(s)
    }
}

impl TryFrom<Fragment> for Heading {
    type Error = ();

    fn try_from(mut fragment: Fragment) -> Result<Self, Self::Error> {
        Ok(Heading(
            fragment
                .0
                .drain(..)
                .map(HeadingEvent::try_from)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeadingEvent<'a> {
    Start(HeadingTag<'a>),
    End(HeadingTag<'a>),
    Text(CowStr<'a>),
    Code(CowStr<'a>),
    Html(CowStr<'a>),
    FootnoteReference(CowStr<'a>),
}

impl<'a> TryFrom<Event<'a>> for HeadingEvent<'a> {
    type Error = ();

    fn try_from(event: Event<'a>) -> Result<Self, Self::Error> {
        match event {
            Event::Start(t) => t.try_into().map(Self::Start),
            Event::End(t) => t.try_into().map(Self::End),
            Event::Text(s) => Ok(Self::Text(s)),
            Event::Code(s) => Ok(Self::Code(s)),
            Event::Html(s) => Ok(Self::Html(s)),
            Event::FootnoteReference(s) => Ok(Self::FootnoteReference(s)),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeadingTag<'a> {
    Emphasis,
    Strong,
    Strikethrough,
    Link(LinkType, CowStr<'a>, CowStr<'a>),
    Image(LinkType, CowStr<'a>, CowStr<'a>),
}

impl<'a> TryFrom<Tag<'a>> for HeadingTag<'a> {
    type Error = ();

    fn try_from(tag: Tag<'a>) -> Result<Self, Self::Error> {
        match tag {
            Tag::Emphasis => Ok(Self::Emphasis),
            Tag::Strong => Ok(Self::Strong),
            Tag::Strikethrough => Ok(Self::Strikethrough),
            Tag::Link(ty, a, b) => Ok(Self::Link(ty, a, b)),
            Tag::Image(ty, a, b) => Ok(Self::Image(ty, a, b)),
            _ => Err(()),
        }
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

    pub fn parse_event(&mut self, req: &Event<'a>) -> Option<Event<'a>> {
        if let Some(ev) = self.peek() {
            if ev == req {
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

    fn parse_start(&mut self, tag: Tag<'a>) -> Option<Event<'a>> {
        self.parse_event(&Event::Start(tag))
    }

    fn parse_end(&mut self, tag: Tag<'a>) -> Option<Event<'a>> {
        self.parse_event(&Event::End(tag))
    }

    fn parse_element_opt<F, T>(&mut self, tag: Tag<'a>, func: F) -> Option<T>
    where
        F: Fn(&mut Self) -> Option<T>,
    {
        self.parse_start(tag.clone())?;
        let output = func(self)?;
        self.parse_end(tag)?;
        Some(output)
    }

    fn parse_element<F, T>(&mut self, tag: Tag<'a>, func: F) -> Option<T>
    where
        F: Fn(&mut Self) -> T,
    {
        self.parse_element_opt(tag, |p| Some(func(p)))
    }

    pub fn parse_heading(&mut self, heading: u32) -> Option<Heading> {
        self.parse_element(Tag::Heading(heading), |p| {
            p.parse_until(Event::End(Tag::Heading(heading)))
        })
        .and_then(|frag| frag.try_into().ok())
    }

    pub fn parse_list(&mut self) -> Option<Vec<Fragment>> {
        self.parse_element(Tag::List(None), |p| {
            std::iter::from_fn(|| p.parse_item()).collect()
        })
    }

    fn parse_item(&mut self) -> Option<Fragment> {
        self.parse_element(Tag::Item, |p| p.parse_until(Event::End(Tag::Item)))
    }

    pub fn parse_tasklist(&mut self) -> Option<Vec<(bool, Fragment)>> {
        self.parse_element(Tag::List(None), |p| {
            std::iter::from_fn(|| p.parse_task()).collect()
        })
    }

    fn parse_task(&mut self) -> Option<(bool, Fragment)> {
        self.parse_element_opt(Tag::Item, |p| {
            let b = match p.next()? {
                Event::TaskListMarker(b) => b,
                _ => return None,
            };

            let text = p.parse_until(Event::End(Tag::Item));
            Some((b, text))
        })
    }

    pub fn parse_tags(&mut self) -> Option<Vec<String>> {
        self.parse_element_opt(Tag::Paragraph, |p| match p.next()? {
            Event::Text(t) => Some(t),
            _ => None,
        })
        .map(|line| {
            line.split(' ')
                .flat_map(|s| s.strip_prefix('#'))
                .map(|s| s.to_string())
                .collect()
        })
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

    mod heading {
        use super::*;

        mod try_as_title_string {
            use super::*;

            #[test]
            fn code_text_is_concatenated() {
                let heading = Heading(vec![
                    HeadingEvent::Text("Foo ".into()),
                    HeadingEvent::Code("bar".into()),
                    HeadingEvent::Text(" baz".into()),
                ]);

                let title = heading.try_as_title_string();
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
                    Some(Heading(vec![HeadingEvent::Text("Heading text".into())]))
                );
            }

            #[test]
            fn complex_heading_is_parsed() {
                let text = "# Heading `complex` text";
                let mut parser = Parser::new(text);
                let heading = parser.parse_heading(1);
                assert_eq!(
                    heading,
                    Some(Heading(vec![
                        HeadingEvent::Text("Heading ".into()),
                        HeadingEvent::Code("complex".into()),
                        HeadingEvent::Text(" text".into()),
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
                    Some(Heading(vec![HeadingEvent::Text("Heading text".into())]))
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
