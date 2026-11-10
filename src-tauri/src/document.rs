//! Translating a text 翻訳単位 by 翻訳単位 (doc-1 §7.1, §7.3).
//!
//! The format cuts the text into 翻訳単位 and the original between them: the whitespace between
//! paragraphs for プレーンテキスト, the markup for Markdown (`markdown.rs`) and HTML (`html.rs`),
//! everything but the string values for JSON (`json.rs`). Each piece goes to the
//! engine as a prompt of its own, carrying the piece before it and that piece's translation, and
//! the translation is put back together with the original between the pieces. A translation that
//! starts repeating itself is cut off and made again sentence by sentence (doc-2 §1.1).
//!
//! A 翻訳単位 with inline elements goes with them as 番号付きのタグ (`tags.rs`), and is translated
//! again without them when its translation does not keep them.

use std::borrow::Cow;
use std::ops::ControlFlow;

use crate::engine::CONTEXT_TOKENS;
use crate::html;
use crate::json;
use crate::markdown;
use crate::repetition::Repetition;
use crate::tags::{Hide, Tagged};
use crate::translate::{self, Completion, Outcome};
use crate::units;

/// The engine, as far as translating a text needs it.
pub trait Model {
    /// How many tokens `text` takes in a prompt.
    fn tokens(&mut self, text: &str) -> Result<usize, String>;

    /// Streams the completion of `prompt` to `on_piece`, until `on_piece` answers `false`.
    fn complete(
        &mut self,
        prompt: &str,
        on_piece: &mut dyn FnMut(&str) -> bool,
    ) -> Result<Completion, String>;
}

/// A change to the translation shown on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Update<'a> {
    /// Text to add at the end.
    Append(&'a str),
    /// The whole translation so far, in place of what was sent: the translation of a piece that
    /// started repeating itself is taken back to be made again.
    Replace(&'a str),
}

/// The longest piece sent in one prompt, in tokens. With the history and the prompt's tags, a
/// prompt then takes at most about 3,100 of the context's 8,192 tokens, which leaves room for a
/// translation five times as long as the piece.
const UNIT_TOKENS: usize = CONTEXT_TOKENS / 8;

/// The longest history a prompt carries, its source text and translation together, in tokens.
const HISTORY_TOKENS: usize = CONTEXT_TOKENS / 4;

/// A text as the parts the translation keeps as they are and the 翻訳単位 between them.
#[derive(Debug, PartialEq, Eq)]
pub enum Segment<'a> {
    /// Text of the original that goes into the translation as it is.
    Kept(&'a str),
    Unit(Unit<'a>),
}

/// One 翻訳単位 and how its translation is written back.
#[derive(Debug, PartialEq, Eq)]
pub struct Unit<'a> {
    /// What the engine translates: with tags in place of its inline elements when it has any.
    pub text: Cow<'a, str>,
    /// The 翻訳単位 as the original writes it, kept in place of a translation that changed what
    /// must not be translated.
    pub original: &'a str,
    /// The elements behind the tags in `text`.
    pub tags: Option<Tagged>,
    pub lines: Lines,
    /// HTML: the `alt` and `title` values, as written, on the inline elements of this 翻訳単位.
    /// They are translated before it, and its tags are made again with their translations.
    pub attributes: Vec<&'a str>,
}

/// How the line breaks of a translation are written where its 翻訳単位 stood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lines {
    /// As they are, each followed by `prefix`: the markup that carries a line on inside a Markdown
    /// block quote or list item (`> `, the item's indent), empty elsewhere.
    Keep(String),
    /// As spaces: a Markdown heading is one line.
    Join,
    /// As spaces, with each `|` that is not escaped escaped: a Markdown table cell is one line, and
    /// a bare `|` ends it.
    Cell,
    /// Escaped, as the rest of the translation is, inside a JSON string.
    Json,
    /// As spaces, with the quote escaped: an HTML attribute value inside these quotes.
    Attribute(char),
}

impl Lines {
    /// Adds the translated `text` to `out`.
    fn write(&self, out: &mut String, text: &str) {
        for c in text.chars() {
            match (self, c) {
                (Lines::Keep(prefix), '\n') => {
                    out.push('\n');
                    out.push_str(prefix);
                }
                (Lines::Join | Lines::Cell, '\n') => out.push(' '),
                (Lines::Cell, '|') if !out.ends_with('\\') => out.push_str("\\|"),
                (Lines::Json, c) => json::escape(out, c),
                (Lines::Attribute(quote), c) => html::escape_attribute(out, c, *quote),
                _ => out.push(c),
            }
        }
    }
}

/// The format of the original (doc-1 §7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    Text,
    Markdown,
    /// JSON, whose values under the keys named here are not translated.
    Json {
        untranslated_keys: Vec<String>,
    },
    Html,
}

impl Format {
    /// The format the screen names (`src/lib/translate.ts`), with the screen's 翻訳しないキー,
    /// which only JSON reads.
    pub fn from_name(name: &str, untranslated_keys: Vec<String>) -> Option<Format> {
        match name {
            "text" => Some(Format::Text),
            "markdown" => Some(Format::Markdown),
            "json" => Some(Format::Json { untranslated_keys }),
            "html" => Some(Format::Html),
            _ => None,
        }
    }
}

/// Translates `text`, read as `format`, into `target`, sending the translation to `emit` as it is
/// generated. Fails before anything is sent when `text` cannot be read as `format`.
pub fn translate(
    model: &mut impl Model,
    text: &str,
    format: &Format,
    target: &str,
    source: Option<&str>,
    emit: impl FnMut(Update<'_>),
) -> Result<Outcome, String> {
    let segments = match format {
        Format::Text => text_segments(text),
        Format::Markdown => markdown::segments(text),
        Format::Json { untranslated_keys } => json::segments(text, untranslated_keys)?,
        Format::Html => html::segments(text),
    };
    let mut run = Run {
        model,
        target,
        source,
        out: Output::new(emit),
        history: None,
    };
    for segment in &segments {
        match segment {
            Segment::Kept(text) => run.out.raw(text),
            Segment::Unit(unit) => {
                if run.unit(unit)? == Outcome::Stopped {
                    return Ok(Outcome::Stopped);
                }
            }
        }
    }
    Ok(Outcome::Done)
}

/// プレーンテキストの翻訳単位 (doc-1 §7.1): its paragraphs, with the whitespace around them kept.
fn text_segments(text: &str) -> Vec<Segment<'_>> {
    let split = units::paragraphs(text);
    let mut segments = vec![Segment::Kept(split.leading)];
    for (unit, after) in split.pieces {
        segments.push(Segment::Unit(Unit {
            text: Cow::Borrowed(unit),
            original: unit,
            tags: None,
            lines: Lines::Keep(String::new()),
            attributes: Vec::new(),
        }));
        segments.push(Segment::Kept(after));
    }
    segments
}

struct Run<'a, M, F> {
    model: &'a mut M,
    target: &'a str,
    source: Option<&'a str>,
    out: Output<F>,
    /// The piece translated last and its translation, for the next piece's prompt.
    history: Option<(String, String)>,
}

impl<M: Model, F: FnMut(Update<'_>)> Run<'_, M, F> {
    /// Translates one 翻訳単位. One with tags goes in one prompt with them; when its translation
    /// does not keep them, or it is too long for a prompt or repeats itself, it is translated again
    /// with only the tags whose originals must survive (code, URLs), and then without tags, as one
    /// without tags is. A translation without tags that changed one of those originals is not
    /// used: the 翻訳単位 stays as the original writes it.
    fn unit(&mut self, unit: &Unit) -> Result<Outcome, String> {
        self.out.begin_unit(unit.lines.clone());
        let retagged;
        let (text, tags) = if unit.attributes.is_empty() {
            (unit.text.as_ref(), unit.tags.as_ref())
        } else {
            let mut values = Vec::new();
            for value in &unit.attributes {
                match self.aside(value)? {
                    ControlFlow::Break(()) => return Ok(Outcome::Stopped),
                    ControlFlow::Continue(translation) => values.push(translation),
                }
            }
            retagged = html::retag(unit.original, &values);
            (retagged.0.as_str(), retagged.1.as_ref())
        };
        let Some(tags) = tags else {
            return self.untagged(text);
        };
        let mut attempt = Some((text, tags));
        while let Some((text, tagged)) = attempt {
            if self.fits(text, UNIT_TOKENS)?
                && let Some(outcome) = self.tagged(text, tagged)?
            {
                return Ok(outcome);
            }
            // A history with tags the translation just failed to keep would ask for them again.
            self.history = None;
            attempt = tagged.fallback();
        }
        let mark = self.out.mark();
        let start = mark.len;
        let outcome = self.untagged(tags.plain())?;
        if outcome == Outcome::Done && !tags.keeps_originals(&self.out.text[start..]) {
            self.out.replace_since(mark, unit.original);
            self.history = None;
        }
        Ok(outcome)
    }

    /// Translates `text` in one prompt without the history, and without showing it: an attribute
    /// value that goes inside the markup of the 翻訳単位 it stands in. `None` when it is too long
    /// for a prompt or its translation repeats itself or is empty; the value then stays as it is.
    /// The history is left as it was, since the text around the value is what the next piece
    /// goes on from.
    fn aside(&mut self, text: &str) -> Result<ControlFlow<(), Option<String>>, String> {
        if !self.fits(text, UNIT_TOKENS)? {
            return Ok(ControlFlow::Continue(None));
        }
        let prompt = translate::prompt(text, self.target, self.source, None);
        let mut check = Repetition::for_input(text);
        let mut generated = String::new();
        let mut repeats = false;
        let completion = self.model.complete(&prompt, &mut |piece| {
            generated.push_str(piece);
            repeats = piece.chars().any(|c| check.push(c).is_some());
            !repeats
        })?;
        if completion == Completion::Stopped {
            return Ok(ControlFlow::Break(()));
        }
        let translation = generated.trim();
        Ok(ControlFlow::Continue(
            (!repeats && !translation.is_empty()).then(|| translation.to_owned()),
        ))
    }

    /// Translates a 翻訳単位 without tags, cut at sentences first when it is too long for a prompt.
    fn untagged(&mut self, unit: &str) -> Result<Outcome, String> {
        let pieces = if surely_fits(unit, UNIT_TOKENS) {
            vec![(unit, "")]
        } else {
            units::fit(unit, UNIT_TOKENS, |text| self.model.tokens(text))?
        };
        for (piece, after) in pieces {
            if self.piece(piece, true)? == Outcome::Stopped {
                return Ok(Outcome::Stopped);
            }
            self.out.join(after);
        }
        Ok(Outcome::Done)
    }

    /// Translates `text`, which has tags, in one prompt. The screen is shown the translation's
    /// text as it comes, and then the translation with its tags put back as the original markup.
    /// `None` when the tags cannot be put back or the translation repeats itself: the translation
    /// is then taken back.
    fn tagged(&mut self, text: &str, tags: &Tagged) -> Result<Option<Outcome>, String> {
        let mark = self.out.mark();
        let prompt = self.prompt(text)?;
        let mut check = Repetition::for_input(text);
        let mut hide = Hide::default();
        let mut generated = String::new();
        let mut repeats = false;
        let out = &mut self.out;
        let completion = self.model.complete(&prompt, &mut |piece| {
            generated.push_str(piece);
            out.translation(&hide.push(piece));
            repeats = piece.chars().any(|c| check.push(c).is_some());
            !repeats
        })?;
        if completion == Completion::Stopped {
            return Ok(Some(Outcome::Stopped));
        }
        let translation = generated.trim();
        let restored = (!repeats)
            .then(|| tags.restore(translation, |out, text| self.out.lines.write(out, text)))
            .flatten();
        let Some(restored) = restored else {
            self.out.rewind(mark);
            return Ok(None);
        };
        self.out.replace_since(mark, &restored);
        self.history = Some((text.to_owned(), translation.to_owned()));
        Ok(Some(Outcome::Done))
    }

    /// The prompt for `text`, with the history when it fits.
    fn prompt(&mut self, text: &str) -> Result<String, String> {
        let history = self
            .history_that_fits()?
            .then_some(self.history.as_ref())
            .flatten();
        Ok(translate::prompt(
            text,
            self.target,
            self.source,
            history.map(|(earlier, translation)| (earlier.as_str(), translation.as_str())),
        ))
    }

    fn fits(&mut self, text: &str, max: usize) -> Result<bool, String> {
        Ok(surely_fits(text, max) || self.model.tokens(text)? <= max)
    }

    /// Translates `text` in one prompt. When the translation starts repeating itself, it is taken
    /// back and, if `may_retry`, made again sentence by sentence; a sentence that repeats itself
    /// all the same keeps its translation up to the end of the first time round.
    fn piece(&mut self, text: &str, may_retry: bool) -> Result<Outcome, String> {
        let mark = self.out.mark();
        let prompt = self.prompt(text)?;

        let mut check = Repetition::for_input(text);
        let mut generated = String::new();
        let mut found = None;
        let out = &mut self.out;
        let completion = self.model.complete(&prompt, &mut |piece| {
            generated.push_str(piece);
            out.translation(piece);
            found = piece.chars().find_map(|c| check.push(c));
            found.is_none()
        })?;

        match (completion, found) {
            (Completion::Stopped, _) => Ok(Outcome::Stopped),
            (Completion::Cut, Some(found)) => {
                self.out.rewind(mark);
                let mut smaller = units::sentences(text);
                if smaller.pieces.len() < 2 {
                    smaller = units::lines(text);
                }
                if may_retry && smaller.pieces.len() >= 2 {
                    for (sentence, after) in smaller.pieces {
                        if self.piece(sentence, false)? == Outcome::Stopped {
                            return Ok(Outcome::Stopped);
                        }
                        self.out.join(after);
                    }
                } else {
                    let keep = generated
                        .char_indices()
                        .nth(found.keep())
                        .map_or(generated.len(), |(at, _)| at);
                    self.out.translation(&generated[..keep]);
                    self.out.end_translation();
                    // A translation cut short would lead the next one astray.
                    self.history = None;
                }
                Ok(Outcome::Done)
            }
            (Completion::Done | Completion::Cut, _) => {
                self.out.end_translation();
                let translation = generated.trim();
                self.history =
                    (!translation.is_empty()).then(|| (text.to_owned(), translation.to_owned()));
                Ok(Outcome::Done)
            }
        }
    }

    /// Whether the history fits in [`HISTORY_TOKENS`]; when it does not, the prompt goes without.
    fn history_that_fits(&mut self) -> Result<bool, String> {
        let Some((earlier, translation)) = &self.history else {
            return Ok(false);
        };
        if surely_fits(earlier, HISTORY_TOKENS.saturating_sub(translation.len())) {
            return Ok(true);
        }
        Ok(self.model.tokens(earlier)? + self.model.tokens(translation)? <= HISTORY_TOKENS)
    }
}

/// Whether `text` fits in `max` tokens without asking the engine: a token stands for at least one
/// byte of text, so a text of no more bytes than `max` does. Most text sent is that short, and is
/// then sent without a `/tokenize` before it.
fn surely_fits(text: &str, max: usize) -> bool {
    text.len() <= max
}

/// The translation as it is sent to the screen.
struct Output<F> {
    text: String,
    emit: F,
    /// Whether the current piece's translation has started: whitespace before it is dropped.
    started: bool,
    /// Whitespace at the end of the current piece's translation, sent only once more of the
    /// translation follows it: the original's own whitespace goes between the pieces.
    trailing: String,
    /// The original's whitespace between the last piece and the next one of the same 翻訳単位, held
    /// until the next piece's translation starts and [`joint`] can see both sides.
    join: Option<String>,
    /// How the current 翻訳単位's translation writes its line breaks.
    lines: Lines,
}

/// Where the translation stood before a piece, to go back to with [`Output::rewind`].
struct Mark {
    len: usize,
    join: Option<String>,
}

impl<F: FnMut(Update<'_>)> Output<F> {
    fn new(emit: F) -> Self {
        Output {
            text: String::new(),
            emit,
            started: false,
            trailing: String::new(),
            join: None,
            lines: Lines::Keep(String::new()),
        }
    }

    /// Text of the original, sent as it is: what lies before the first 翻訳単位 and between two of
    /// them.
    fn raw(&mut self, text: &str) {
        self.join = None;
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        (self.emit)(Update::Append(text));
    }

    fn begin_unit(&mut self, lines: Lines) {
        self.join = None;
        self.lines = lines;
    }

    /// The whitespace after a piece that the next piece of the same 翻訳単位 follows.
    fn join(&mut self, original: &str) {
        self.join = Some(original.to_owned());
    }

    /// Text of the translation, its line breaks written as [`Output::lines`] says.
    fn push(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let start = self.text.len();
        self.lines.write(&mut self.text, text);
        (self.emit)(Update::Append(&self.text[start..]));
    }

    /// The next piece of the engine's translation.
    fn translation(&mut self, piece: &str) {
        let piece = if self.started {
            piece
        } else {
            piece.trim_start()
        };
        let Some(first) = piece.chars().next() else {
            return;
        };
        if !self.started {
            self.started = true;
            if let Some(original) = self.join.take() {
                let joint = joint(&original, self.text.chars().next_back(), first);
                self.push(joint);
            }
        }
        let body = piece.trim_end();
        if body.is_empty() {
            self.trailing.push_str(piece);
            return;
        }
        let mut text = std::mem::take(&mut self.trailing);
        text.push_str(body);
        self.push(&text);
        self.trailing.push_str(&piece[body.len()..]);
    }

    fn end_translation(&mut self) {
        self.started = false;
        self.trailing.clear();
    }

    fn mark(&self) -> Mark {
        Mark {
            len: self.text.len(),
            join: self.join.clone(),
        }
    }

    /// Takes back everything sent since `mark`.
    fn rewind(&mut self, mark: Mark) {
        self.text.truncate(mark.len);
        self.join = mark.join;
        self.end_translation();
        (self.emit)(Update::Replace(&self.text));
    }

    /// Puts `text` in place of everything sent since `mark`.
    fn replace_since(&mut self, mark: Mark, text: &str) {
        self.text.truncate(mark.len);
        self.text.push_str(text);
        self.join = None;
        self.end_translation();
        (self.emit)(Update::Replace(&self.text));
    }
}

/// The whitespace between the translations of two pieces of one 翻訳単位, whose characters on either
/// side are `before` and `after`. A line break of the original stays. Otherwise the pieces are
/// sentences, and the space between them follows the translation's script rather than the
/// original's: Japanese and Chinese set sentences close, and the languages written with spaces
/// put one between them — `。` then `次` joins as it is, `.` then `T` gets a space even when the
/// original was Japanese without one.
fn joint(original: &str, before: Option<char>, after: char) -> &str {
    if original.contains('\n') {
        return original;
    }
    match before {
        Some(before) if is_set_close(before) && is_set_close(after) => "",
        _ if original.is_empty() => " ",
        _ => original,
    }
}

/// Characters of the scripts that put no space between sentences: kanji, kana, and the CJK and
/// full-width punctuation that goes with them.
fn is_set_close(c: char) -> bool {
    matches!(
        c,
        '\u{3000}'..='\u{30FF}'
            | '\u{31F0}'..='\u{31FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FF00}'..='\u{FFEF}'
            | '\u{20000}'..='\u{3FFFF}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the stand-in engine answers for one input.
    enum Reply {
        Text(String),
        /// `prefix`, then `cycle` over and over until it is cut off.
        Loop(String, String),
        /// Ended by a stop.
        Stopped,
        Error,
    }

    /// A stand-in engine: one token per character, and translations from `reply`.
    struct Fake {
        reply: Box<dyn FnMut(&str) -> Reply>,
        prompts: Vec<String>,
        counted: Vec<String>,
    }

    impl Fake {
        fn new(reply: impl FnMut(&str) -> Reply + 'static) -> Self {
            Fake {
                reply: Box::new(reply),
                prompts: Vec::new(),
                counted: Vec::new(),
            }
        }

        /// The text each prompt asked to translate.
        fn inputs(&self) -> Vec<&str> {
            self.prompts.iter().map(|prompt| input_of(prompt)).collect()
        }
    }

    fn input_of(prompt: &str) -> &str {
        let last = prompt.rsplit("<|plamo:tag|>input").next().unwrap();
        let text = last.split_once("<|plamo:msg|>").unwrap().1;
        text.split_once("<|plamo:tag|>output").unwrap().0
    }

    /// Sends `text` three characters at a time, as a stream of tokens would.
    fn send(text: &str, on_piece: &mut dyn FnMut(&str) -> bool) -> bool {
        let chars: Vec<char> = text.chars().collect();
        chars
            .chunks(3)
            .all(|chunk| on_piece(&chunk.iter().collect::<String>()))
    }

    impl Model for Fake {
        fn tokens(&mut self, text: &str) -> Result<usize, String> {
            self.counted.push(text.to_owned());
            Ok(text.chars().count())
        }

        fn complete(
            &mut self,
            prompt: &str,
            on_piece: &mut dyn FnMut(&str) -> bool,
        ) -> Result<Completion, String> {
            self.prompts.push(prompt.to_owned());
            match (self.reply)(input_of(prompt)) {
                Reply::Text(text) => Ok(if send(&text, on_piece) {
                    Completion::Done
                } else {
                    Completion::Cut
                }),
                Reply::Loop(prefix, cycle) => {
                    if !send(&prefix, on_piece) {
                        return Ok(Completion::Cut);
                    }
                    for _ in 0..1000 {
                        if !send(&cycle, on_piece) {
                            return Ok(Completion::Cut);
                        }
                    }
                    Ok(Completion::Done)
                }
                Reply::Stopped => Ok(Completion::Stopped),
                Reply::Error => Err("the engine failed".into()),
            }
        }
    }

    /// Translates `text` from English into Japanese, and answers the outcome, the translation
    /// the screen ends up showing, and every update sent.
    fn run(fake: &mut Fake, text: &str) -> (Result<Outcome, String>, String, Vec<String>) {
        run_as(fake, text, Format::Text)
    }

    fn run_as(
        fake: &mut Fake,
        text: &str,
        format: Format,
    ) -> (Result<Outcome, String>, String, Vec<String>) {
        let mut shown = String::new();
        let mut updates = Vec::new();
        let outcome = translate(
            fake,
            text,
            &format,
            "ja",
            Some("en"),
            |update| match update {
                Update::Append(piece) => {
                    shown.push_str(piece);
                    updates.push(format!("+{piece}"));
                }
                Update::Replace(whole) => {
                    shown = whole.to_owned();
                    updates.push(format!("={whole}"));
                }
            },
        );
        (outcome, shown, updates)
    }

    /// Translates by wrapping each input in brackets.
    fn bracket(input: &str) -> Reply {
        Reply::Text(format!("[{input}]"))
    }

    #[test]
    fn paragraphs_are_translated_in_order_and_joined_with_the_original_whitespace() {
        let mut fake = Fake::new(bracket);
        let text = "\n  First line\nstill first.\n\n \nSecond.\n";
        let (outcome, shown, _) = run(&mut fake, text);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "\n  [First line\nstill first.]\n\n \n[Second.]\n");
        assert_eq!(fake.inputs(), ["First line\nstill first.", "Second."]);
    }

    #[test]
    fn each_prompt_carries_the_piece_before_it_and_its_translation() {
        let mut fake = Fake::new(bracket);
        run(&mut fake, "One.\n\nTwo.\n\nThree.").0.unwrap();
        assert_eq!(
            fake.prompts,
            [
                translate::prompt("One.", "ja", Some("en"), None),
                translate::prompt("Two.", "ja", Some("en"), Some(("One.", "[One.]"))),
                translate::prompt("Three.", "ja", Some("en"), Some(("Two.", "[Two.]"))),
            ]
        );
    }

    #[test]
    fn whitespace_around_the_engines_translation_is_dropped_and_inside_it_is_kept() {
        let mut fake = Fake::new(|input| Reply::Text(format!("\n  {input}\n\n訳の中の改行  \n ")));
        let (_, shown, _) = run(&mut fake, "A.\n\nB.");
        assert_eq!(shown, "A.\n\n訳の中の改行\n\nB.\n\n訳の中の改行");
        // The history carries the translation without it either.
        assert!(fake.prompts[1].contains("<|plamo:msg|>A.\n\n訳の中の改行<|plamo:tag|>input"));
    }

    #[test]
    fn a_short_text_is_sent_without_counting_its_tokens() {
        let mut fake = Fake::new(bracket);
        run(&mut fake, &"短い。".repeat(50)).0.unwrap();
        assert!(fake.counted.is_empty(), "{:?}", fake.counted);
    }

    /// `count` distinct sentences of 30 characters each.
    fn sentences(count: usize) -> Vec<String> {
        (0..count)
            .map(|i| format!("This is sentence number {i:04}."))
            .collect()
    }

    #[test]
    fn a_paragraph_too_long_for_a_prompt_is_cut_at_sentences() {
        let sentences = sentences(80);
        let paragraph = sentences.join(" ");
        assert!(paragraph.chars().count() > UNIT_TOKENS);
        let mut fake = Fake::new(bracket);
        let (outcome, shown, _) = run(&mut fake, &paragraph);
        assert_eq!(outcome, Ok(Outcome::Done));

        let inputs = fake.inputs();
        assert_eq!(inputs.len(), 3);
        for input in &inputs {
            assert!(input.chars().count() <= UNIT_TOKENS, "{} long", input.len());
            assert!(input.ends_with('.'), "cut inside a sentence: {input}");
        }
        assert_eq!(inputs.join(" "), paragraph);
        let brackets: Vec<String> = inputs.iter().map(|input| format!("[{input}]")).collect();
        assert_eq!(shown, brackets.join(" "));
        // The second piece carries the first as its history.
        assert_eq!(
            fake.prompts[1],
            translate::prompt(inputs[1], "ja", Some("en"), Some((inputs[0], &brackets[0])))
        );
    }

    #[test]
    fn a_history_too_long_is_left_out() {
        let first = sentences(30).join(" ");
        // Long enough that the first piece and its translation exceed HISTORY_TOKENS together.
        let long: String = (0..400).map(|i| format!("訳{i:03}、")).collect();
        let reply_long = long.clone();
        let mut fake = Fake::new(move |input| {
            if input.starts_with("This") {
                Reply::Text(reply_long.clone())
            } else {
                bracket(input)
            }
        });
        let (outcome, _, _) = run(&mut fake, &format!("{first}\n\nNext."));
        assert_eq!(outcome, Ok(Outcome::Done));
        assert!(first.chars().count() + long.chars().count() > HISTORY_TOKENS);
        assert_eq!(
            fake.prompts[1],
            translate::prompt("Next.", "ja", Some("en"), None)
        );
    }

    #[test]
    fn a_translation_that_repeats_itself_is_taken_back_and_made_again_by_sentence() {
        let paragraph = "First one. Second one. Third one.";
        let mut fake = Fake::new(move |input| {
            if input == paragraph {
                Reply::Loop("一つ目。".into(), "同じ文の繰り返し。".into())
            } else {
                bracket(input)
            }
        });
        let (outcome, shown, updates) =
            run(&mut fake, &format!("Before.\n\n{paragraph}\n\nAfter."));
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(
            shown,
            "[Before.]\n\n[First one.] [Second one.] [Third one.]\n\n[After.]"
        );
        assert_eq!(
            fake.inputs(),
            [
                "Before.",
                paragraph,
                "First one.",
                "Second one.",
                "Third one.",
                "After.",
            ]
        );
        // The repetition shown so far is taken back to where the paragraph began.
        assert!(
            updates.contains(&"=[Before.]\n\n".to_owned()),
            "{updates:?}"
        );
        // Each sentence carries the one before it.
        assert_eq!(
            fake.prompts[3],
            translate::prompt(
                "Second one.",
                "ja",
                Some("en"),
                Some(("First one.", "[First one.]"))
            )
        );
    }

    #[test]
    fn a_sentence_that_repeats_itself_keeps_its_first_time_round() {
        let mut fake = Fake::new(|input| {
            if input == "Loops." {
                Reply::Loop("始めに".into(), "同じ言葉、".into())
            } else {
                bracket(input)
            }
        });
        let (outcome, shown, _) = run(&mut fake, "Loops.\n\nNext.");
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "始めに同じ言葉、\n\n[Next.]");
        // One prompt for it: a single sentence cannot be cut smaller.
        assert_eq!(fake.inputs(), ["Loops.", "Next."]);
        // The next prompt does not carry a translation that was cut short.
        assert_eq!(
            fake.prompts[1],
            translate::prompt("Next.", "ja", Some("en"), None)
        );
    }

    #[test]
    fn sentences_of_japanese_get_a_space_between_their_english_translations() {
        let mut fake = Fake::new(|input| match input {
            "一文目。二文目。" => Reply::Loop(String::new(), "繰り返し".into()),
            "一文目。" => Reply::Text("The first.".into()),
            "二文目。" => Reply::Text("The second.".into()),
            _ => bracket(input),
        });
        let (_, shown, _) = run(&mut fake, "一文目。二文目。");
        assert_eq!(shown, "The first. The second.");
    }

    #[test]
    fn sentences_of_english_join_close_in_their_japanese_translations() {
        let mut fake = Fake::new(|input| match input {
            "One. Two." => Reply::Loop(String::new(), "repeat ".into()),
            "One." => Reply::Text("一つ。".into()),
            "Two." => Reply::Text("二つ。".into()),
            _ => bracket(input),
        });
        let (_, shown, _) = run(&mut fake, "One. Two.");
        assert_eq!(shown, "一つ。二つ。");
    }

    #[test]
    fn the_joint_keeps_a_line_break_and_otherwise_follows_the_script() {
        assert_eq!(joint("\n  ", Some('.'), 'T'), "\n  ");
        assert_eq!(joint("\n", Some('。'), '次'), "\n");
        assert_eq!(joint("", Some('.'), 'T'), " ");
        assert_eq!(joint("", Some('。'), 'S'), " ");
        assert_eq!(joint("  ", Some('.'), 'T'), "  ");
        assert_eq!(joint(" ", Some('。'), '次'), "");
        assert_eq!(joint("", Some('」'), '「'), "");
        assert_eq!(joint("", Some('！'), '한'), " ");
    }

    #[test]
    fn a_stop_ends_the_translation_where_it_is() {
        let mut fake = Fake::new(|input| {
            if input == "Two." {
                Reply::Stopped
            } else {
                bracket(input)
            }
        });
        let (outcome, shown, _) = run(&mut fake, "One.\n\nTwo.\n\nThree.");
        assert_eq!(outcome, Ok(Outcome::Stopped));
        assert_eq!(shown, "[One.]\n\n");
        assert_eq!(fake.inputs(), ["One.", "Two."]);
    }

    #[test]
    fn a_stop_while_retrying_by_sentence_ends_it_too() {
        let mut fake = Fake::new(|input| match input {
            "A one. B two." => Reply::Loop(String::new(), "繰り返し".into()),
            "B two." => Reply::Stopped,
            _ => bracket(input),
        });
        let (outcome, shown, _) = run(&mut fake, "A one. B two.\n\nC.");
        assert_eq!(outcome, Ok(Outcome::Stopped));
        assert_eq!(shown, "[A one.]");
        assert_eq!(fake.inputs(), ["A one. B two.", "A one.", "B two."]);
    }

    #[test]
    fn an_engine_error_is_passed_on() {
        let mut fake = Fake::new(|input| {
            if input == "Two." {
                Reply::Error
            } else {
                bracket(input)
            }
        });
        let (outcome, shown, _) = run(&mut fake, "One.\n\nTwo.");
        assert_eq!(outcome, Err("the engine failed".into()));
        assert_eq!(shown, "[One.]\n\n");
    }

    #[test]
    fn a_text_of_whitespace_asks_the_engine_nothing() {
        let mut fake = Fake::new(bracket);
        let (outcome, shown, _) = run(&mut fake, " \n\n ");
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, " \n\n ");
        assert!(fake.prompts.is_empty());
    }

    /// Translates by writing the letters outside tags in capitals, keeping the tags, as an engine
    /// that keeps them does.
    fn shout(input: &str) -> Reply {
        let mut out = String::new();
        let mut in_tag = false;
        for c in input.chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                _ => {}
            }
            if in_tag || c == '>' {
                out.push(c);
            } else {
                out.extend(c.to_uppercase());
            }
        }
        Reply::Text(out)
    }

    const MARKDOWN: &str = r#"---
title: Guide
---

# Getting *started* {#start}

Install the app with `npm install tiny`, then read
[the guide](https://example.com/guide "Guide") or https://example.com/faq.

> [!NOTE]
> Keep the **model** file
> next to the app.  
> It is large.

- First item
- [x] Done item with ~~old~~ text
  1. Nested _step_

| Option | Meaning |
|--------|---------|
| `--port` | The port \| number |
| 42 | Answer |

```sh
npm run build
```

<div align="center">
  <img src="logo.png">
</div>

See [the guide] and the note[^1].

[the guide]: https://example.com/guide
[^1]: A footnote.
"#;

    #[test]
    fn markdown_keeps_its_structure_and_translates_only_its_text() {
        let mut fake = Fake::new(shout);
        let (outcome, shown, updates) = run_as(&mut fake, MARKDOWN, Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(
            shown,
            r#"---
title: Guide
---

# GETTING *STARTED* {#start}

INSTALL THE APP WITH `npm install tiny`, THEN READ
[THE GUIDE](https://example.com/guide "Guide") OR https://example.com/faq.

> [!NOTE]
> KEEP THE **MODEL** FILE
> NEXT TO THE APP.  
> IT IS LARGE.

- FIRST ITEM
- [x] DONE ITEM WITH ~~OLD~~ TEXT
  1. NESTED *STEP*

| OPTION | MEANING |
|--------|---------|
| `--port` | THE PORT \| NUMBER |
| 42 | ANSWER |

```sh
npm run build
```

<div align="center">
  <img src="logo.png">
</div>

SEE [THE GUIDE][the guide] AND THE NOTE[^1].

[the guide]: https://example.com/guide
[^1]: A FOOTNOTE.
"#
        );
        assert_eq!(
            fake.inputs(),
            [
                "Getting <em i=\"1\">started</em>",
                "Install the app with <code i=\"1\">npm install tiny</code>, then read\n\
                 <a i=\"2\">the guide</a> or <code i=\"3\">https://example.com/faq</code>.",
                "Keep the <strong i=\"1\">model</strong> file\nnext to the app.<br i=\"2\"/>It is large.",
                "First item",
                "Done item with <s i=\"1\">old</s> text",
                "Nested <em i=\"1\">step</em>",
                "Option",
                "Meaning",
                "The port \\| number",
                "Answer",
                "See <a i=\"1\">the guide</a> and the note<x i=\"2\"/>.",
                "A footnote.",
            ]
        );
        // The screen never shows a tag, not even while a translation streams in.
        for update in &updates {
            assert!(!update.contains(" i=\""), "{update}");
        }
        // The history carries the tags, so the next translation keeps them likewise.
        assert_eq!(
            fake.prompts[1],
            translate::prompt(
                fake.inputs()[1],
                "ja",
                Some("en"),
                Some((
                    "Getting <em i=\"1\">started</em>",
                    "GETTING <em i=\"1\">STARTED</em>"
                ))
            )
        );
    }

    #[test]
    fn tags_the_translation_does_not_keep_are_left_out_and_it_is_made_again() {
        let mut fake = Fake::new(|input| match input {
            "Read <em i=\"1\">this</em> now." => Reply::Text("READ THIS NOW.".into()),
            _ => shout(input),
        });
        let (outcome, shown, _) =
            run_as(&mut fake, "Intro.\n\nRead *this* now.\n", Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "INTRO.\n\nREAD THIS NOW.\n");
        assert_eq!(
            fake.inputs(),
            [
                "Intro.",
                "Read <em i=\"1\">this</em> now.",
                "Read this now."
            ]
        );
        // Without the history, whose tags would ask for tags again.
        assert_eq!(
            fake.prompts[2],
            translate::prompt("Read this now.", "ja", Some("en"), None)
        );
    }

    #[test]
    fn code_survives_a_translation_that_does_not_keep_the_tags() {
        let mut fake = Fake::new(|input| match input {
            "Run <code i=\"1\">hello</code> <em i=\"2\">now</em>." => {
                Reply::Text("RUN HELLO NOW.".into())
            }
            _ => shout(input),
        });
        let (outcome, shown, _) = run_as(&mut fake, "Run `hello` *now*.", Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        // The engine wrote the code in capitals; the original comes back all the same.
        assert_eq!(shown, "RUN `hello` NOW.");
        assert_eq!(
            fake.inputs(),
            [
                "Run <code i=\"1\">hello</code> <em i=\"2\">now</em>.",
                "Run <code i=\"1\">hello</code> now.",
            ]
        );
    }

    #[test]
    fn without_any_tags_kept_the_unit_is_made_again_as_plain_text() {
        let mut fake = Fake::new(|input| {
            if input.contains('<') {
                Reply::Text("タグなし".into())
            } else {
                Reply::Text(input.replace("Run", "実行").replace("now", "今"))
            }
        });
        let (outcome, shown, _) = run_as(&mut fake, "Run `hello` *now*.", Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "実行 `hello` 今.");
        assert_eq!(fake.inputs().len(), 3);
        assert_eq!(fake.inputs()[2], "Run `hello` now.");
    }

    #[test]
    fn a_unit_whose_code_the_last_try_changed_stays_as_the_original() {
        // With paired elements: both tagged tries fail, and the try without tags changes the code.
        let mut fake = Fake::new(|input| {
            if input.contains('<') {
                Reply::Text("タグなし".into())
            } else {
                shout(input)
            }
        });
        let text = "Intro.\n\n> Run `hello` *now*\n> please.\n\nNext.";
        let (outcome, shown, updates) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "INTRO.\n\n> Run `hello` *now*\n> please.\n\nNEXT.");
        assert!(updates.iter().any(|update| update.starts_with('=')));
        // The next 翻訳単位 goes without a history the original stands in for.
        assert_eq!(
            fake.prompts.last().unwrap(),
            &translate::prompt("Next.", "ja", Some("en"), None)
        );

        // With kept elements only, there is no try in between.
        let mut fake = Fake::new(|input| {
            if input.contains('<') {
                Reply::Text("USE こんにちは NOW.".into())
            } else {
                Reply::Text("Use `こんにちは` now.".into())
            }
        });
        let (outcome, shown, _) = run_as(&mut fake, "Use `hello` now.", Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "Use `hello` now.");
        assert_eq!(
            fake.inputs(),
            ["Use <code i=\"1\">hello</code> now.", "Use `hello` now."]
        );
    }

    #[test]
    fn a_url_the_untagged_try_extended_keeps_the_original() {
        let translate_to = |url: &'static str| {
            move |input: &str| {
                if input.contains('<') {
                    Reply::Text("タグなし".into())
                } else {
                    Reply::Text(format!("今 {url} を見る。"))
                }
            }
        };
        let text = "Visit https://example.com now.";
        let mut fake = Fake::new(translate_to("https://example.com/ja"));
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(shown, text);
        let mut fake = Fake::new(translate_to("https://example.com"));
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(shown, "今 https://example.com を見る。");
        // A URL that ends in punctuation can be continued as well.
        let text = "Visit https://example.com/search?q= now.";
        let mut fake = Fake::new(translate_to("https://example.com/search?q=ja"));
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(shown, text);
        let text = "Visit https://example.com/a_(b) now.";
        let mut fake = Fake::new(translate_to("https://example.com/a_(b)/ja"));
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(shown, text);
    }

    #[test]
    fn a_collapsed_link_that_ends_a_unit_comes_back_once() {
        let text = "Go [Home][]\n\n[home]: /h\n";
        let mut fake = Fake::new(shout);
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(shown, "GO [HOME][Home]\n\n[home]: /h\n");
        let mut fake = Fake::new(|input| {
            if input.contains('<') {
                Reply::Text("GO HOME".into())
            } else {
                shout(input)
            }
        });
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(shown, "GO HOME\n\n[home]: /h\n");
    }

    #[test]
    fn a_tagged_unit_that_repeats_itself_is_made_again_without_tags() {
        let mut fake = Fake::new(|input| {
            if input.contains("<em") {
                Reply::Loop("始め".into(), "<em i=\"1\">同じ</em>".into())
            } else {
                shout(input)
            }
        });
        let (outcome, shown, _) = run_as(&mut fake, "One *two*. Three.", Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, "ONE TWO. THREE.");
        assert_eq!(
            fake.inputs(),
            ["One <em i=\"1\">two</em>. Three.", "One two. Three."]
        );
    }

    #[test]
    fn a_tagged_unit_too_long_for_a_prompt_goes_without_tags() {
        let paragraph = format!("*Start.* {}", sentences(80).join(" "));
        let mut fake = Fake::new(shout);
        let (outcome, _, _) = run_as(&mut fake, &paragraph, Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Done));
        let inputs = fake.inputs();
        assert!(inputs.len() > 1);
        assert!(inputs[0].starts_with("Start. This is"), "{}", inputs[0]);
        assert!(inputs.iter().all(|input| !input.contains('<')));
    }

    #[test]
    fn line_breaks_of_a_translation_follow_its_block() {
        let mut fake = Fake::new(|input| match input {
            "quoted" => Reply::Text("一行目\n二行目".into()),
            "Title" => Reply::Text("見出し\nの続き".into()),
            "cell" => Reply::Text("a | b \\| c\nd".into()),
            _ => shout(input),
        });
        let text = "> - quoted\n\n## Title\n\n| cell |\n|---|\n";
        let (_, shown, _) = run_as(&mut fake, text, Format::Markdown);
        assert_eq!(
            shown,
            "> - 一行目\n>   二行目\n\n## 見出し の続き\n\n| a \\| b \\| c d |\n|---|\n"
        );
    }

    #[test]
    fn a_stop_during_a_tagged_unit_ends_the_translation() {
        let mut fake = Fake::new(|input| {
            if input.contains("<em") {
                Reply::Stopped
            } else {
                shout(input)
            }
        });
        let (outcome, shown, _) = run_as(&mut fake, "One.\n\n*Two*.\n\nThree.", Format::Markdown);
        assert_eq!(outcome, Ok(Outcome::Stopped));
        assert_eq!(shown, "ONE.\n\n");
    }

    const JSON: &str = r#"{
  "app": {
    "title": "Tiny \"Translator\"",
    "greeting": "Hello, ",
    "id": "home",
    "lines": "First line\nSecond line"
  },
  "count": 3,
  "ratio": 1.50,
  "enabled": true,
  "missing": null,
  "items": ["One", 2, "", "42", {"id": "x", "label": "Two\\Three"}],
  "zeta": "Last"
}"#;

    #[test]
    fn json_keeps_its_structure_and_key_order_and_translates_only_string_values() {
        let mut fake = Fake::new(|input| {
            Reply::Text(
                match input {
                    "Tiny \"Translator\"" => "小さな「翻訳機」\"引用\"",
                    "Hello," => "こんにちは、",
                    "First line\nSecond line" => "一行目\n二行目\t終わり",
                    "One" => "一",
                    "Two\\Three" => "二\\三",
                    "Last" => "最後",
                    other => panic!("not to be translated: {other:?}"),
                }
                .into(),
            )
        });
        let format = Format::Json {
            untranslated_keys: vec!["id".into()],
        };
        let (outcome, shown, updates) = run_as(&mut fake, JSON, format);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(
            shown,
            r#"{
  "app": {
    "title": "小さな「翻訳機」\"引用\"",
    "greeting": "こんにちは、 ",
    "id": "home",
    "lines": "一行目\n二行目\t終わり"
  },
  "count": 3,
  "ratio": 1.50,
  "enabled": true,
  "missing": null,
  "items": ["一", 2, "", "42", {"id": "x", "label": "二\\三"}],
  "zeta": "最後"
}"#
        );
        // The translation reads as JSON, with the original's keys, types and nesting.
        let original: serde_json::Value = serde_json::from_str(JSON).unwrap();
        let translated: serde_json::Value = serde_json::from_str(&shown).unwrap();
        assert_eq!(translated["app"]["title"], "小さな「翻訳機」\"引用\"");
        assert_eq!(translated["app"]["lines"], "一行目\n二行目\t終わり");
        assert_eq!(translated["items"][4]["label"], "二\\三");
        for (key, value) in
            ["count", "ratio", "enabled", "missing"].map(|key| (key, &original[key]))
        {
            assert_eq!(&translated[key], value, "{key}");
        }
        assert_eq!(translated["app"]["id"], "home");
        assert_eq!(translated["items"][1], 2);
        // Every update the screen was sent keeps the strings escaped, as they stream in.
        assert!(updates.iter().all(|update| !update.contains("\n二行目")));
    }

    #[test]
    fn a_long_json_string_is_cut_at_sentences_and_its_joints_escaped() {
        let mut fake = Fake::new(bracket);
        let sentences: Vec<String> = (0..60)
            .map(|i| format!("Sentence number {i} here."))
            .collect();
        let value = sentences.join("\n");
        let text = format!("{{\"long\": {}}}", serde_json::to_string(&value).unwrap());
        let format = Format::Json {
            untranslated_keys: Vec::new(),
        };
        let (outcome, shown, _) = run_as(&mut fake, &text, format);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert!(fake.prompts.len() >= 2);
        let translated: serde_json::Value = serde_json::from_str(&shown).unwrap();
        let long = translated["long"].as_str().unwrap();
        assert!(long.starts_with("[Sentence number 0 here."));
        assert_eq!(long.matches('\n').count(), 59);
    }

    fn json() -> Format {
        Format::Json {
            untranslated_keys: Vec::new(),
        }
    }

    #[test]
    fn json_placeholders_come_back_as_written_whatever_the_translation_holds() {
        let text = r#"{
  "greet": "Hi {name}, you have %1$d new {{kind}}",
  "files": "{count, plural, =0 {No files} one {# file} other {# files}} in \"{dir}\"",
  "only": "%s"
}"#;
        let mut fake = Fake::new(|input| {
            Reply::Text(
                match input {
                    // Reordered, with what is inside the tags changed.
                    "Hi <code i=\"1\">{name}</code>, you have <code i=\"2\">%1$d</code> new <code i=\"3\">{{kind}}</code>" => {
                        "<code i=\"1\">{名前}</code>さん、新しい<code i=\"3\">{{種類}}</code>が<code i=\"2\">%d</code>件"
                    }
                    "No files" => "ファイルなし",
                    "<code i=\"1\">#</code> file" | "<code i=\"1\">#</code> files" => {
                        "<code i=\"1\"># </code>件のファイル"
                    }
                    "in \"<code i=\"1\">{dir}</code>\"" => "「<code i=\"1\">{フォルダ}</code>」の中",
                    other => panic!("not to be translated: {other:?}"),
                }
                .into(),
            )
        });
        let (outcome, shown, updates) = run_as(&mut fake, text, json());
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(
            shown,
            r#"{
  "greet": "{name}さん、新しい{{kind}}が%1$d件",
  "files": "{count, plural, =0 {ファイルなし} one {#件のファイル} other {#件のファイル}} 「{dir}」の中",
  "only": "%s"
}"#
        );
        assert!(serde_json::from_str::<serde_json::Value>(&shown).is_ok());
        for update in &updates {
            assert!(!update.contains(" i=\""), "{update}");
        }
    }

    #[test]
    fn a_json_value_is_made_again_without_tags_and_kept_when_that_changes_a_placeholder() {
        let text = r#"["Hi {name}!", "Bye {name}.", "%s left", "Next."]"#;
        let mut fake = Fake::new(|input| {
            Reply::Text(
                match input {
                    // Tags dropped, so each goes again without them.
                    i if i.contains('<') => "タグなし",
                    // Holds the placeholder as written: the translation stands.
                    "Hi {name}!" => "こんにちは、{name}!",
                    // Changes it: the value stays as the original.
                    "Bye {name}." => "さようなら、{名前}。",
                    // Drops it: the value stays as the original too.
                    "%s left" => "残り",
                    "Next." => "次。",
                    other => panic!("not to be translated: {other:?}"),
                }
                .into(),
            )
        });
        let (outcome, shown, _) = run_as(&mut fake, text, json());
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(
            shown,
            r#"["こんにちは、{name}!", "Bye {name}.", "%s left", "次。"]"#
        );
        assert_eq!(
            fake.inputs(),
            [
                "Hi <code i=\"1\">{name}</code>!",
                "Hi {name}!",
                "Bye <code i=\"1\">{name}</code>.",
                "Bye {name}.",
                "<code i=\"1\">%s</code> left",
                "%s left",
                "Next.",
            ]
        );
        // The value kept as the original leaves no history for the next one.
        assert_eq!(
            fake.prompts.last().unwrap(),
            &translate::prompt("Next.", "ja", Some("en"), None)
        );
    }

    #[test]
    fn a_text_that_is_not_json_fails_before_anything_is_sent() {
        let mut fake = Fake::new(bracket);
        let format = Format::Json {
            untranslated_keys: Vec::new(),
        };
        let (outcome, shown, updates) = run_as(&mut fake, "{\"a\": \"b\",}", format);
        assert!(outcome.unwrap_err().starts_with("not JSON: "));
        assert_eq!(shown, "");
        assert!(updates.is_empty());
        assert!(fake.prompts.is_empty());
    }

    #[test]
    fn formats_are_named_as_the_screen_names_them() {
        let keys = vec!["id".to_owned()];
        assert_eq!(Format::from_name("text", keys.clone()), Some(Format::Text));
        assert_eq!(
            Format::from_name("markdown", keys.clone()),
            Some(Format::Markdown)
        );
        assert_eq!(
            Format::from_name("json", keys.clone()),
            Some(Format::Json {
                untranslated_keys: keys.clone()
            })
        );
        assert_eq!(Format::from_name("html", keys.clone()), Some(Format::Html));
        assert_eq!(Format::from_name("xml", keys), None);
    }

    const HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Tiny Translator</title>
  <style>body { font: 16px/1.5 sans-serif; }</style>
</head>
<body>
  <h1 id="top">Getting <em>started</em></h1>
  <p class="lead">
    Install the app with <code>npm install tiny</code>, then read
    <a href="/guide" title="The guide">the guide</a>.
  </p>
  <img src="logo.png" alt="The logo">
  <ul>
    <li>First item</li>
    <li>Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to copy.<br>Then paste.</li>
  </ul>
  <table>
    <tr><th>Option</th><td>42</td><td>&nbsp;</td></tr>
  </table>
  <pre><code>npm run build</code></pre>
  <!-- footer -->
  <script>document.title = "Hi";</script>
</body>
</html>
"#;

    #[test]
    fn html_keeps_its_structure_and_translates_only_its_text_alt_and_title() {
        let mut fake = Fake::new(shout);
        let (outcome, shown, updates) = run_as(&mut fake, HTML, Format::Html);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(
            shown,
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>TINY TRANSLATOR</title>
  <style>body { font: 16px/1.5 sans-serif; }</style>
</head>
<body>
  <h1 id="top">GETTING <em>STARTED</em></h1>
  <p class="lead">
    INSTALL THE APP WITH <code>npm install tiny</code>, THEN READ
    <a href="/guide" title="THE GUIDE">THE GUIDE</a>.
  </p>
  <img src="logo.png" alt="THE LOGO">
  <ul>
    <li>FIRST ITEM</li>
    <li>PRESS <kbd>CTRL</kbd>+<kbd>C</kbd> TO COPY.<br>THEN PASTE.</li>
  </ul>
  <table>
    <tr><th>OPTION</th><td>42</td><td>&nbsp;</td></tr>
  </table>
  <pre><code>npm run build</code></pre>
  <!-- footer -->
  <script>document.title = "Hi";</script>
</body>
</html>
"#
        );
        assert_eq!(
            fake.inputs(),
            [
                "Tiny Translator",
                "Getting <em i=\"1\">started</em>",
                "The guide",
                "Install the app with <code i=\"1\">npm install tiny</code>, then read\n\
                 <a i=\"2\">the guide</a>.",
                "The logo",
                "First item",
                "Press <kbd i=\"1\">Ctrl</kbd>+<kbd i=\"2\">C</kbd> to copy.<br i=\"3\"/>Then paste.",
                "Option",
            ]
        );
        for update in &updates {
            assert!(!update.contains(" i=\""), "{update}");
        }
        // The title inside the paragraph goes on its own, and the paragraph after it carries the
        // heading's translation as before.
        assert_eq!(
            fake.prompts[2],
            translate::prompt("The guide", "ja", Some("en"), None)
        );
        assert_eq!(
            fake.prompts[3],
            translate::prompt(
                fake.inputs()[3],
                "ja",
                Some("en"),
                Some((
                    "Getting <em i=\"1\">started</em>",
                    "GETTING <em i=\"1\">STARTED</em>"
                ))
            )
        );
    }

    #[test]
    fn html_markup_the_try_without_tags_dropped_keeps_the_original() {
        let text = "<p>Press<br>now <b>quickly</b>.</p>";
        let mut fake = Fake::new(|input| {
            if input.contains('<') && !input.contains("<br>") {
                Reply::Text("タグなし".into())
            } else {
                Reply::Text("PRESS NOW QUICKLY.".into())
            }
        });
        let (outcome, shown, _) = run_as(&mut fake, text, Format::Html);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, text);
        assert_eq!(
            fake.inputs(),
            [
                "Press<br i=\"1\"/>now <b i=\"2\">quickly</b>.",
                "Press<br i=\"1\"/>now quickly.",
                "Press<br>now quickly.",
            ]
        );

        // Kept as written, the markup lets the translation without tags stand.
        let mut fake = Fake::new(|input| {
            if input.contains("<br>") {
                shout(input)
            } else {
                Reply::Text("タグなし".into())
            }
        });
        let (_, shown, _) = run_as(&mut fake, text, Format::Html);
        assert_eq!(shown, "<p>PRESS<br>NOW QUICKLY.</p>");
    }

    #[test]
    fn an_attribute_value_that_repeats_itself_stays_as_written() {
        let mut fake = Fake::new(|input| {
            if input == "Home" {
                Reply::Loop("ホーム".into(), "ホーム".into())
            } else {
                shout(input)
            }
        });
        let text = r#"<p>Go <a href="/" title="Home">back</a>.</p>"#;
        let (outcome, shown, _) = run_as(&mut fake, text, Format::Html);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(shown, r#"<p>GO <a href="/" title="Home">BACK</a>.</p>"#);
    }

    #[test]
    fn a_stop_while_translating_an_attribute_value_ends_the_translation() {
        let mut fake = Fake::new(|input| {
            if input == "Home" {
                Reply::Stopped
            } else {
                shout(input)
            }
        });
        let text = r#"<p>One.</p><p>Go <a href="/" title="Home">back</a>.</p><p>Three.</p>"#;
        let (outcome, shown, _) = run_as(&mut fake, text, Format::Html);
        assert_eq!(outcome, Ok(Outcome::Stopped));
        assert_eq!(shown, "<p>ONE.</p><p>");
    }
}
