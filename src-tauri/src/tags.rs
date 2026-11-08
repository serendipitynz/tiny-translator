//! 番号付きのタグ (doc-1 §7.1): the inline elements of a 翻訳単位, sent to the engine as tags it keeps
//! in place, and put back as the original markup.
//!
//! PLaMo 3 Translate learnt to keep tags for browser translation, so an element goes to it as
//! `<em i="1">…</em>` and the translation's tags say where the element's markup goes. The number
//! ties each tag to its element, so the translation may reorder them as its language needs. A
//! translation whose tags do not answer the original's one for one is not put back: the caller
//! translates the text again with only the tags of kept and void elements, whose originals must
//! survive, and without tags at all when that fails too.
//!
//! Three kinds of element:
//! - 対 (paired): markup around text that is translated, such as emphasis or a link.
//! - そのまま (kept): text that is not translated, such as inline code or a URL. It is shown to the
//!   engine for the sentence's sake, and whatever the translation holds inside the tag is replaced
//!   by the original.
//! - 空要素 (void): markup that carries no text, such as a hard line break.

/// The elements of a 翻訳単位, for putting a translation's tags back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tagged {
    elements: Vec<Element>,
    /// The 翻訳単位 without tags: the text translated when the tags cannot be put back.
    plain: String,
    /// The 翻訳単位 with only its kept and void elements as tags, and those elements: the text to
    /// try before going without tags. `None` when that would be the same text as this one or as
    /// `plain`.
    fallback: Option<Box<(String, Tagged)>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Element {
    name: &'static str,
    kind: Kind,
    /// Paired: the markup before the text. Kept and void: the original as a whole.
    open: String,
    /// Paired: the markup after the text.
    close: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Paired,
    Kept,
    Void,
}

/// Builds a 翻訳単位's text with tags and, alongside, the same text with only the tags of kept and
/// void elements, and without tags.
#[derive(Debug, Default)]
pub struct Builder {
    text: String,
    plain: String,
    elements: Vec<Element>,
    /// The paired elements open at the end of the text.
    open: Vec<usize>,
    words: bool,
    kept_text: String,
    kept_elements: Vec<Element>,
}

impl Builder {
    /// Text to translate.
    pub fn text(&mut self, text: &str) {
        self.words |= text.chars().any(char::is_alphabetic);
        self.text.push_str(text);
        self.kept_text.push_str(text);
        self.plain.push_str(text);
    }

    /// Starts a paired element written `open` before its text. Ended by [`Builder::close`].
    pub fn open(&mut self, name: &'static str, open: String) {
        let i = self.push(name, Kind::Paired, open);
        self.text.push_str(&format!("<{name} i=\"{i}\">"));
        self.open.push(i - 1);
    }

    /// Ends the paired element opened last, written `close` after its text.
    pub fn close(&mut self, close: String) {
        let index = self.open.pop().expect("a paired element is open");
        let element = &mut self.elements[index];
        element.close = close;
        self.text.push_str(&format!("</{}>", element.name));
    }

    /// Text kept as `original`, shown to the engine as `shown`.
    pub fn kept(&mut self, name: &'static str, original: &str, shown: &str) {
        let i = self.push(name, Kind::Kept, original.to_owned());
        self.text
            .push_str(&format!("<{name} i=\"{i}\">{shown}</{name}>"));
        let i = self.push_kept(name, Kind::Kept, original);
        self.kept_text
            .push_str(&format!("<{name} i=\"{i}\">{shown}</{name}>"));
        self.plain.push_str(original);
    }

    /// Markup kept as `original`, standing for `plain` in the text without tags.
    pub fn void(&mut self, name: &'static str, original: &str, plain: &str) {
        let i = self.push(name, Kind::Void, original.to_owned());
        self.text.push_str(&format!("<{name} i=\"{i}\"/>"));
        let i = self.push_kept(name, Kind::Void, original);
        self.kept_text.push_str(&format!("<{name} i=\"{i}\"/>"));
        self.plain.push_str(plain);
    }

    fn push_kept(&mut self, name: &'static str, kind: Kind, original: &str) -> usize {
        self.kept_elements.push(Element {
            name,
            kind,
            open: original.to_owned(),
            close: String::new(),
        });
        self.kept_elements.len()
    }

    fn push(&mut self, name: &'static str, kind: Kind, open: String) -> usize {
        self.elements.push(Element {
            name,
            kind,
            open,
            close: String::new(),
        });
        self.elements.len()
    }

    /// Whether the text to translate, outside kept and void elements, has a letter in it: a
    /// 翻訳単位 without one (a number, a URL alone) is left as it is.
    pub fn has_words(&self) -> bool {
        self.words
    }

    /// The text with tags, and its elements when it has any.
    pub fn finish(self) -> (String, Option<Tagged>) {
        debug_assert!(self.open.is_empty(), "every paired element is closed");
        if self.elements.is_empty() {
            return (self.text, None);
        }
        let all_kept = self.kept_elements.len() == self.elements.len();
        let fallback = (!all_kept && !self.kept_elements.is_empty()).then(|| {
            let tagged = Tagged {
                elements: self.kept_elements,
                plain: self.plain.clone(),
                fallback: None,
            };
            Box::new((self.kept_text, tagged))
        });
        let tagged = Tagged {
            elements: self.elements,
            plain: self.plain,
            fallback,
        };
        (self.text, Some(tagged))
    }
}

impl Tagged {
    pub fn plain(&self) -> &str {
        &self.plain
    }

    /// Whether `translation`, made from the text without tags, holds every kept element's original
    /// as written: the model was free to change them there.
    pub fn keeps_originals(&self, translation: &str) -> bool {
        let kept = || self.elements.iter().filter(|e| e.kind == Kind::Kept);
        kept().all(|element| {
            let wanted = kept().filter(|other| other.open == element.open).count();
            whole_occurrences(translation, &element.open) >= wanted
        })
    }

    /// The text with only the tags of kept and void elements, and those elements, to translate when
    /// this one's tags cannot be put back.
    pub fn fallback(&self) -> Option<(&str, &Tagged)> {
        self.fallback
            .as_deref()
            .map(|(text, tagged)| (text.as_str(), tagged))
    }

    /// `translation` with its tags put back as the original markup, or `None` when they do not
    /// answer the elements one for one: an element missing or twice, a tag of another name or kind
    /// under its number, or tags that cross. `write` adds the translated text between them to the
    /// result, as the format writes it.
    pub fn restore(
        &self,
        translation: &str,
        mut write: impl FnMut(&mut String, &str),
    ) -> Option<String> {
        let mut out = String::new();
        let mut used = vec![false; self.elements.len()];
        let mut open: Vec<usize> = Vec::new();
        // Inside a kept element, the translation's text is dropped for the original.
        let mut kept: Option<usize> = None;
        let mut rest = translation;
        while let Some(at) = rest.find('<') {
            let (before, from) = rest.split_at(at);
            if kept.is_none() {
                write(&mut out, before);
            }
            let Some((tag, len)) = Tag::parse(from) else {
                if kept.is_none() {
                    write(&mut out, "<");
                }
                rest = &from[1..];
                continue;
            };
            rest = &from[len..];
            if let Some(index) = kept {
                // Text inside a kept element can look like a tag (code that writes HTML); only its
                // own end tag counts.
                if tag == Tag::Close(self.elements[index].name) {
                    out.push_str(&self.elements[index].open);
                    kept = None;
                }
                continue;
            }
            match tag {
                Tag::Open(name, i) | Tag::Void(name, i) => {
                    let index = i.checked_sub(1)?;
                    let element = self.elements.get(index)?;
                    let void = matches!(tag, Tag::Void(..));
                    if used[index] || element.name != name || void != (element.kind == Kind::Void) {
                        return None;
                    }
                    used[index] = true;
                    match element.kind {
                        Kind::Paired => {
                            out.push_str(&element.open);
                            open.push(index);
                        }
                        Kind::Kept => kept = Some(index),
                        Kind::Void => out.push_str(&element.open),
                    }
                }
                Tag::Close(name) => match open.last() {
                    Some(&index) if self.elements[index].name == name => {
                        out.push_str(&self.elements[index].close);
                        open.pop();
                    }
                    _ if self.elements.iter().any(|element| element.name == name) => return None,
                    // Not one of ours: text the translation happens to hold.
                    _ => write(&mut out, &from[..len]),
                },
            }
        }
        if kept.is_some() || !open.is_empty() || used.contains(&false) {
            return None;
        }
        write(&mut out, rest);
        Some(out)
    }
}

/// How many times `original` stands in `text` whole: a URL that the text goes on with
/// (`https://example.com/ja` for `https://example.com`) is another URL. Only an original that ends
/// (or starts) with a letter, a digit or `/` can be continued; one between delimiters, such as
/// inline code in backquotes, cannot.
fn whole_occurrences(text: &str, original: &str) -> usize {
    let open_ended = |c: char| c.is_ascii_alphanumeric() || c == '/';
    text.match_indices(original)
        .filter(|&(at, _)| {
            let continued_before = original.starts_with(open_ended)
                && text[..at].chars().next_back().is_some_and(open_ended);
            let continued_after =
                original.ends_with(open_ended) && continues(&text[at + original.len()..]);
            !continued_before && !continued_after
        })
        .count()
}

/// Whether `rest` goes on with the URL or word before it: it has a character that a URL takes
/// before whitespace, a character outside ASCII, or its end. The punctuation that closes a
/// sentence or a bracket around a URL does not count on its own (`https://a.dev).`).
fn continues(rest: &str) -> bool {
    for c in rest.chars() {
        if !c.is_ascii_graphic() {
            return false;
        }
        if !matches!(
            c,
            '.' | ','
                | ':'
                | ';'
                | '!'
                | '?'
                | '\''
                | '"'
                | '*'
                | '_'
                | ')'
                | ']'
                | '<'
                | '>'
                | '`'
        ) {
            return true;
        }
    }
    false
}

/// A tag as [`Builder`] writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tag<'a> {
    /// `<name i="N">`
    Open(&'a str, usize),
    /// `</name>`
    Close(&'a str),
    /// `<name i="N"/>`
    Void(&'a str, usize),
}

impl<'a> Tag<'a> {
    /// The tag at the start of `text`, and its length.
    fn parse(text: &'a str) -> Option<(Tag<'a>, usize)> {
        let rest = text.strip_prefix('<')?;
        if let Some(rest) = rest.strip_prefix('/') {
            let (name, rest) = name(rest)?;
            rest.starts_with('>')
                .then(|| (Tag::Close(name), text.len() - rest.len() + 1))
        } else {
            let (name, rest) = name(rest)?;
            let rest = rest.strip_prefix(" i=\"")?;
            let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let i = rest[..digits].parse().ok()?;
            let rest = rest[digits..].strip_prefix('"')?;
            let (void, rest) = match rest.trim_start().strip_prefix('/') {
                Some(rest) => (true, rest),
                None => (false, rest),
            };
            let rest = rest.strip_prefix('>')?;
            let tag = if void {
                Tag::Void(name, i)
            } else {
                Tag::Open(name, i)
            };
            Some((tag, text.len() - rest.len()))
        }
    }
}

fn name(text: &str) -> Option<(&str, &str)> {
    let len = text.len()
        - text
            .trim_start_matches(|c: char| c.is_ascii_lowercase())
            .len();
    (len > 0).then(|| text.split_at(len))
}

/// Takes the tags out of a translation as it streams in, so the screen shows its text until the
/// whole translation is there to put the markup back.
#[derive(Debug, Default)]
pub struct Hide {
    /// The end of the stream so far that may be the start of a tag.
    pending: String,
}

/// The longest tag [`Hide`] waits for; a `<` that has gone this far without becoming one is text.
const LONGEST_TAG: usize = 32;

impl Hide {
    /// The text of `piece`, and of what was held back before it, that is surely not part of a tag.
    pub fn push(&mut self, piece: &str) -> String {
        self.pending.push_str(piece);
        let mut shown = String::new();
        let mut rest = self.pending.as_str();
        while let Some(at) = rest.find('<') {
            shown.push_str(&rest[..at]);
            let from = &rest[at..];
            if let Some((_, len)) = Tag::parse(from) {
                rest = &from[len..];
            } else if may_become_tag(from) {
                rest = from;
                break;
            } else {
                shown.push('<');
                rest = &from[1..];
            }
        }
        let held = if rest.contains('<') {
            rest
        } else {
            shown.push_str(rest);
            ""
        };
        self.pending = held.to_owned();
        shown
    }
}

/// Whether `from`, which starts with a `<` that is not a whole tag, may still become one as more
/// of the stream arrives.
fn may_become_tag(from: &str) -> bool {
    let next = from[1..].chars().next();
    !from.contains('>')
        && from.len() < LONGEST_TAG
        && next.is_none_or(|c| c == '/' || c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Run <code i="1">ls</code> as <em i="2">root</em> or <strong i="3">see <a i="4">docs</a></strong>.<br i="5"/>Done`
    fn sample() -> (String, Tagged) {
        let mut b = Builder::default();
        b.text("Run ");
        b.kept("code", "`ls`", "ls");
        b.text(" as ");
        b.open("em", "*".into());
        b.text("root");
        b.close("*".into());
        b.text(" or ");
        b.open("strong", "**".into());
        b.text("see ");
        b.open("a", "[".into());
        b.text("docs");
        b.close("](https://x)".into());
        b.close("**".into());
        b.text(".");
        b.void("br", "\\\n", "\n");
        b.text("Done");
        let (text, tagged) = b.finish();
        (text, tagged.unwrap())
    }

    fn plain_write(out: &mut String, text: &str) {
        out.push_str(text);
    }

    #[test]
    fn the_builder_writes_numbered_tags_and_the_text_without_them() {
        let (text, tagged) = sample();
        assert_eq!(
            text,
            "Run <code i=\"1\">ls</code> as <em i=\"2\">root</em> or \
             <strong i=\"3\">see <a i=\"4\">docs</a></strong>.<br i=\"5\"/>Done"
        );
        assert_eq!(tagged.plain(), "Run `ls` as root or see docs.\nDone");
    }

    #[test]
    fn the_fallback_keeps_only_the_tags_of_kept_and_void_elements() {
        let (_, tagged) = sample();
        let (text, fallback) = tagged.fallback().unwrap();
        assert_eq!(
            text,
            "Run <code i=\"1\">ls</code> as root or see docs.<br i=\"2\"/>Done"
        );
        assert_eq!(fallback.plain(), tagged.plain());
        assert!(fallback.fallback().is_none());
        assert_eq!(
            fallback
                .restore(
                    "<code i=\"1\">エルエス</code> を root として。<br i=\"2\"/>完了",
                    plain_write
                )
                .as_deref(),
            Some("`ls` を root として。\\\n完了")
        );
    }

    #[test]
    fn a_translation_without_tags_is_checked_for_every_kept_original() {
        let mut b = Builder::default();
        b.kept("code", "`a`", "a");
        b.text(" and ");
        b.kept("code", "`a`", "a");
        b.text(" or ");
        b.kept("code", "https://x", "https://x");
        let tagged = b.finish().1.unwrap();
        assert!(tagged.keeps_originals("`a` と `a`、https://x"));
        assert!(!tagged.keeps_originals("`a` と https://x"));
        assert!(!tagged.keeps_originals("`a` と `A`、https://x"));
        assert!(!tagged.keeps_originals("`a` と `a`、https://y"));
        // A URL the translation goes on with is another URL.
        assert!(!tagged.keeps_originals("`a` と `a`、https://x/ja"));
        assert!(!tagged.keeps_originals("`a` と `a`、https://x.jp"));
        assert!(!tagged.keeps_originals("`a` と `a`、xhttps://x"));
        assert!(tagged.keeps_originals("`a`と`a`、(https://x)."));
        assert!(tagged.keeps_originals("`a`s and `a`, https://x... or"));
    }

    #[test]
    fn there_is_no_fallback_that_would_repeat_a_text() {
        // Only paired elements: the fallback would be the text without tags.
        let mut b = Builder::default();
        b.open("em", "*".into());
        b.text("a");
        b.close("*".into());
        assert!(b.finish().1.unwrap().fallback().is_none());
        // Only kept elements: the fallback would be the text itself.
        let mut b = Builder::default();
        b.text("a ");
        b.kept("code", "`b`", "b");
        assert!(b.finish().1.unwrap().fallback().is_none());
    }

    #[test]
    fn a_text_without_elements_has_no_tags() {
        let mut b = Builder::default();
        b.text("Plain.");
        assert!(b.has_words());
        assert_eq!(b.finish(), ("Plain.".to_owned(), None));
    }

    #[test]
    fn letters_only_in_kept_text_are_not_words() {
        let mut b = Builder::default();
        b.text("42 ");
        b.kept("code", "`code`", "code");
        assert!(!b.has_words());
        let mut b = Builder::default();
        b.text("価格");
        assert!(b.has_words());
    }

    #[test]
    fn tags_are_put_back_as_the_original_markup_in_any_order() {
        let (_, tagged) = sample();
        let translation = "<strong i=\"3\"><a i=\"4\">文書</a>を見る</strong>か、<em i=\"2\">root</em> として \
                           <code i=\"1\">ls を</code> 実行する。<br i=\"5\" />完了";
        assert_eq!(
            tagged.restore(translation, plain_write).as_deref(),
            Some("**[文書](https://x)を見る**か、*root* として `ls` 実行する。\\\n完了")
        );
    }

    #[test]
    fn the_text_between_tags_goes_through_write() {
        let (_, tagged) = sample();
        let translation = "a <code i=\"1\">x\ny</code> <em i=\"2\">b\nc</em> <strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/>e\nf";
        let restored = tagged.restore(translation, |out, text| {
            out.push_str(&text.replace('\n', "\n> "));
        });
        assert_eq!(
            restored.as_deref(),
            Some("a `ls` *b\n> c* **[d](https://x)**\\\ne\n> f")
        );
    }

    #[test]
    fn text_that_only_looks_like_a_tag_stays() {
        let (_, tagged) = sample();
        let translation = "1 < 2 </p> <code i=\"1\"></em><b></code><em i=\"2\">r</em> \
                           <strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/><x>";
        assert_eq!(
            tagged.restore(translation, plain_write).as_deref(),
            Some("1 < 2 </p> `ls`*r* **[d](https://x)**\\\n<x>")
        );
    }

    #[test]
    fn tags_that_do_not_answer_the_elements_are_not_put_back() {
        let (_, tagged) = sample();
        let whole = "<code i=\"1\">ls</code><em i=\"2\">r</em><strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/>";
        assert!(tagged.restore(whole, plain_write).is_some());
        for broken in [
            // One left out.
            "<code i=\"1\">ls</code><em i=\"2\">r</em><strong i=\"3\">d</strong><br i=\"5\"/>",
            // One twice.
            "<code i=\"1\">ls</code><em i=\"2\">r</em><em i=\"2\">r</em><strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/>",
            // A number of another name.
            "<code i=\"1\">ls</code><strong i=\"2\">r</strong><em i=\"3\"><a i=\"4\">d</a></em><br i=\"5\"/>",
            // A number that is not there.
            "<code i=\"1\">ls</code><em i=\"2\">r</em><strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/><em i=\"6\">x</em>",
            // Crossing.
            "<code i=\"1\">ls</code><em i=\"2\">r<strong i=\"3\"></em><a i=\"4\">d</a></strong><br i=\"5\"/>",
            // Not closed.
            "<code i=\"1\">ls</code><em i=\"2\">r<strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/>",
            // A kept element not closed.
            "<code i=\"1\">ls<em i=\"2\">r</em><strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/>",
            // A void element written as a pair, and a pair as void.
            "<code i=\"1\">ls</code><em i=\"2\">r</em><strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\">x</br>",
            "<code i=\"1\">ls</code><em i=\"2\"/>r<strong i=\"3\"><a i=\"4\">d</a></strong><br i=\"5\"/>",
            // No tags at all.
            "ls r d",
        ] {
            assert_eq!(tagged.restore(broken, plain_write), None, "{broken}");
        }
    }

    #[test]
    fn hide_shows_the_text_of_a_stream_without_its_tags() {
        let mut hide = Hide::default();
        let pieces = [
            "Run <co",
            "de i=\"1",
            "\">ls</code> a",
            "s <",
            "em i=\"2\">root</em> 1 <",
            " 2 <b",
            "r i=\"5\"/>",
            "end <",
        ];
        let shown: Vec<String> = pieces.iter().map(|piece| hide.push(piece)).collect();
        assert_eq!(
            shown,
            ["Run ", "", "ls a", "s ", "root 1 ", "< 2 ", "", "end "]
        );
        // A `<` that runs too long without a `>` is text.
        assert_eq!(
            hide.push(&"x".repeat(LONGEST_TAG)),
            format!("<{}", "x".repeat(LONGEST_TAG))
        );
    }
}
