//! Helpers to wrap types from `pulldown_cmark`.

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Tag};
use std::fmt;

/// Extends the lifetime of a `pulldown_cmark::CowStr` to `'static`.
pub fn cow_str_static<'a>(cow: CowStr<'a>) -> CowStr<'static> {
    match cow {
        CowStr::Borrowed(s) => CowStr::Boxed(s.into()),
        CowStr::Boxed(s) => CowStr::Boxed(s),
        CowStr::Inlined(s) => CowStr::Inlined(s),
    }
}

/// Extends the lifetime of a `pulldown_cmark::Event` to `'static`.
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

/// Extends the lifetime of a `pulldown_cmark::Tag` to `'static`.
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

/// Extends the lifetime of a `pulldown_cmark::CodeBlockKind` to `'static`.
fn code_block_kind_static<'a>(kind: CodeBlockKind<'a>) -> CodeBlockKind<'static> {
    match kind {
        CodeBlockKind::Fenced(f) => CodeBlockKind::Fenced(cow_str_static(f)),
        CodeBlockKind::Indented => CodeBlockKind::Indented,
    }
}

/// Wrapper for `pulldown_cmark::Event`s that allows them to be displayed.
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

/// Wrapper for `pulldown_cmark::Tag`s that allows them to be displayed.
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
