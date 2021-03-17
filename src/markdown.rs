use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};
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

pub fn parse_event<'a, I>(mut parser: I, req: Event<'a>) -> Option<Event<'a>>
where
    I: Iterator<Item = Event<'a>>,
{
    parser.next().filter(|ev| ev == &req)
}

pub fn parse_until<'a, I>(parser: &mut Peekable<I>, until: Event<'a>) -> Fragment
where
    I: Iterator<Item = Event<'a>>,
{
    let mut frag = Vec::new();

    loop {
        if parser.peek().is_none() || parser.peek() == Some(&until) {
            break;
        }

        frag.push(parser.next().unwrap());
    }

    Fragment::from_events(frag)
}

pub fn parse_until_incl<'a, I>(parser: &mut I, until: Event<'a>) -> Fragment
where
    I: Iterator<Item = Event<'a>>,
{
    let frag = parser.take_while(|p| p != &until).collect();
    Fragment::from_events(frag)
}

pub fn parse_heading<'a, I>(mut parser: I, heading: u32) -> Option<Fragment>
where
    I: Iterator<Item = Event<'a>>,
{
    parse_event(&mut parser, Event::Start(Tag::Heading(heading)))?;
    let frag = parse_until_incl(&mut parser, Event::End(Tag::Heading(heading)));
    Some(frag)
}

pub fn parse_list<'a, I>(mut parser: I) -> Option<Vec<Fragment>>
where
    I: Iterator<Item = Event<'a>>,
{
    parse_event(&mut parser, Event::Start(Tag::List(None)))?;
    let items = std::iter::from_fn(|| parse_item(&mut parser)).collect();
    Some(items)
}

pub fn parse_item<'a, I>(mut parser: I) -> Option<Fragment>
where
    I: Iterator<Item = Event<'a>>,
{
    parse_event(&mut parser, Event::Start(Tag::Item))?;
    let text = parse_until_incl(&mut parser, Event::End(Tag::Item));
    Some(text)
}

pub fn parse_tasklist<'a, I>(mut parser: I) -> Option<Vec<(bool, Fragment)>>
where
    I: Iterator<Item = Event<'a>>,
{
    parse_event(&mut parser, Event::Start(Tag::List(None)))?;
    let tasks = std::iter::from_fn(|| parse_task(&mut parser)).collect();
    Some(tasks)
}

pub fn parse_task<'a, I>(mut parser: I) -> Option<(bool, Fragment)>
where
    I: Iterator<Item = Event<'a>>,
{
    parse_event(&mut parser, Event::Start(Tag::Item))?;

    let b = match parser.next()? {
        Event::TaskListMarker(b) => b,
        _ => return None,
    };

    let text = parse_until_incl(&mut parser, Event::End(Tag::Item));

    Some((b, text))
}

// TODO: Should return borrowed, and also error if gtd-project isn't found.
pub fn parse_tags<'a, I>(mut parser: I) -> Option<Vec<String>>
where
    I: Iterator<Item = Event<'a>>,
{
    parse_event(&mut parser, Event::Start(Tag::Paragraph));
    let tag_line = match parser.next()? {
        Event::Text(t) => t,
        _ => return None,
    };
    parse_event(&mut parser, Event::End(Tag::Paragraph));

    let tags = tag_line
        .split(' ')
        .flat_map(|s| s.strip_prefix('#'))
        .map(|s| s.to_string())
        .collect();

    Some(tags)
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
    use pulldown_cmark::Parser;

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

    mod parse_until {
        use super::*;

        #[test]
        fn text_up_to_until_is_parsed() {
            let text = "- foo\n- bar\n";
            let mut parser = Parser::new(text).peekable();
            assert_eq!(parser.next().unwrap(), Event::Start(Tag::List(None)));
            let list = parse_until(&mut parser, Event::End(Tag::List(None)));
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
            let mut parser = Parser::new(text).peekable();
            assert_eq!(parser.next().unwrap(), Event::Start(Tag::List(None)));
            let until = Event::End(Tag::List(None));
            let _list = parse_until(&mut parser, until.clone());
            let next = parser.next();
            assert_eq!(next, Some(until));
        }

        #[test]
        fn rest_of_text_is_parsed_if_until_not_found() {
            let text = "Remaining `stuff`";
            let mut parser = Parser::new(text).peekable();
            let stuff = parse_until(&mut parser, Event::Start(Tag::List(None)));
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
            let heading = parse_heading(&mut parser, 1);
            assert_eq!(
                heading,
                Some(Fragment(vec![Event::Text("Heading text".into())]))
            );
        }

        #[test]
        fn complex_heading_is_parsed() {
            let text = "# Heading `complex` text";
            let mut parser = Parser::new(text);
            let heading = parse_heading(&mut parser, 1);
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
            let heading = parse_heading(&mut parser, 2);
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
            let tags = parse_tags(&mut parser);
            assert_eq!(tags, Some(vec!["foo".into(), "bar".into()]),);
        }
    }
}
