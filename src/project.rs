use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};

#[derive(Debug, Clone)]
pub struct Project<'a> {
    title: Vec<Event<'a>>,
    // TODO: Have this be a Vec<CowStr<'a>>.
    tags: Vec<String>,
}

impl<'a> Project<'a> {
    pub fn parse(text: &'a str) -> Option<Self> {
        let options =
            Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_TASKLISTS;
        let mut parser = Parser::new_ext(text, options);

        let title = parse_title(&mut parser)?;
        let tags = parse_tags(&mut parser)?;
        let has_gtd_tag = tags.iter().any(|s| s == "#gtd");
        let has_project_tag = tags.iter().any(|s| s == "#project");
        if !has_gtd_tag || !has_project_tag {
            return None;
        }

        Some(Self { title, tags })
    }
}

macro_rules! require {
    ( $parser:expr, $ev:expr ) => {
        if $parser.next()? != $ev {
            return None;
        }
    };
}

fn parse_title<'a>(parser: &mut Parser<'a>) -> Option<Vec<Event<'a>>> {
    require!(parser, Event::Start(Tag::Heading(1)));

    let mut title_vec = Vec::new();
    loop {
        let ev = parser.next()?;
        if let Event::End(Tag::Heading(1)) = ev {
            break;
        }
        title_vec.push(ev);
    }
    Some(title_vec)
}

fn parse_tags(parser: &mut Parser) -> Option<Vec<String>> {
    require!(parser, Event::Start(Tag::Paragraph));
    let tag_line = match parser.next()? {
        Event::Text(t) => t,
        _ => return None,
    };
    require!(parser, Event::End(Tag::Paragraph));
    let tags = tag_line.split(' ').map(|s| s.to_string()).collect();
    Some(tags)
}
