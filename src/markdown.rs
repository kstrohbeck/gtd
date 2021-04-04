use crate::parser::{DisplayableEvent, DisplayableTag, ParseError, Parser};
use pulldown_cmark::{CodeBlockKind, CowStr, Event, LinkType, Tag};
use std::{
    convert::{TryFrom, TryInto},
    error::Error,
    fmt,
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

pub fn event_static<'a>(event: Event<'a>) -> Event<'static> {
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

pub struct Doc<'a> {
    pub title: Heading,
    pub tags: Vec<String>,
    pub parser: Parser<'a>,
}

impl<'a> Doc<'a> {
    pub fn parse(text: &'a str) -> Result<Self, ParseError<'a>> {
        let mut parser = Parser::new(text);

        let title = parser.parse_heading(1)?;
        let tags = parser.parse_tags().unwrap_or_else(|_| Vec::new());

        Ok(Self {
            title,
            tags,
            parser,
        })
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

    pub fn into_events(self) -> Vec<Event<'static>> {
        self.0
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

impl fmt::Display for Heading {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct LinkParts<'a> {
            url: &'a CowStr<'a>,
            title: &'a CowStr<'a>,
        }

        impl<'a> fmt::Display for LinkParts<'a> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.url)?;
                if self.title.is_empty() {
                    write!(f, " {}", self.title)?;
                }
                Ok(())
            }
        }

        for ev in &self.0 {
            match ev {
                HeadingEvent::Start(tag) => match tag {
                    HeadingTag::Emphasis => write!(f, "_")?,
                    HeadingTag::Strong => write!(f, "**")?,
                    HeadingTag::Strikethrough => write!(f, "~~")?,
                    HeadingTag::Link(ty, _, _) => match ty {
                        LinkType::Autolink | LinkType::Email => write!(f, "<")?,
                        _ => write!(f, "[")?,
                    },
                    HeadingTag::Image(ty, _, _) => match ty {
                        LinkType::Autolink | LinkType::Email => write!(f, "!<")?,
                        _ => write!(f, "![")?,
                    },
                },
                HeadingEvent::End(tag) => match tag {
                    HeadingTag::Emphasis => write!(f, "_")?,
                    HeadingTag::Strong => write!(f, "**")?,
                    HeadingTag::Strikethrough => write!(f, "~~")?,
                    HeadingTag::Link(ty, url, title) | HeadingTag::Image(ty, url, title) => {
                        let parts = LinkParts { url, title };
                        match ty {
                            LinkType::Inline => write!(f, "]({})", parts)?,
                            LinkType::Reference | LinkType::ReferenceUnknown => {
                                write!(f, "][{}]", parts)?
                            }
                            LinkType::Collapsed | LinkType::CollapsedUnknown => write!(f, "][]")?,
                            LinkType::Shortcut | LinkType::ShortcutUnknown => write!(f, "]")?,
                            LinkType::Autolink | LinkType::Email => write!(f, ">")?,
                        }
                    }
                },
                HeadingEvent::Text(t) => write!(f, "{}", t)?,
                HeadingEvent::Code(c) => write!(f, "`{}`", c)?,
                HeadingEvent::Html(h) => write!(f, "<{}>", h)?,
                HeadingEvent::FootnoteReference(s) => write!(f, "^{}", s)?,
            }
        }
        Ok(())
    }
}

impl TryFrom<Fragment> for Heading {
    type Error = HeadingEventError<'static>;

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
    type Error = HeadingEventError<'a>;

    fn try_from(event: Event<'a>) -> Result<Self, Self::Error> {
        match event {
            Event::Start(t) => t
                .try_into()
                .map(Self::Start)
                .map_err(HeadingEventError::InvalidStartTag),
            Event::End(t) => t
                .try_into()
                .map(Self::End)
                .map_err(HeadingEventError::InvalidEndTag),
            Event::Text(s) => Ok(Self::Text(s)),
            Event::Code(s) => Ok(Self::Code(s)),
            Event::Html(s) => Ok(Self::Html(s)),
            Event::FootnoteReference(s) => Ok(Self::FootnoteReference(s)),
            e => Err(HeadingEventError::InvalidEvent(e)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HeadingEventError<'a> {
    InvalidStartTag(HeadingTagError<'a>),
    InvalidEndTag(HeadingTagError<'a>),
    InvalidEvent(Event<'a>),
}

impl<'a> fmt::Display for HeadingEventError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidStartTag(HeadingTagError(t)) => {
                write!(f, "start of {} is invalid in header", DisplayableTag(t))
            }
            Self::InvalidEndTag(HeadingTagError(t)) => {
                write!(f, "end of {} is invalid in header", DisplayableTag(t))
            }
            Self::InvalidEvent(e) => write!(f, "{} is invalid in header", DisplayableEvent(e)),
        }
    }
}

impl<'a> Error for HeadingEventError<'a> {}

#[derive(Debug, Clone, PartialEq)]
pub enum HeadingTag<'a> {
    Emphasis,
    Strong,
    Strikethrough,
    Link(LinkType, CowStr<'a>, CowStr<'a>),
    Image(LinkType, CowStr<'a>, CowStr<'a>),
}

impl<'a> TryFrom<Tag<'a>> for HeadingTag<'a> {
    type Error = HeadingTagError<'a>;

    fn try_from(tag: Tag<'a>) -> Result<Self, Self::Error> {
        match tag {
            Tag::Emphasis => Ok(Self::Emphasis),
            Tag::Strong => Ok(Self::Strong),
            Tag::Strikethrough => Ok(Self::Strikethrough),
            Tag::Link(ty, a, b) => Ok(Self::Link(ty, a, b)),
            Tag::Image(ty, a, b) => Ok(Self::Image(ty, a, b)),
            tag => Err(HeadingTagError(tag)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadingTagError<'a>(Tag<'a>);

impl<'a> fmt::Display for HeadingTagError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} is invalid in header", DisplayableTag(&self.0))
    }
}

impl<'a> Error for HeadingTagError<'a> {}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockRef {
    pub link: String,
    pub id: String,
    pub is_embedded: bool,
}

impl BlockRef {
    pub fn from_fragment(frag: &Fragment) -> Option<Self> {
        let evs = frag.as_events();

        if evs.len() != 5 {
            return None;
        }

        let is_embedded = match &evs[0] {
            Event::Text(s) if &**s == "![" => true,
            Event::Text(s) if &**s == "[" => false,
            _ => return None,
        };

        if !matches!(&evs[1], Event::Text(s) if &**s == "[") {
            return None;
        }

        let text = match &evs[2] {
            Event::Text(s) => s.clone(),
            _ => return None,
        };

        for i in [3, 4].iter() {
            if !matches!(&evs[*i], Event::Text(s) if &**s == "]") {
                return None;
            }
        }

        let text = text.to_string();
        let idx = text.find("#^")?;
        let link = text[..idx].to_string();
        let id = text[idx + 2..].to_string();

        Some(Self {
            link,
            id,
            is_embedded,
        })
    }
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

    mod block_ref {
        use super::*;

        mod from_fragment {
            use super::*;

            #[test]
            fn parses_project_ref() {
                let frag = Fragment::from_events(vec![
                    Event::Text("[".into()),
                    Event::Text("[".into()),
                    Event::Text("197001010000 Project title#^abcdef".into()),
                    Event::Text("]".into()),
                    Event::Text("]".into()),
                ]);
                let block_ref = BlockRef::from_fragment(&frag).unwrap();
                assert_eq!(block_ref.link, String::from("197001010000 Project title"));
            }

            #[test]
            fn parses_action_id() {
                let frag = Fragment::from_events(vec![
                    Event::Text("[".into()),
                    Event::Text("[".into()),
                    Event::Text("197001010000 Project title#^abcdef".into()),
                    Event::Text("]".into()),
                    Event::Text("]".into()),
                ]);
                let block_ref = BlockRef::from_fragment(&frag).unwrap();
                assert_eq!(block_ref.id, String::from("abcdef"));
            }

            #[test]
            fn parses_unembedded() {
                let frag = Fragment::from_events(vec![
                    Event::Text("[".into()),
                    Event::Text("[".into()),
                    Event::Text("197001010000 Project title#^abcdef".into()),
                    Event::Text("]".into()),
                    Event::Text("]".into()),
                ]);
                let block_ref = BlockRef::from_fragment(&frag).unwrap();
                assert!(!block_ref.is_embedded);
            }

            #[test]
            fn parses_embedded() {
                let frag = Fragment::from_events(vec![
                    Event::Text("![".into()),
                    Event::Text("[".into()),
                    Event::Text("197001010000 Project title#^abcdef".into()),
                    Event::Text("]".into()),
                    Event::Text("]".into()),
                ]);
                let block_ref = BlockRef::from_fragment(&frag).unwrap();
                assert!(block_ref.is_embedded);
            }
        }
    }
}
