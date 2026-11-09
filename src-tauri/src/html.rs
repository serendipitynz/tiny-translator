//! HTML の翻訳単位 (doc-1 §7.1).
//!
//! The original is read with html5gum, a WHATWG tokenizer whose tokens carry their place in the
//! original, and the translation is the original with each 翻訳単位's place taken by its
//! translation. Tags, attributes, comments, `script`, `style`, `pre` and the rest that is not
//! translated are copied rather than written again, so they come out exactly as they went in.
//!
//! A 翻訳単位 is a run of text and inline elements between two block boundaries: the content of a
//! `p`, an `li`, a `td`, or the text a `div` holds before its first `p`. Its text is cut from the
//! original as written (character references stay), with the inline elements as 番号付きのタグ
//! whose markup, attributes included, is the original's. The `alt` and `title` values are
//! translated as well: on their own where their tag stands outside a 翻訳単位, and before the
//! 翻訳単位 where it stands inside one (see [`retag`]).
//!
//! There is no tree builder: a run ends at any tag that is not an inline element, which is where
//! the HTML parser's implied end tags (`<p>` without `</p>`, `<li>` after `<li>`) end a block too.

use std::borrow::Cow;
use std::ops::Range;

use html5gum::{DefaultEmitter, Token, Tokenizer};

use crate::document::{Lines, Segment, Unit};
use crate::tags::{Builder, Tagged};

/// `text` as the 翻訳単位 of its blocks and attribute values, and the original between them.
pub fn segments(text: &str) -> Vec<Segment<'_>> {
    let items = items(text);
    let mut out = Out {
        text,
        segments: Vec::new(),
        kept_from: 0,
    };
    let mut i = 0;
    while i < items.len() {
        if is_inline(&items[i]) {
            let start = i;
            while i < items.len() && is_inline(&items[i]) {
                i = past(&items, i);
            }
            out.run(&items[start..i]);
            continue;
        }
        if let Item::Start { values, .. } = &items[i] {
            for value in values {
                out.attribute(value.clone());
            }
        }
        i = past(&items, i);
    }
    out.segments.push(Segment::Kept(&text[out.kept_from..]));
    out.segments
}

/// The text with tags and the tags of the 翻訳単位 written `original`, with its attribute values
/// (the 翻訳単位's [`Unit::attributes`], in order) put in the markup as `values` translates
/// them; a `None` keeps the original value.
pub fn retag(original: &str, values: &[Option<String>]) -> (String, Option<Tagged>) {
    let built = build(original, values);
    (built.text, built.tags)
}

/// Adds `c` of a translated attribute value to `out`, inside `quote`: whitespace becomes a space,
/// which an attribute value reads the same, and the quote a character reference.
pub fn escape_attribute(out: &mut String, c: char, quote: char) {
    match c {
        '\n' | '\r' | '\t' => out.push(' '),
        '"' if quote == '"' => out.push_str("&quot;"),
        '\'' if quote == '\'' => out.push_str("&#39;"),
        c => out.push(c),
    }
}

/// One token of the original, and its place.
#[derive(Debug)]
enum Item {
    Text {
        range: Range<usize>,
        /// Whether the text, its character references undone, has a letter in it.
        words: bool,
    },
    Start {
        name: String,
        range: Range<usize>,
        /// Whether an SVG or MathML element ends itself (`<svg/>`). A browser ignores `/>`
        /// elsewhere, and so does this.
        self_closing: bool,
        /// Its `alt` and `title` values that have a letter in them, in the order written.
        values: Vec<Value>,
    },
    End {
        name: String,
        range: Range<usize>,
    },
    /// A comment, or a doctype.
    Other {
        range: Range<usize>,
    },
}

impl Item {
    fn range(&self) -> &Range<usize> {
        match self {
            Item::Text { range, .. }
            | Item::Start { range, .. }
            | Item::End { range, .. }
            | Item::Other { range } => range,
        }
    }
}

/// An attribute value to translate.
#[derive(Debug, Clone)]
struct Value {
    /// The value as written, without its quotes.
    range: Range<usize>,
    quote: Option<char>,
}

/// The tokens of `text`. The tokenizer switches to raw text after `script`, `style`, `textarea`
/// and the like, as a browser does, so their content is one text token whatever it holds.
fn items(text: &str) -> Vec<Item> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut items = Vec::new();
    for token in Tokenizer::new_with_emitter(text, emitter) {
        let Ok(token) = token;
        let item = match token {
            Token::StartTag(tag) => {
                let mut values: Vec<Value> = tag
                    .attributes
                    .iter()
                    .filter(|(name, value)| {
                        matches!(name.as_slice(), b"alt" | b"title") && has_letter(value)
                    })
                    .filter_map(|(_, value)| value_in(text, value.span.start..value.span.end))
                    .collect();
                values.sort_by_key(|value| value.range.start);
                let name = String::from_utf8_lossy(&tag.name).into_owned();
                // A browser reads `/>` only in SVG and MathML: `<span/>` opens a span, and
                // `<script src=a.js />` a script that runs to its end tag.
                let self_closing = tag.self_closing && matches!(name.as_str(), "svg" | "math");
                Item::Start {
                    name,
                    range: tag.span.start..tag.span.end,
                    self_closing,
                    values,
                }
            }
            Token::EndTag(tag) => Item::End {
                name: String::from_utf8_lossy(&tag.name).into_owned(),
                range: tag.span.start..tag.span.end,
            },
            Token::String(string) => Item::Text {
                range: string.span.start..string.span.end,
                words: has_letter(&string.value),
            },
            Token::Comment(comment) => Item::Other {
                range: comment.span.start..comment.span.end,
            },
            Token::Doctype(doctype) => Item::Other {
                range: doctype.span.start..doctype.span.end,
            },
            Token::Error(_) => continue,
        };
        items.push(item);
    }
    items
}

fn has_letter(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes)
        .chars()
        .any(char::is_alphabetic)
}

/// The value of the attribute written at `range` (`alt="…"`, `alt='…'`, `alt=…`).
fn value_in(text: &str, range: Range<usize>) -> Option<Value> {
    let written = text.get(range.clone())?;
    let after = written.find('=')? + 1;
    let value = written[after..].trim_start_matches(|c: char| c.is_ascii_whitespace());
    let start = range.start + written.len() - value.len();
    match value.chars().next()? {
        quote @ ('"' | '\'') => {
            let len = value[1..].find(quote)?;
            Some(Value {
                range: start + 1..start + 1 + len,
                quote: Some(quote),
            })
        }
        _ => {
            let len = value
                .find(|c: char| c.is_ascii_whitespace() || c == '>')
                .unwrap_or(value.len());
            Some(Value {
                range: start..start + len,
                quote: None,
            })
        }
    }
}

/// How an element stands in a 翻訳単位.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Its text is translated with the text around it: a tag pair, under this name.
    Paired(&'static str),
    /// Its text is not translated: a kept element, under this name.
    Kept(&'static str),
    /// It has no text: one void tag, under this name.
    Void(&'static str),
    /// Its content is not text to translate (`script`, `svg`, a video's fallback): one void tag
    /// for the element and everything in it.
    Opaque,
    /// It is not inline: a block boundary.
    Block,
    /// A block whose content is kept as it is, to its end tag.
    Verbatim,
}

/// The phrasing elements that hold text (HTML Living Standard §3.2.5.2.5), and the obsolete ones
/// still met (`font`, `tt`). An element not listed here or below is a block, unless its name is a
/// custom element's (it has a `-`), which a browser shows inline.
fn role(name: &str) -> Role {
    const PAIRED: &[&str] = &[
        "a", "abbr", "b", "bdi", "bdo", "big", "button", "cite", "data", "del", "dfn", "em",
        "font", "i", "ins", "kbd", "label", "mark", "nobr", "output", "q", "rb", "rp", "rt",
        "ruby", "s", "small", "span", "strike", "strong", "sub", "sup", "time", "tt", "u", "var",
    ];
    if let Some(name) = PAIRED.iter().find(|paired| **paired == name) {
        return Role::Paired(name);
    }
    match name {
        "code" => Role::Kept("code"),
        // A program's output, which the reader matches against what the program prints. `kbd`
        // is translated instead: a key or a button's label (`保存`) follows the reader's language.
        "samp" => Role::Kept("samp"),
        "br" | "wbr" => Role::Void("br"),
        "img" => Role::Void("img"),
        "input" | "embed" => Role::Void("x"),
        "script" | "style" | "textarea" | "svg" | "math" | "iframe" | "video" | "audio"
        | "canvas" | "object" | "picture" | "map" | "noscript" | "noembed" | "noframes"
        | "select" | "template" | "xmp" => Role::Opaque,
        "pre" | "listing" | "plaintext" => Role::Verbatim,
        _ if name.contains('-') => Role::Paired("span"),
        _ => Role::Block,
    }
}

fn is_inline(item: &Item) -> bool {
    match item {
        Item::Text { .. } => true,
        Item::Start { name, .. } | Item::End { name, .. } => {
            !matches!(role(name), Role::Block | Role::Verbatim)
        }
        Item::Other { .. } => true,
    }
}

/// The index after the item at `i` and, for an element whose content is not read, after its end.
fn past(items: &[Item], i: usize) -> usize {
    match &items[i] {
        Item::Start {
            name,
            self_closing: false,
            ..
        } if matches!(role(name), Role::Opaque | Role::Verbatim) => {
            end_of(items, i).map_or(items.len(), |end| end + 1)
        }
        _ => i + 1,
    }
}

/// The index of the end tag that closes the element starting at `items[start]`, counting the
/// elements of the same name inside it.
fn end_of(items: &[Item], start: usize) -> Option<usize> {
    let Item::Start { name: own, .. } = &items[start] else {
        return None;
    };
    let mut depth = 0;
    for (k, item) in items.iter().enumerate().skip(start) {
        match item {
            Item::Start {
                name,
                self_closing: false,
                ..
            } if name == own => depth += 1,
            Item::End { name, .. } if name == own => {
                depth -= 1;
                if depth == 0 {
                    return Some(k);
                }
            }
            _ => {}
        }
    }
    None
}

/// The segments made so far.
struct Out<'a> {
    text: &'a str,
    segments: Vec<Segment<'a>>,
    /// Where the original not yet in `segments` starts.
    kept_from: usize,
}

impl<'a> Out<'a> {
    /// A run of inline items: one 翻訳単位 from its first to its last text or tag, or, when it has
    /// nothing to translate, the attribute values in its tags.
    fn run(&mut self, items: &[Item]) {
        let content = |item: &&Item| match item {
            Item::Text { range, .. } => !self.text[range.clone()].trim().is_empty(),
            Item::Other { .. } => false,
            _ => true,
        };
        let (Some(first), Some(last)) = (items.iter().find(content), items.iter().rfind(content))
        else {
            return;
        };
        let start = first.range().start + leading_whitespace(&self.text[first.range().clone()]);
        let end = last.range().end - trailing_whitespace(&self.text[last.range().clone()]);
        let original = &self.text[start..end];
        let built = build(original, &[]);
        if built.words {
            let attributes = built
                .values
                .iter()
                .map(|value| &original[value.range.clone()])
                .collect();
            self.segments
                .push(Segment::Kept(&self.text[self.kept_from..start]));
            self.segments.push(Segment::Unit(Unit {
                text: Cow::Owned(built.text),
                original,
                tags: built.tags,
                lines: Lines::Keep(built.indent),
                attributes,
            }));
            self.kept_from = end;
        } else {
            for value in built.values {
                let range = start + value.range.start..start + value.range.end;
                self.attribute(Value { range, ..value });
            }
        }
    }

    /// An attribute value as a 翻訳単位 of its own. One written without quotes gets them, since
    /// its translation may hold a space.
    fn attribute(&mut self, value: Value) {
        let range = value.range;
        self.segments
            .push(Segment::Kept(&self.text[self.kept_from..range.start]));
        if value.quote.is_none() {
            self.segments.push(Segment::Kept("\""));
        }
        let original = &self.text[range.clone()];
        self.segments.push(Segment::Unit(Unit {
            text: Cow::Borrowed(original),
            original,
            tags: None,
            lines: Lines::Attribute(value.quote.unwrap_or('"')),
            attributes: Vec::new(),
        }));
        if value.quote.is_none() {
            self.segments.push(Segment::Kept("\""));
        }
        self.kept_from = range.end;
    }
}

fn leading_whitespace(text: &str) -> usize {
    text.len() - text.trim_start().len()
}

fn trailing_whitespace(text: &str) -> usize {
    text.len() - text.trim_end().len()
}

/// A 翻訳単位 made from its original.
struct Built {
    text: String,
    tags: Option<Tagged>,
    /// Whether its text, outside kept and opaque elements, has a letter in it.
    words: bool,
    /// The attribute values to translate in its tags, in `original`.
    values: Vec<Value>,
    /// What a line inside it starts with after a line break.
    indent: String,
}

/// The 翻訳単位 written `original`, a run of inline items, with the attribute values it holds
/// replaced in their markup as `values` says.
fn build(original: &str, values: &[Option<String>]) -> Built {
    let items = items(original);
    let pairs = pairs(&items);
    let mut b = Builder::default();
    let mut words = false;
    let mut found = Vec::new();
    let mut cursor = 0;
    let mut i = 0;
    while i < items.len() {
        let item = &items[i];
        let range = item.range().clone();
        if cursor < range.start {
            b.text(&lines(&original[cursor..range.start]));
        }
        cursor = range.end;
        match item {
            Item::Text { words: w, .. } => {
                words |= w;
                b.text(&lines(&original[range]));
            }
            Item::Other { .. } => b.markup("x", &original[range]),
            Item::End { .. } if pairs[i].is_some() => b.close(original[range].to_owned()),
            Item::End { .. } => b.markup("x", &original[range]),
            Item::Start {
                name,
                self_closing,
                values: own,
                ..
            } => {
                let role = role(name);
                let end = (!self_closing).then(|| end_of(&items, i)).flatten();
                match (role, end) {
                    (Role::Opaque, Some(end)) => {
                        let to = items[end].range().end;
                        b.markup("x", &original[range.start..to]);
                        cursor = to;
                        i = end;
                    }
                    (Role::Opaque, None) if *self_closing => b.markup("x", &original[range]),
                    // Without its end tag, the element runs to the end of the text.
                    (Role::Opaque, None) => {
                        b.markup("x", &original[range.start..]);
                        cursor = original.len();
                        i = items.len();
                    }
                    (Role::Kept(name), Some(end)) => {
                        let to = items[end].range().end;
                        let shown: String = items[i + 1..end]
                            .iter()
                            .filter(|item| matches!(item, Item::Text { .. }))
                            .map(|item| &original[item.range().clone()])
                            .collect();
                        b.kept(name, &original[range.start..to], &lines(&shown));
                        cursor = to;
                        i = end;
                    }
                    (Role::Paired(name), _) if pairs[i].is_some() => {
                        b.open(name, markup(original, range, own, values, &mut found));
                    }
                    (Role::Void(name), _) => {
                        b.markup(name, &markup(original, range, own, values, &mut found));
                    }
                    _ => b.markup("x", &markup(original, range, own, values, &mut found)),
                }
            }
        }
        i += 1;
    }
    if cursor < original.len() {
        b.text(&lines(&original[cursor..]));
    }
    let (text, tags) = b.finish();
    Built {
        text,
        tags,
        words,
        values: found,
        indent: indent(original),
    }
}

/// For each paired element's start and end tag in `items`, the index of the other. A start tag
/// without an end tag in the run (`<p>Read <b>this</p>`), or an end tag without a start tag,
/// has none, and goes to the engine as a void tag; so does a start tag whose end tag closes an
/// element opened before it (`<b><i></b>`), which a pair would cross.
fn pairs(items: &[Item]) -> Vec<Option<usize>> {
    let mut pairs = vec![None; items.len()];
    let mut open: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < items.len() {
        match &items[i] {
            Item::Start {
                name, self_closing, ..
            } => match role(name) {
                Role::Paired(_) if !self_closing => open.push(i),
                Role::Opaque | Role::Kept(_) if !self_closing => {
                    i = end_of(items, i).unwrap_or(i);
                }
                _ => {}
            },
            Item::End { name, .. } => {
                let start = open.iter().rposition(
                    |&start| matches!(&items[start], Item::Start { name: own, .. } if own == name),
                );
                if let Some(at) = start {
                    pairs[open[at]] = Some(i);
                    pairs[i] = Some(open[at]);
                    open.truncate(at);
                }
            }
            _ => {}
        }
        i += 1;
    }
    pairs
}

/// The start tag at `range` as written, with each of its attribute values (`own`) put in as
/// `values` translates it. `found` collects the values, whose place in it is their number.
fn markup(
    original: &str,
    range: Range<usize>,
    own: &[Value],
    values: &[Option<String>],
    found: &mut Vec<Value>,
) -> String {
    let mut out = String::new();
    let mut at = range.start;
    for value in own {
        let translation = values.get(found.len()).and_then(Option::as_ref);
        found.push(value.clone());
        let Some(translation) = translation else {
            continue;
        };
        out.push_str(&original[at..value.range.start]);
        let quote = value.quote.unwrap_or('"');
        if value.quote.is_none() {
            out.push(quote);
        }
        for c in translation.chars() {
            escape_attribute(&mut out, c, quote);
        }
        if value.quote.is_none() {
            out.push(quote);
        }
        at = value.range.end;
    }
    out.push_str(&original[at..range.end]);
    out
}

/// Text of the original as written, with each run of whitespace that breaks the line as one
/// line break: HTML reads it as a space, and the indent after it is the markup's, not the text's.
fn lines(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find(|c: char| c.is_ascii_whitespace()) {
        out.push_str(&rest[..at]);
        let space = &rest[at..];
        let len = space.len()
            - space
                .trim_start_matches(|c: char| c.is_ascii_whitespace())
                .len();
        let space = &space[..len];
        if space.contains('\n') {
            out.push('\n');
        } else {
            out.push_str(space);
        }
        rest = &rest[at + len..];
    }
    out.push_str(rest);
    out
}

/// The indent of the line after the first line break in `original`: what the translation's line
/// breaks are followed by, so that its lines line up as the original's did.
fn indent(original: &str) -> String {
    original
        .split_once('\n')
        .map(|(_, after)| {
            let len = after.len() - after.trim_start_matches([' ', '\t']).len();
            after[..len].to_owned()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `segments` makes of `text`: `[…]` around each 翻訳単位 as the engine is given it, and
    /// the original between them.
    fn shape(text: &str) -> String {
        segments(text)
            .iter()
            .map(|segment| match segment {
                Segment::Kept(text) => (*text).to_owned(),
                Segment::Unit(unit) => format!("[{}]", unit.text),
            })
            .collect()
    }

    fn units(text: &str) -> Vec<String> {
        segments(text)
            .into_iter()
            .filter_map(|segment| match segment {
                Segment::Unit(unit) => Some(unit.text.into_owned()),
                Segment::Kept(_) => None,
            })
            .collect()
    }

    #[test]
    fn blocks_are_units_and_the_markup_between_them_is_kept() {
        let text = "<!DOCTYPE html>\n<html><head><title>My page</title></head>\n<body>\n  <h1 id=top>Hello</h1>\n  <p>One</p><p>Two\n</body></html>\n";
        assert_eq!(
            shape(text),
            "<!DOCTYPE html>\n<html><head><title>[My page]</title></head>\n<body>\n  <h1 id=top>[Hello]</h1>\n  <p>[One]</p><p>[Two]\n</body></html>\n"
        );
    }

    #[test]
    fn inline_elements_go_as_numbered_tags() {
        assert_eq!(
            units(r#"<p>Read <a href="/doc" class="x">the <b>docs</b></a> first.</p>"#),
            [r#"Read <a i="1">the <b i="2">docs</b></a> first."#]
        );
        assert_eq!(
            units("<p>Hello <my-name>Tom</my-name>.</p>"),
            [r#"Hello <span i="1">Tom</span>."#]
        );
    }

    #[test]
    fn code_and_samp_are_kept_and_shown_with_their_text() {
        assert_eq!(
            units("<p>Run <code class=sh>npm <b>i</b></code>, see <samp>Done</samp>.</p>"),
            [r#"Run <code i="1">npm i</code>, see <samp i="2">Done</samp>."#]
        );
        assert_eq!(
            shape("<p><code>npm i</code></p>"),
            "<p><code>npm i</code></p>"
        );
    }

    #[test]
    fn script_style_pre_and_their_like_are_not_translated() {
        let text = "<script>let a = '<p>Hi</p>';</script>\n<style>p > a { color: red }</style>\n<pre>Some <b>code</b>\n  here</pre>\n<textarea>Draft</textarea><svg><text>Label</text></svg>";
        assert_eq!(shape(text), text);
    }

    #[test]
    fn a_script_or_an_image_inside_a_sentence_is_a_void_tag() {
        assert_eq!(
            units(r#"<p>Press <img src="k.png"> then <script>go("<b>")</script>wait.<br>Done</p>"#),
            [r#"Press <img i="1"/> then <x i="2"/>wait.<br i="3"/>Done"#]
        );
    }

    #[test]
    fn whitespace_around_a_unit_is_kept_and_line_breaks_in_it_are_one() {
        let text = "<li>\n    Hello  world,\n    and more.\n</li>";
        let segments = segments(text);
        assert_eq!(segments[0], Segment::Kept("<li>\n    "));
        let Segment::Unit(unit) = &segments[1] else {
            panic!("a unit")
        };
        assert_eq!(unit.text, "Hello  world,\nand more.");
        assert_eq!(unit.original, "Hello  world,\n    and more.");
        assert_eq!(unit.lines, Lines::Keep("    ".into()));
        assert_eq!(segments[2], Segment::Kept("\n</li>"));
    }

    #[test]
    fn character_references_stay_as_written_and_do_not_count_as_words() {
        assert_eq!(
            shape("<p>Tom &amp; Jerry</p><td>&nbsp;</td><td>&#42;</td><td>&copy; 2026</td>"),
            "<p>[Tom &amp; Jerry]</p><td>&nbsp;</td><td>&#42;</td><td>&copy; 2026</td>"
        );
    }

    #[test]
    fn comments_at_the_edges_stay_outside_the_unit() {
        assert_eq!(
            shape("<p><!-- a --> Hello <!-- b --> world <!-- c --></p>"),
            r#"<p><!-- a --> [Hello <x i="1"/> world] <!-- c --></p>"#
        );
    }

    #[test]
    fn unmatched_and_crossing_tags_go_as_void_tags() {
        assert_eq!(
            units("<p>Read <b>this</p><p>and</i> that</p>"),
            [r#"Read <x i="1"/>this"#, r#"and<x i="1"/> that"#]
        );
        assert_eq!(
            units("<p><b>bold <i>both</b> italic</i></p>"),
            [r#"<b i="1">bold <x i="2"/>both</b> italic<x i="3"/>"#]
        );
    }

    #[test]
    fn alt_and_title_outside_a_unit_are_units_of_their_own() {
        assert_eq!(
            shape(
                r#"<div title="Notes"><img src=a.png alt='A logo'><img alt=Badge src=b.png></div>"#
            ),
            r#"<div title="[Notes]"><img src=a.png alt='[A logo]'><img alt="[Badge]" src=b.png></div>"#
        );
        assert_eq!(
            shape(r#"<img alt="42" title="">"#),
            r#"<img alt="42" title="">"#
        );
    }

    #[test]
    fn alt_and_title_inside_a_unit_go_with_it_and_its_tags_are_made_again() {
        let text = r#"<p>See <img src="a.png" alt="the logo"> and <abbr title='Hypertext "Markup"'>HTML</abbr>.</p>"#;
        let segments = segments(text);
        let Segment::Unit(unit) = &segments[1] else {
            panic!("a unit")
        };
        assert_eq!(unit.attributes, ["the logo", r#"Hypertext "Markup""#]);
        let (retagged, tags) = retag(
            unit.original,
            &[
                Some("ロゴ".into()),
                Some("ハイパーテキスト 'マークアップ'".into()),
            ],
        );
        assert_eq!(retagged, unit.text);
        let restored = tags.unwrap().restore(
            r#"<img i="1"/> と <abbr i="2">HTML</abbr> を見る。"#,
            |out, text| out.push_str(text),
        );
        assert_eq!(
            restored.as_deref(),
            Some(
                r#"<img src="a.png" alt="ロゴ"> と <abbr title='ハイパーテキスト &#39;マークアップ&#39;'>HTML</abbr> を見る。"#
            )
        );
    }

    #[test]
    fn a_kept_value_stays_as_written_when_retagged() {
        let (_, tags) = retag(r#"Go <a href=x title="Home">home</a>"#, &[None]);
        let restored = tags
            .unwrap()
            .restore(r#"<a i="1">ホーム</a>へ"#, |out, text| {
                out.push_str(text)
            });
        assert_eq!(
            restored.as_deref(),
            Some(r#"<a href=x title="Home">ホーム</a>へ"#)
        );
    }

    #[test]
    fn values_in_kept_and_opaque_elements_are_not_translated() {
        assert_eq!(
            shape(r#"<code title="Copy">x</code><svg><title>Logo</title><a title="Go"/></svg>"#),
            r#"<code title="Copy">x</code><svg><title>Logo</title><a title="Go"/></svg>"#
        );
    }

    #[test]
    fn unquoted_values_end_at_whitespace_or_the_tag() {
        let text = "<img alt=Logo/><img title=Big\nsrc=x>";
        assert_eq!(
            shape(text),
            "<img alt=\"[Logo/]\"><img title=\"[Big]\"\nsrc=x>"
        );
    }

    #[test]
    fn attribute_values_are_escaped_for_their_quotes() {
        let mut out = String::new();
        for c in "a\"b'c\nd".chars() {
            escape_attribute(&mut out, c, '"');
        }
        assert_eq!(out, "a&quot;b'c d");
    }

    #[test]
    fn only_svg_and_math_end_themselves() {
        assert_eq!(
            units(
                r#"<p>A <svg/> icon, a <span/>name</span> and <script src=a.js />x</script> here.</p>"#
            ),
            [r#"A <x i="1"/> icon, a <span i="2">name</span> and <x i="3"/> here."#]
        );
    }

    #[test]
    fn a_fragment_of_text_and_inline_elements_is_one_unit() {
        assert_eq!(
            shape("Hello <em>world</em>!\n"),
            "[Hello <em i=\"1\">world</em>!]\n"
        );
    }

    #[test]
    fn crlf_line_breaks_stay_in_the_markup() {
        assert_eq!(
            shape("<ul>\r\n  <li>One</li>\r\n  <li>Two</li>\r\n</ul>\r\n"),
            "<ul>\r\n  <li>[One]</li>\r\n  <li>[Two]</li>\r\n</ul>\r\n"
        );
    }
}
