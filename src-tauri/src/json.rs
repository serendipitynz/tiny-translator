//! JSON の翻訳単位 (doc-1 §7.1).
//!
//! A 翻訳単位 is the content of one string value. Everything else — keys, numbers, `true`,
//! `false`, `null`, the punctuation and the whitespace between them — is copied from the original,
//! so the translation keeps the original's structure, key order and layout exactly. The value's
//! escapes are undone before it goes to the engine, and the translation is escaped again as it is
//! written back.

use std::borrow::Cow;
use std::ops::Range;

use crate::document::{Lines, Segment, Unit};

/// `text` as the 翻訳単位 of its string values and the original between them, or why `text` is not
/// JSON. The values under a key named in `untranslated_keys` stay as they are, at any depth and
/// however deep inside that value (2026-11-09 オーナー判断: the key's name, not its path).
pub fn segments<'a>(
    text: &'a str,
    untranslated_keys: &[String],
) -> Result<Vec<Segment<'a>>, String> {
    // Checked by serde_json, which also bounds the nesting, so the scan below can take the text
    // as well-formed and recurse without running out of stack.
    serde_json::from_str::<serde_json::Value>(text).map_err(|err| format!("not JSON: {err}"))?;
    let mut scan = Scan {
        text,
        at: 0,
        untranslated_keys,
        segments: Vec::new(),
        kept_from: 0,
    };
    scan.value(false);
    scan.segments.push(Segment::Kept(&text[scan.kept_from..]));
    Ok(scan.segments)
}

struct Scan<'a, 'k> {
    text: &'a str,
    at: usize,
    untranslated_keys: &'k [String],
    segments: Vec<Segment<'a>>,
    /// Where the original not yet in `segments` starts.
    kept_from: usize,
}

impl<'a> Scan<'a, '_> {
    fn byte(&self) -> u8 {
        self.text.as_bytes()[self.at]
    }

    fn skip_whitespace(&mut self) {
        while self.at < self.text.len() && matches!(self.byte(), b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }
    }

    /// Reads one value, taking its strings as 翻訳単位 unless `untranslated`.
    fn value(&mut self, untranslated: bool) {
        self.skip_whitespace();
        match self.byte() {
            b'{' => self.object(untranslated),
            b'[' => self.array(untranslated),
            b'"' => {
                let content = self.string();
                if !untranslated {
                    self.unit(content);
                }
            }
            _ => {
                while self.at < self.text.len()
                    && !matches!(
                        self.byte(),
                        b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r'
                    )
                {
                    self.at += 1;
                }
            }
        }
    }

    fn object(&mut self, untranslated: bool) {
        self.at += 1;
        self.skip_whitespace();
        if self.byte() == b'}' {
            self.at += 1;
            return;
        }
        loop {
            self.skip_whitespace();
            let key = decode(&self.text[self.string()]);
            self.skip_whitespace();
            self.at += 1; // `:`
            let named = self.untranslated_keys.contains(&key);
            self.value(untranslated || named);
            self.skip_whitespace();
            let end = self.byte();
            self.at += 1;
            if end == b'}' {
                return;
            }
        }
    }

    fn array(&mut self, untranslated: bool) {
        self.at += 1;
        self.skip_whitespace();
        if self.byte() == b']' {
            self.at += 1;
            return;
        }
        loop {
            self.value(untranslated);
            self.skip_whitespace();
            let end = self.byte();
            self.at += 1;
            if end == b']' {
                return;
            }
        }
    }

    /// Reads a string and answers where its content lies, inside the quotes.
    fn string(&mut self) -> Range<usize> {
        self.at += 1;
        let start = self.at;
        loop {
            match self.byte() {
                b'\\' => self.at += 2,
                b'"' => break,
                _ => self.at += 1,
            }
        }
        self.at += 1;
        start..self.at - 1
    }

    /// Takes the string content at `range` as a 翻訳単位, leaving out the whitespace around it,
    /// which an i18n message may need (`"Name: "`) and the engine would drop. A value without
    /// letters (`""`, `"42"`, `"—"`) is not sent, as a Markdown block without them is not.
    fn unit(&mut self, range: Range<usize>) {
        let raw = &self.text[range.clone()];
        let chars = escaped_chars(raw);
        let Some(first) = chars.iter().position(|(_, c)| !c.is_whitespace()) else {
            return;
        };
        let last = chars
            .iter()
            .rposition(|(_, c)| !c.is_whitespace())
            .unwrap_or(first);
        let start = range.start + chars[first].0.start;
        let end = range.start + chars[last].0.end;
        let content = &chars[first..=last];
        if !content.iter().any(|(_, c)| c.is_alphabetic()) {
            return;
        }
        let original = &self.text[start..end];
        let text = if original.contains('\\') {
            Cow::Owned(content.iter().map(|(_, c)| c).collect())
        } else {
            Cow::Borrowed(original)
        };
        self.segments
            .push(Segment::Kept(&self.text[self.kept_from..start]));
        self.segments.push(Segment::Unit(Unit {
            text,
            original,
            tags: None,
            lines: Lines::Json,
            attributes: Vec::new(),
        }));
        self.kept_from = end;
    }
}

/// The characters of a string's content as JSON writes them, each with where it is written.
fn escaped_chars(raw: &str) -> Vec<(Range<usize>, char)> {
    let mut chars = Vec::new();
    let mut at = 0;
    while let Some(c) = raw[at..].chars().next() {
        if c != '\\' {
            chars.push((at..at + c.len_utf8(), c));
            at += c.len_utf8();
            continue;
        }
        let (c, len) = match raw.as_bytes()[at + 1] {
            b'n' => ('\n', 2),
            b't' => ('\t', 2),
            b'r' => ('\r', 2),
            b'b' => ('\u{8}', 2),
            b'f' => ('\u{c}', 2),
            b'u' => unicode_escape(&raw[at..]),
            other => (char::from(other), 2),
        };
        chars.push((at..at + len, c));
        at += len;
    }
    chars
}

/// The character of a `\uXXXX` escape at the start of `raw`, and how many bytes it takes: twelve
/// for a surrogate pair. serde_json has already refused a lone surrogate.
fn unicode_escape(raw: &str) -> (char, usize) {
    let hex = |at: usize| {
        raw.get(at..at + 4)
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
    };
    let Some(high) = hex(2) else {
        return (char::REPLACEMENT_CHARACTER, 2);
    };
    if (0xD800..0xDC00).contains(&high)
        && raw.get(6..8) == Some("\\u")
        && let Some(low @ 0xDC00..0xE000) = hex(8)
    {
        let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
        return (
            char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER),
            12,
        );
    }
    (
        char::from_u32(high).unwrap_or(char::REPLACEMENT_CHARACTER),
        6,
    )
}

fn decode(raw: &str) -> String {
    escaped_chars(raw).into_iter().map(|(_, c)| c).collect()
}

/// Adds `c` to `out` as a JSON string's content writes it.
pub fn escape(out: &mut String, c: char) {
    match c {
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if c < ' ' => out.push_str(&format!("\\u{:04x}", u32::from(c))),
        c => out.push(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each segment as `=kept` or `[unit]`, for comparing at a glance.
    fn show(text: &str, untranslated_keys: &[&str]) -> Vec<String> {
        let keys: Vec<String> = untranslated_keys
            .iter()
            .map(|&key| key.to_owned())
            .collect();
        segments(text, &keys)
            .unwrap()
            .into_iter()
            .filter_map(|segment| match segment {
                Segment::Kept("") => None,
                Segment::Kept(kept) => Some(format!("={kept}")),
                Segment::Unit(unit) => {
                    assert_eq!(unit.lines, Lines::Json);
                    Some(format!("[{}]", unit.text))
                }
            })
            .collect()
    }

    #[test]
    fn string_values_are_units_and_the_rest_is_kept() {
        let text = "{\n  \"title\": \"Hello\",\n  \"count\": 3,\n  \"ratio\": -1.5e3,\n  \"on\": true,\n  \"off\": false,\n  \"none\": null,\n  \"list\": [\"one\", 2, \"two\"],\n  \"nested\": {\"deep\": {\"text\": \"Deep\"}}\n}\n";
        assert_eq!(
            show(text, &[]),
            [
                "={\n  \"title\": \"",
                "[Hello]",
                "=\",\n  \"count\": 3,\n  \"ratio\": -1.5e3,\n  \"on\": true,\n  \"off\": false,\n  \"none\": null,\n  \"list\": [\"",
                "[one]",
                "=\", 2, \"",
                "[two]",
                "=\"],\n  \"nested\": {\"deep\": {\"text\": \"",
                "[Deep]",
                "=\"}}\n}\n",
            ]
        );
    }

    #[test]
    fn a_text_that_is_not_json_is_refused() {
        for text in [
            "",
            "{\"a\": }",
            "{\"a\": \"b\",}",
            "// comment\n{}",
            "{} {}",
            "'a'",
        ] {
            let err = segments(text, &[]).unwrap_err();
            assert!(err.starts_with("not JSON: "), "{text:?}: {err}");
        }
    }

    #[test]
    fn a_top_level_string_or_scalar_is_read() {
        assert_eq!(show(" \"Hello\" ", &[]), ["= \"", "[Hello]", "=\" "]);
        assert_eq!(show("42", &[]), ["=42"]);
        assert_eq!(show("[]", &[]), ["=[]"]);
        assert_eq!(show("{ }", &[]), ["={ }"]);
    }

    #[test]
    fn escapes_are_undone_for_the_engine_and_the_original_kept_as_written() {
        let keys = Vec::new();
        let text = r#"{"a": "Say \"hi\"\n\tto \u00e9t\u00E9 and \ud83d\ude00 \/ \\ back"}"#;
        let unit = segments(text, &keys)
            .unwrap()
            .into_iter()
            .find_map(|segment| match segment {
                Segment::Unit(unit) => Some(unit),
                Segment::Kept(_) => None,
            })
            .unwrap();
        assert_eq!(unit.text, "Say \"hi\"\n\tto été and 😀 / \\ back");
        assert_eq!(
            unit.original,
            r#"Say \"hi\"\n\tto \u00e9t\u00E9 and \ud83d\ude00 \/ \\ back"#
        );
    }

    #[test]
    fn whitespace_around_a_value_stays_outside_the_unit() {
        assert_eq!(
            show(r#"["Name: ", "  Two words\n", "\n\t Tab \u0020"]"#, &[]),
            [
                "=[\"",
                "[Name:]",
                "= \", \"  ",
                "[Two words]",
                "=\\n\", \"\\n\\t ",
                "[Tab]",
                "= \\u0020\"]",
            ]
        );
    }

    #[test]
    fn values_without_letters_are_not_sent() {
        assert_eq!(
            show(
                r#"{"a": "", "b": "  ", "c": "42", "d": "—", "e": "%s", "f": "x"}"#,
                &[]
            ),
            [
                "={\"a\": \"\", \"b\": \"  \", \"c\": \"42\", \"d\": \"—\", \"e\": \"",
                "[%s]",
                "=\", \"f\": \"",
                "[x]",
                "=\"}",
            ]
        );
    }

    #[test]
    fn keys_are_never_units() {
        assert_eq!(
            show(
                r#"{"Hello world": "Hi", "Key \"quoted\"": {"Inner key": 1}}"#,
                &[]
            ),
            [
                "={\"Hello world\": \"",
                "[Hi]",
                "=\", \"Key \\\"quoted\\\"\": {\"Inner key\": 1}}",
            ]
        );
    }

    #[test]
    fn untranslated_keys_keep_their_values_at_any_depth() {
        let text = r#"{"id": "home", "title": "Home", "meta": {"id": "x", "note": "Note"}, "list": [{"id": "y", "label": "Label"}]}"#;
        assert_eq!(
            show(text, &["id"]),
            [
                "={\"id\": \"home\", \"title\": \"",
                "[Home]",
                "=\", \"meta\": {\"id\": \"x\", \"note\": \"",
                "[Note]",
                "=\"}, \"list\": [{\"id\": \"y\", \"label\": \"",
                "[Label]",
                "=\"}]}",
            ]
        );
    }

    #[test]
    fn an_untranslated_key_keeps_everything_inside_its_value() {
        let text =
            r#"{"code": {"text": "Kept", "more": ["Kept too", {"deep": "Kept"}]}, "text": "Sent"}"#;
        assert_eq!(
            show(text, &["code"]),
            [
                "={\"code\": {\"text\": \"Kept\", \"more\": [\"Kept too\", {\"deep\": \"Kept\"}]}, \"text\": \"",
                "[Sent]",
                "=\"}",
            ]
        );
    }

    #[test]
    fn an_untranslated_key_is_matched_by_its_unescaped_name_and_exactly() {
        let text = r#"{"i\u0064": "Kept", "ID": "Sent", "ids": "Sent", "a.id": "Sent"}"#;
        assert_eq!(
            show(text, &["id"]),
            [
                "={\"i\\u0064\": \"Kept\", \"ID\": \"",
                "[Sent]",
                "=\", \"ids\": \"",
                "[Sent]",
                "=\", \"a.id\": \"",
                "[Sent]",
                "=\"}",
            ]
        );
    }

    #[test]
    fn escaping_writes_what_json_reads_back() {
        let original = "quote \" back \\ line\nreturn\r tab\t bell\u{7} 日本語 😀 / \u{2028}";
        let mut out = String::from("\"");
        for c in original.chars() {
            escape(&mut out, c);
        }
        out.push('"');
        assert_eq!(
            out,
            "\"quote \\\" back \\\\ line\\nreturn\\r tab\\t bell\\u0007 日本語 😀 / \u{2028}\""
        );
        assert_eq!(serde_json::from_str::<String>(&out).unwrap(), original);
    }
}
