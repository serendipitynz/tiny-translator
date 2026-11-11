//! The 翻訳単位 of an i18n resource's string values (doc-1 §7.1): JSON and ARB (`json.rs`), Apple
//! .strings (`strings.rs`).
//!
//! A value goes to the engine with its escapes undone, and its translation is escaped again as it is
//! written back, by the format's [`Lines`]. Its placeholders go as kept elements and come back as
//! written, and the messages of an ICU argument that chooses one are 翻訳単位 of their own
//! (`placeholders.rs`).

use std::borrow::Cow;
use std::ops::Range;

use crate::document::{Lines, Segment, Unit};
use crate::placeholders::{self, Inline};
use crate::tags::Builder;

/// A string value's content with its escapes undone, and where each of its characters is written.
pub struct Value<'a> {
    text: &'a str,
    decoded: String,
    /// Where each character of `decoded` starts in it, and its end.
    starts: Vec<usize>,
    /// Where each character is written in `text`.
    written: Vec<Range<usize>>,
    /// Where the content ends in `text`.
    end: usize,
}

impl<'a> Value<'a> {
    /// The content at `range` of `text`, whose characters are `chars`: each with where it is
    /// written, relative to `range`.
    pub fn new(text: &'a str, range: Range<usize>, chars: Vec<(Range<usize>, char)>) -> Self {
        let mut decoded = String::new();
        let mut starts = Vec::with_capacity(chars.len() + 1);
        let mut written = Vec::with_capacity(chars.len());
        for (at, c) in chars {
            starts.push(decoded.len());
            decoded.push(c);
            written.push(range.start + at.start..range.start + at.end);
        }
        starts.push(decoded.len());
        Value {
            text,
            decoded,
            starts,
            written,
            end: range.end,
        }
    }

    /// Where `text` writes the characters at `decoded`.
    fn raw(&self, decoded: Range<usize>) -> Range<usize> {
        self.at(decoded.start)..self.at(decoded.end)
    }

    fn at(&self, decoded: usize) -> usize {
        let i = self
            .starts
            .binary_search(&decoded)
            .expect("a range of the decoded value lies on its characters");
        self.written
            .get(i)
            .map_or(self.end, |written| written.start)
    }

    /// The characters at `decoded` as `text` writes them.
    fn original(&self, decoded: Range<usize>) -> &'a str {
        &self.text[self.raw(decoded)]
    }
}

/// The segments of a resource, made as its values are found in order.
pub struct Units<'a> {
    text: &'a str,
    segments: Vec<Segment<'a>>,
    /// Where the original not yet in `segments` starts.
    kept_from: usize,
}

impl<'a> Units<'a> {
    pub fn new(text: &'a str) -> Self {
        Units {
            text,
            segments: Vec::new(),
            kept_from: 0,
        }
    }

    /// Takes `value` as 翻訳単位: the whole value, or where an ICU argument chooses a message, each
    /// of its messages and the text around it. The placeholders go to the engine as kept elements,
    /// so the translation holds them as the original writes them. Its translation is written as
    /// `lines` says: escaped as the value's quotes need.
    pub fn value(&mut self, value: &Value<'a>, lines: &Lines) {
        for message in placeholders::messages(&value.decoded) {
            self.message(value, &message, lines);
        }
    }

    /// Takes one message of `value` as a 翻訳単位, leaving out the whitespace around it, which an
    /// i18n message may need (`"Name: "`) and the engine would drop. A message without letters
    /// outside its placeholders (`""`, `"42"`, `"—"`, `"%s"`) is not sent, as a Markdown block
    /// without them is not.
    fn message(&mut self, value: &Value<'a>, message: &[Inline], lines: &Lines) {
        let (Some(first), Some(last)) = (message.first(), message.last()) else {
            return;
        };
        let span = &value.decoded[inline_range(first).start..inline_range(last).end];
        let trimmed = span.trim_start();
        let start = inline_range(first).start + span.len() - trimmed.len();
        let end = start + trimmed.trim_end().len();
        let mut b = Builder::default();
        for inline in message {
            match inline {
                Inline::Text(range) => {
                    let range = range.start.max(start)..range.end.min(end);
                    if !range.is_empty() {
                        b.text(&value.decoded[range]);
                    }
                }
                Inline::Placeholder(range) => {
                    let shown = &value.decoded[range.clone()];
                    b.placeholder("code", value.original(range.clone()), shown);
                }
            }
        }
        if !b.has_words() {
            return;
        }
        let (text, tags) = b.finish();
        let raw = value.raw(start..end);
        let original = &self.text[raw.clone()];
        let text = if tags.is_none() && !original.contains('\\') {
            Cow::Borrowed(original)
        } else {
            Cow::Owned(text)
        };
        self.segments
            .push(Segment::Kept(&self.text[self.kept_from..raw.start]));
        self.segments.push(Segment::Unit(Unit {
            text,
            original,
            tags,
            lines: lines.clone(),
            attributes: Vec::new(),
        }));
        self.kept_from = raw.end;
    }

    /// The segments, ending with the original after the last 翻訳単位.
    pub fn finish(mut self) -> Vec<Segment<'a>> {
        self.segments
            .push(Segment::Kept(&self.text[self.kept_from..]));
        self.segments
    }
}

fn inline_range(inline: &Inline) -> &Range<usize> {
    match inline {
        Inline::Text(range) | Inline::Placeholder(range) => range,
    }
}
