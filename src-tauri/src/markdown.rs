//! Markdown の翻訳単位 (doc-1 §7.1).
//!
//! The original is parsed with pulldown-cmark, whose events carry their place in the original,
//! and the translation is the original with each 翻訳単位's place taken by its translation. The
//! markup around the 翻訳単位 — list markers, `>`, table pipes, code blocks, HTML blocks, front matter,
//! link definitions — is copied rather than written again, so it comes out exactly as it went in.
//!
//! A 翻訳単位 is the inline content of one block: a paragraph, a heading, a table cell, or a tight
//! list item, whose text pulldown-cmark gives without a paragraph around it. Its text is cut from
//! the original as written (escapes and entities stay), with the inline elements as 番号付きのタグ.

use std::borrow::Cow;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

use crate::document::{Lines, Segment, Unit};
use crate::tags::Builder;

/// What the parser reads beyond CommonMark: GitHub's tables, strikethrough, task lists,
/// footnotes and alerts (`> [!NOTE]`, whose marker is not text), front matter, and heading
/// attributes (`# Title {#id}`, which are not text either).
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// `text` as the 翻訳単位 of its blocks and the original between them.
pub fn segments(text: &str) -> Vec<Segment<'_>> {
    let events: Vec<(Event, Range<usize>)> = Parser::new_ext(text, options())
        .into_offset_iter()
        .collect();
    let mut segments = Vec::new();
    let mut kept_from = 0;
    // The blocks around the current event, innermost last.
    let mut blocks: Vec<Block> = Vec::new();
    // Inside a code block, an HTML block or front matter, whose text is not translated.
    let mut verbatim = false;
    let mut i = 0;
    while i < events.len() {
        let (event, _) = &events[i];
        if !verbatim && is_inline(event) {
            let start = i;
            while i < events.len() && is_inline(&events[i].0) {
                i += 1;
            }
            let block = blocks.last().copied().unwrap_or(Block::Other);
            if let Some((range, unit)) = unit(text, &events[start..i], block) {
                segments.push(Segment::Kept(&text[kept_from..range.start]));
                segments.push(Segment::Unit(unit));
                kept_from = range.end;
            }
            continue;
        }
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock | Tag::MetadataBlock(_)) => {
                verbatim = true;
            }
            Event::End(TagEnd::CodeBlock | TagEnd::HtmlBlock | TagEnd::MetadataBlock(_)) => {
                verbatim = false;
            }
            Event::Start(tag) => blocks.push(match tag {
                Tag::Heading { .. } => Block::Heading,
                Tag::TableCell => Block::Cell,
                _ => Block::Other,
            }),
            Event::End(_) => {
                blocks.pop();
            }
            _ => {}
        }
        i += 1;
    }
    segments.push(Segment::Kept(&text[kept_from..]));
    segments
}

/// The block a 翻訳単位 stands in, as far as writing its translation back goes.
#[derive(Debug, Clone, Copy)]
enum Block {
    Heading,
    Cell,
    Other,
}

/// What tells `text` apart as Markdown (doc-1 §7.2), read with the parser it would be translated
/// by.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Signs {
    /// How many headings written with `#`, lists and fenced code blocks it has. A heading
    /// underlined with `-` or `=` is not counted: a plain-text mail's `--` before its signature
    /// underlines the line above it.
    pub syntax: usize,
    /// Where its fenced code blocks and code spans are, whose content is an example rather than
    /// the text's own markup.
    pub code: Vec<Range<usize>>,
}

/// The [`Signs`] of `text`.
pub fn signs(text: &str) -> Signs {
    let mut signs = Signs::default();
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { .. })
                if text[range.clone()].trim_start().starts_with('#') =>
            {
                signs.syntax += 1;
            }
            Event::Start(Tag::List(_)) => signs.syntax += 1,
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                signs.syntax += 1;
                signs.code.push(range);
            }
            Event::Code(_) => signs.code.push(range),
            _ => {}
        }
    }
    signs
}

fn is_inline(event: &Event) -> bool {
    match event {
        Event::Start(tag) => matches!(
            tag,
            Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
        ),
        Event::End(tag) => matches!(
            tag,
            TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::Link
                | TagEnd::Image
        ),
        Event::Text(_)
        | Event::Code(_)
        | Event::InlineMath(_)
        | Event::InlineHtml(_)
        | Event::FootnoteReference(_)
        | Event::SoftBreak
        | Event::HardBreak => true,
        Event::Html(_) | Event::DisplayMath(_) | Event::Rule | Event::TaskListMarker(_) => false,
    }
}

/// A paired element open while its 翻訳単位 is read.
struct Open {
    /// Where its text starts in the original.
    content: usize,
    /// For a link or image, how it names its destination.
    link: Option<LinkType>,
}

/// The 翻訳単位 made of the inline `events` of one block, and its place in `text`; `None` when it
/// has nothing to translate.
fn unit<'a>(
    text: &'a str,
    events: &[(Event, Range<usize>)],
    block: Block,
) -> Option<(Range<usize>, Unit<'a>)> {
    let start = events.first()?.1.start;
    let last = events.last()?.1.end;
    let end = start + text[start..last].trim_end().len();
    let mut b = Builder::default();
    let mut open: Vec<Open> = Vec::new();
    let mut cursor = start;
    let mut k = 0;
    while k < events.len() {
        let (event, range) = &events[k];
        let next_start = events.get(k + 1).map_or(end, |(_, next)| next.start);
        match event {
            Event::Text(_) => {}
            Event::SoftBreak => {
                written(&mut b, &text[cursor..range.start]);
                b.text("\n");
                cursor = past_line_prefix(text, range.end, next_start);
            }
            Event::HardBreak => {
                written(&mut b, &text[cursor..range.start]);
                let to = past_line_prefix(text, range.end, next_start);
                b.void("br", &text[range.start..to], "\n");
                cursor = to;
            }
            Event::Code(code) | Event::InlineMath(code) => {
                written(&mut b, &text[cursor..range.start]);
                b.kept("code", &text[range.clone()], code);
                cursor = range.end;
            }
            Event::InlineHtml(_) | Event::FootnoteReference(_) => {
                written(&mut b, &text[cursor..range.start]);
                let original = &text[range.clone()];
                b.void("x", original, original);
                cursor = range.end;
            }
            Event::Start(Tag::Link {
                link_type: LinkType::Autolink | LinkType::Email,
                dest_url,
                ..
            }) => {
                written(&mut b, &text[cursor..range.start]);
                b.kept("code", &text[range.clone()], dest_url);
                cursor = range.end;
                k = end_of(events, k);
            }
            Event::Start(tag) => {
                written(&mut b, &text[cursor..range.start]);
                if matches!(events.get(k + 1), Some((Event::End(_), _))) {
                    // Nothing inside to translate, as in `[](#top)`.
                    b.void("x", &text[range.clone()], "");
                    cursor = range.end;
                    k += 2;
                    continue;
                }
                let delimiter = &text[range.start..next_start];
                let (name, markup, link) = match tag {
                    Tag::Emphasis => ("em", star(delimiter), None),
                    Tag::Strong => ("strong", star(delimiter), None),
                    Tag::Strikethrough => ("s", delimiter.to_owned(), None),
                    Tag::Link { link_type, .. } | Tag::Image { link_type, .. } => {
                        ("a", delimiter.to_owned(), Some(*link_type))
                    }
                    _ => unreachable!("only inline elements are in a 翻訳単位"),
                };
                b.open(name, markup);
                open.push(Open {
                    content: next_start,
                    link,
                });
                cursor = next_start;
            }
            Event::End(tag) => {
                let element = open.pop().expect("an inline element ends after it starts");
                let content_end = events[k - 1].1.end.max(element.content);
                if cursor < content_end {
                    written(&mut b, &text[cursor..content_end]);
                }
                let delimiter = &text[content_end..range.end];
                let markup = match (tag, element.link) {
                    (TagEnd::Emphasis | TagEnd::Strong, _) => star(delimiter),
                    // `[label]` and `[label][]` take their destination from the label, which the
                    // translation changes: the original label goes after it instead.
                    (_, Some(LinkType::Shortcut | LinkType::Collapsed)) => {
                        format!("][{}]", &text[element.content..content_end])
                    }
                    _ => delimiter.to_owned(),
                };
                b.close(markup);
                cursor = range.end.max(cursor);
                // pulldown-cmark leaves the `[]` of `[label][]` out of the link.
                if element.link == Some(LinkType::Collapsed) && text[cursor..].starts_with("[]") {
                    cursor += "[]".len();
                }
            }
            _ => {}
        }
        k += 1;
    }
    // The `[]` of a `[label][]` that ends the block lies past the last event.
    let end = end.max(cursor);
    written(&mut b, &text[cursor..end]);

    if !b.has_words() {
        return None;
    }
    let (unit_text, tags) = b.finish();
    let lines = match block {
        Block::Heading => Lines::Join,
        Block::Cell => Lines::Cell,
        Block::Other => Lines::Keep(continuation_prefix(text, start)),
    };
    let unit = Unit {
        text: Cow::Owned(unit_text),
        original: &text[start..end],
        tags,
        lines,
        attributes: Vec::new(),
    };
    Some((start..end, unit))
}

/// The index of the event that ends the element starting at `events[start]`.
fn end_of(events: &[(Event, Range<usize>)], start: usize) -> usize {
    let mut depth = 0;
    for (k, (event, _)) in events.iter().enumerate().skip(start) {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return k;
                }
            }
            _ => {}
        }
    }
    events.len() - 1
}

/// Emphasis written with `*` rather than `_`: Japanese and Chinese put no space around words, and
/// `_` does not open or close emphasis inside a word (`これは_強調_です` stays as it is).
fn star(delimiter: &str) -> String {
    delimiter.replace('_', "*")
}

/// Where a line inside a block starts after the markup that carries the block on (`>`, the
/// indent of a list item), from `from` up to `limit`.
fn past_line_prefix(text: &str, from: usize, limit: usize) -> usize {
    let prefix = text[from..limit]
        .find(|c: char| !matches!(c, ' ' | '\t' | '>'))
        .unwrap_or(limit - from);
    from + prefix
}

/// The markup that carries a block on to its next line, from how its first line starts at
/// `start`: each `>` stays and the rest (`-`, `1.`, `[^1]:`) becomes as many spaces, so
/// `> - item` goes on as `>   `.
fn continuation_prefix(text: &str, start: usize) -> String {
    let line_start = text[..start].rfind('\n').map_or(0, |at| at + 1);
    text[line_start..start]
        .chars()
        .map(|c| if matches!(c, '>' | '\t') { c } else { ' ' })
        .collect()
}

/// Text of the original as written, with each bare URL in it kept as it is.
fn written(b: &mut Builder, text: &str) {
    let mut rest = text;
    while let Some(url) = find_url(rest) {
        b.text(&rest[..url.start]);
        b.kept("code", &rest[url.clone()], &rest[url.clone()]);
        rest = &rest[url.end..];
    }
    b.text(rest);
}

/// The first `http://` or `https://` URL in `text` that does not go on from a word of ASCII
/// letters (Japanese sets a URL right after its words: `詳細はhttps://…`). It ends before whitespace,
/// characters outside ASCII, and the punctuation that closes a sentence or a bracket around it:
/// `(see https://example.com/a_(b).)` gives `https://example.com/a_(b)`.
fn find_url(text: &str) -> Option<Range<usize>> {
    let mut from = 0;
    loop {
        let at = from + text[from..].find("http")?;
        let rest = &text[at..];
        let scheme = ["https://", "http://"]
            .into_iter()
            .find(|scheme| rest.starts_with(scheme));
        let starts_word = !text[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '/');
        let Some(scheme) = scheme.filter(|_| starts_word) else {
            from = at + "http".len();
            continue;
        };
        let len = rest
            .find(|c: char| !c.is_ascii_graphic() || matches!(c, '<' | '>' | '"' | '`'))
            .unwrap_or(rest.len());
        let mut url = &rest[..len];
        loop {
            let trimmed = url.trim_end_matches(['.', ',', ':', ';', '!', '?', '\'', '*', '_']);
            let trimmed = match trimmed.chars().next_back() {
                Some(close @ (')' | ']')) => {
                    let open = if close == ')' { '(' } else { '[' };
                    let unbalanced = trimmed.matches(close).count() > trimmed.matches(open).count();
                    if unbalanced {
                        &trimmed[..trimmed.len() - 1]
                    } else {
                        trimmed
                    }
                }
                _ => trimmed,
            };
            if trimmed.len() == url.len() {
                break;
            }
            url = trimmed;
        }
        if url.len() > scheme.len() {
            return Some(at..at + url.len());
        }
        from = at + scheme.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each segment as `=kept` or `[unit]` with its line mode, for comparing at a glance.
    fn show(text: &str) -> Vec<String> {
        segments(text)
            .into_iter()
            .filter_map(|segment| match segment {
                Segment::Kept("") => None,
                Segment::Kept(kept) => Some(format!("={kept}")),
                Segment::Unit(unit) => {
                    let lines = match &unit.lines {
                        Lines::Keep(prefix) if prefix.is_empty() => String::new(),
                        Lines::Keep(prefix) => format!(" keep {prefix:?}"),
                        Lines::Join => " join".to_owned(),
                        Lines::Cell => " cell".to_owned(),
                        Lines::Json => " json".to_owned(),
                        Lines::Strings(quote) => format!(" strings {quote}"),
                        Lines::Attribute(quote) => format!(" attribute {quote}"),
                    };
                    Some(format!("[{}]{lines}", unit.text))
                }
            })
            .collect()
    }

    /// The first 翻訳単位 of `text`.
    fn the_unit(text: &str) -> Unit<'_> {
        let mut units = segments(text)
            .into_iter()
            .filter_map(|segment| match segment {
                Segment::Unit(unit) => Some(unit),
                Segment::Kept(_) => None,
            });
        units.next().expect("a 翻訳単位")
    }

    #[test]
    fn blocks_are_units_and_their_markup_is_kept() {
        assert_eq!(
            show("# Title\n\nA paragraph\nwrapped.\n\n- one\n- two\n\n> quoted\n"),
            [
                "=# ",
                "[Title] join",
                "=\n\n",
                "[A paragraph\nwrapped.]",
                "=\n\n- ",
                "[one] keep \"  \"",
                "=\n- ",
                "[two] keep \"  \"",
                "=\n\n> ",
                "[quoted] keep \"> \"",
                "=\n",
            ]
        );
    }

    #[test]
    fn code_html_front_matter_and_link_definitions_are_kept() {
        let text = "---\ntitle: Front matter\n---\n\n```rust\nlet text = \"code\";\n```\n\n    indented code\n\n<div>\nHTML block\n</div>\n\n[ref]: https://example.com \"Title\"\n\nText.\n";
        assert_eq!(
            show(text),
            [
                "=---\ntitle: Front matter\n---\n\n```rust\nlet text = \"code\";\n```\n\n    indented code\n\n<div>\nHTML block\n</div>\n\n[ref]: https://example.com \"Title\"\n\n",
                "[Text.]",
                "=\n",
            ]
        );
    }

    #[test]
    fn table_cells_are_units_of_their_own() {
        assert_eq!(
            show("| Name | Note |\n|---|:-:|\n| a \\| b | 42 |\n"),
            [
                "=| ",
                "[Name] cell",
                "= | ",
                "[Note] cell",
                "= |\n|---|:-:|\n| ",
                "[a \\| b] cell",
                "= | 42 |\n",
            ]
        );
    }

    #[test]
    fn a_line_inside_a_quote_or_item_loses_its_prefix_and_gets_it_back_from_lines() {
        let unit = the_unit("> > first line\n> > second *line*\n");
        assert_eq!(unit.text, "first line\nsecond <em i=\"1\">line</em>");
        assert_eq!(unit.lines, Lines::Keep("> > ".into()));
        let unit = the_unit("1. first\n   lazy second\n");
        assert_eq!(unit.text, "first\nlazy second");
        assert_eq!(unit.lines, Lines::Keep("   ".into()));
        let unit = the_unit("> - [ ] task\n>   more\n");
        assert_eq!(unit.text, "task\nmore");
        assert_eq!(unit.lines, Lines::Keep(">       ".into()));
    }

    #[test]
    fn inline_elements_become_tags_and_the_plain_text_drops_them() {
        let unit = the_unit(
            "Run `npm i` *now* or **see [the docs](https://x.dev \"Docs\")** ~~later~~ ![a logo](logo.png).",
        );
        assert_eq!(
            unit.text,
            "Run <code i=\"1\">npm i</code> <em i=\"2\">now</em> or <strong i=\"3\">see <a i=\"4\">the docs</a></strong> \
             <s i=\"5\">later</s> <a i=\"6\">a logo</a>."
        );
        let tags = unit.tags.unwrap();
        assert_eq!(
            tags.plain(),
            "Run `npm i` now or see the docs later a logo."
        );
        let restored = tags.restore(
            "<code i=\"1\">x</code> <em i=\"2\">N</em> <strong i=\"3\">S <a i=\"4\">D</a></strong> <s i=\"5\">L</s> <a i=\"6\">A</a>",
            |out, text| out.push_str(text),
        );
        assert_eq!(
            restored.as_deref(),
            Some("`npm i` *N* **S [D](https://x.dev \"Docs\")** ~~L~~ ![A](logo.png)")
        );
    }

    #[test]
    fn urls_html_and_footnote_references_are_kept() {
        let unit = the_unit(
            "See <https://a.dev>, https://b.dev/x_(y). or<br>[^1] (https://c.dev/p).\n\n[^1]: Note.",
        );
        assert_eq!(
            unit.text,
            "See <code i=\"1\">https://a.dev</code>, <code i=\"2\">https://b.dev/x_(y)</code>. \
             or<x i=\"3\"/><x i=\"4\"/> (<code i=\"5\">https://c.dev/p</code>)."
        );
        assert_eq!(
            unit.tags.unwrap().plain(),
            "See <https://a.dev>, https://b.dev/x_(y). or<br>[^1] (https://c.dev/p)."
        );
    }

    #[test]
    fn escapes_and_entities_stay_as_written() {
        let unit = the_unit("> Not \\*emphasis\\* &amp; 5 \\< 6\n> \\> still\n");
        assert_eq!(unit.text, "Not \\*emphasis\\* &amp; 5 \\< 6\n\\> still");
        assert!(unit.tags.is_none());
    }

    #[test]
    fn a_hard_break_is_kept_with_the_prefix_of_its_next_line() {
        let unit = the_unit("> one  \n> two\\\n> three\n");
        assert_eq!(unit.text, "one<br i=\"1\"/>two<br i=\"2\"/>three");
        let tags = unit.tags.unwrap();
        assert_eq!(tags.plain(), "one\ntwo\nthree");
        let restored = tags.restore("1<br i=\"1\"/>2<br i=\"2\"/>3", |out, text| {
            out.push_str(text)
        });
        assert_eq!(restored.as_deref(), Some("1  \n> 2\\\n> 3"));
    }

    #[test]
    fn emphasis_with_underscores_comes_back_with_stars() {
        let tags = the_unit("An _emphasis_ and __strong__.").tags.unwrap();
        let restored = tags.restore(
            "<em i=\"1\">強調</em>と<strong i=\"2\">太字</strong>。",
            |out, text| out.push_str(text),
        );
        assert_eq!(restored.as_deref(), Some("*強調*と**太字**。"));
    }

    #[test]
    fn a_link_named_by_its_label_keeps_the_label() {
        let text = "See [the guide] and [Home][] or [this][home].\n\n[the guide]: https://g\n[home]: https://h\n";
        let unit = the_unit(text);
        assert_eq!(
            unit.text,
            "See <a i=\"1\">the guide</a> and <a i=\"2\">Home</a> or <a i=\"3\">this</a>."
        );
        let restored = unit.tags.unwrap().restore(
            "<a i=\"1\">案内</a>、<a i=\"2\">ホーム</a>、<a i=\"3\">これ</a>",
            |out, text| out.push_str(text),
        );
        assert_eq!(
            restored.as_deref(),
            Some("[案内][the guide]、[ホーム][Home]、[これ][home]")
        );
    }

    #[test]
    fn an_empty_link_is_kept_whole() {
        let unit = the_unit("Back [](#top) up.");
        assert_eq!(unit.text, "Back <x i=\"1\"/> up.");
    }

    #[test]
    fn a_block_without_words_is_kept() {
        assert_eq!(
            show(
                "| 1 | 2 |\n|---|---|\n\n`code` only\n\nhttps://example.com\n\n![](x.png)\n\n---\n"
            ),
            [
                "=| 1 | 2 |\n|---|---|\n\n",
                "[<code i=\"1\">code</code> only]",
                "=\n\nhttps://example.com\n\n![](x.png)\n\n---\n",
            ]
        );
    }

    #[test]
    fn alert_markers_and_heading_attributes_are_not_text() {
        assert_eq!(
            show("> [!NOTE]\n> Be careful.\n\n## Setup {#setup}\n"),
            [
                "=> [!NOTE]\n> ",
                "[Be careful.] keep \"> \"",
                "=\n\n## ",
                "[Setup] join",
                "= {#setup}\n",
            ]
        );
    }

    #[test]
    fn find_url_stops_where_the_url_does() {
        fn found(text: &str) -> Option<&str> {
            find_url(text).map(|range| &text[range])
        }
        assert_eq!(found("go to https://a.dev/x."), Some("https://a.dev/x"));
        assert_eq!(found("(http://a.dev/b_(c))"), Some("http://a.dev/b_(c)"));
        assert_eq!(found("詳細はhttps://a.dev/を参照"), Some("https://a.dev/"));
        assert_eq!(found("[https://a.dev]"), Some("https://a.dev"));
        assert_eq!(found("xhttps://a.dev http"), None);
        assert_eq!(found("https:// alone"), None);
    }
}
