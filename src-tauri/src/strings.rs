//! Apple .strings の翻訳単位 (doc-1 §7.1).
//!
//! A .strings file is a list of `"key" = "value";` entries with comments between them, read as
//! Core Foundation reads an old-style property list's dictionary without its braces. A 翻訳単位
//! is the content of one quoted value, taken as `values.rs` takes a resource's; keys, comments,
//! `=`, `;` and the whitespace between them are copied from the original. A value written without
//! quotes (`CFBundleName = MyApp;`) is a name, and stays as it is.

use std::ops::Range;

use crate::document::{Lines, Segment};
use crate::values::{Units, Value};

/// `text` as the 翻訳単位 of its quoted values and the original between them, or why `text` is
/// not a .strings file. The values of the keys named in `untranslated_keys` stay as they are.
pub fn segments<'a>(
    text: &'a str,
    untranslated_keys: &[String],
) -> Result<Vec<Segment<'a>>, String> {
    let mut units = Units::new(text);
    for entry in entries(text)? {
        let Some(value) = entry.value else {
            continue;
        };
        let Some(quote) = value.quote else {
            continue;
        };
        if untranslated_keys.contains(&decode(text, &entry.key)) {
            continue;
        }
        let chars = escaped_chars(&text[value.content.clone()]);
        let value_text = Value::new(text, value.content, chars);
        units.value(&value_text, &Lines::Strings(quote));
    }
    Ok(units.finish())
}

/// Whether `text` reads as a .strings file: its entries read, and one of them has a quoted value.
/// A text of names alone (`a = b;`) is more likely code than a .strings file.
pub fn is_strings(text: &str) -> bool {
    entries(text).is_ok_and(|entries| {
        entries.iter().any(|entry| {
            entry
                .value
                .as_ref()
                .is_some_and(|value| value.quote.is_some())
        })
    })
}

/// Adds `c` to `out` as a .strings string in `quote`s writes it. A line break is written as `\n`,
/// as Xcode writes it, rather than as a line break inside the quotes, which reads the same.
pub fn escape(out: &mut String, c: char, quote: char) {
    match c {
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if c == quote => {
            out.push('\\');
            out.push(c);
        }
        c if c < ' ' => out.push_str(&format!("\\U{:04x}", u32::from(c))),
        c => out.push(c),
    }
}

/// A string as the file writes it: its content, inside the quotes when it has them.
struct Written {
    content: Range<usize>,
    /// `"` or `'`, or `None` for a name written without quotes.
    quote: Option<char>,
}

struct Entry {
    key: Written,
    /// `None` for an entry of a key alone (`"key";`), whose value is the key.
    value: Option<Written>,
}

fn entries(text: &str) -> Result<Vec<Entry>, String> {
    let mut parser = Parser { text, at: 0 };
    let mut entries = Vec::new();
    loop {
        parser.skip()?;
        if parser.at == text.len() {
            return Ok(entries);
        }
        let key = parser.string()?;
        parser.skip()?;
        let value = if parser.eat('=') {
            parser.skip()?;
            let value = parser.string()?;
            parser.skip()?;
            Some(value)
        } else {
            None
        };
        if !parser.eat(';') {
            return Err(parser.error("expected `;`"));
        }
        entries.push(Entry { key, value });
    }
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.peek() == Some(c);
        if found {
            self.at += c.len_utf8();
        }
        found
    }

    fn error(&self, reason: &str) -> String {
        let line = self.text[..self.at].matches('\n').count() + 1;
        format!("not .strings: line {line}: {reason}")
    }

    /// Skips whitespace, a byte order mark and comments (`// …` to the end of the line, `/* … */`).
    fn skip(&mut self) -> Result<(), String> {
        loop {
            let rest = &self.text[self.at..];
            if let Some(c) = rest
                .chars()
                .next()
                .filter(|&c| c.is_whitespace() || c == '\u{feff}')
            {
                self.at += c.len_utf8();
            } else if rest.starts_with("//") {
                self.at += rest.find('\n').unwrap_or(rest.len());
            } else if let Some(comment) = rest.strip_prefix("/*") {
                let Some(end) = comment.find("*/") else {
                    return Err(self.error("a comment is not closed"));
                };
                self.at += 2 + end + 2;
            } else {
                return Ok(());
            }
        }
    }

    /// Reads a quoted string, or a name of the characters Core Foundation allows without quotes.
    fn string(&mut self) -> Result<Written, String> {
        let start = self.at;
        match self.peek() {
            Some(quote @ ('"' | '\'')) => {
                self.at += 1;
                let content = self.at;
                loop {
                    match self.peek() {
                        None => {
                            self.at = start;
                            return Err(self.error("a string is not closed"));
                        }
                        Some('\\') => {
                            self.at += 1;
                            if let Some(c) = self.peek() {
                                self.at += c.len_utf8();
                            }
                        }
                        Some(c) if c == quote => break,
                        Some(c) => self.at += c.len_utf8(),
                    }
                }
                let written = Written {
                    content: content..self.at,
                    quote: Some(quote),
                };
                self.at += 1;
                Ok(written)
            }
            _ => {
                let rest = &self.text[self.at..];
                let len = rest.find(|c: char| !is_name_char(c)).unwrap_or(rest.len());
                if len == 0 {
                    return Err(self.error("expected a string"));
                }
                self.at += len;
                Ok(Written {
                    content: start..self.at,
                    quote: None,
                })
            }
        }
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '/' | ':' | '.' | '-')
}

fn decode(text: &str, written: &Written) -> String {
    let raw = &text[written.content.clone()];
    if written.quote.is_none() {
        return raw.to_owned();
    }
    escaped_chars(raw).into_iter().map(|(_, c)| c).collect()
}

/// The characters of a quoted string's content as a .strings file writes them, each with where it
/// is written. Core Foundation reads `\U` and up to four hex digits as a UTF-16 code unit, a
/// backslash and up to three octal digits as a byte, the C escapes as C does, and a backslash before
/// any other character as that character.
fn escaped_chars(raw: &str) -> Vec<(Range<usize>, char)> {
    let mut chars = Vec::new();
    let mut at = 0;
    while let Some(c) = raw[at..].chars().next() {
        if c != '\\' {
            chars.push((at..at + c.len_utf8(), c));
            at += c.len_utf8();
            continue;
        }
        let Some(next) = raw[at + 1..].chars().next() else {
            chars.push((at..at + 1, '\\'));
            break;
        };
        let (c, len) = match next {
            'a' => ('\u{7}', 2),
            'b' => ('\u{8}', 2),
            'f' => ('\u{c}', 2),
            'n' => ('\n', 2),
            'r' => ('\r', 2),
            't' => ('\t', 2),
            'v' => ('\u{b}', 2),
            'U' | 'u' => unicode_escape(&raw[at..]),
            '0'..='7' => {
                let digits = raw[at + 1..]
                    .bytes()
                    .take(3)
                    .take_while(|b| (b'0'..=b'7').contains(b))
                    .count();
                let byte = u32::from_str_radix(&raw[at + 1..at + 1 + digits], 8).unwrap_or(0);
                // A byte past ASCII is NeXTSTEP's encoding, which Latin-1 stands in for: it only
                // goes to the engine, and the original is written back as it is.
                (
                    char::from_u32(byte).unwrap_or(char::REPLACEMENT_CHARACTER),
                    1 + digits,
                )
            }
            other => (other, 1 + other.len_utf8()),
        };
        chars.push((at..at + len, c));
        at += len;
    }
    chars
}

/// The character of a `\U` escape at the start of `raw`, and how many bytes it takes: two escapes
/// for a surrogate pair.
fn unicode_escape(raw: &str) -> (char, usize) {
    let unit = |at: usize| {
        let digits = raw[at..]
            .bytes()
            .take(4)
            .take_while(u8::is_ascii_hexdigit)
            .count();
        let value = u32::from_str_radix(&raw[at..at + digits], 16).ok()?;
        Some((value, digits))
    };
    let Some((high, digits)) = unit(2) else {
        return ('U', 2);
    };
    let len = 2 + digits;
    if (0xD800..0xDC00).contains(&high)
        && matches!(raw.get(len..len + 2), Some("\\U" | "\\u"))
        && let Some((low @ 0xDC00..0xE000, low_digits)) = unit(len + 2)
    {
        let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
        return (
            char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER),
            len + 2 + low_digits,
        );
    }
    (
        char::from_u32(high).unwrap_or(char::REPLACEMENT_CHARACTER),
        len,
    )
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
                Segment::Unit(unit) => Some(format!("[{}]", unit.text)),
            })
            .collect()
    }

    fn the_unit(text: &str) -> crate::document::Unit<'_> {
        segments(text, &[])
            .unwrap()
            .into_iter()
            .find_map(|segment| match segment {
                Segment::Unit(unit) => Some(unit),
                Segment::Kept(_) => None,
            })
            .unwrap()
    }

    #[test]
    fn quoted_values_are_units_and_keys_and_comments_are_kept() {
        let text = "/* The title of the main window. */\n\"window.title\" = \"Tiny Translator\";\n\n// Shown on the button.\n\"Translate\" = \"Translate\";\n";
        assert_eq!(
            show(text, &[]),
            [
                "=/* The title of the main window. */\n\"window.title\" = \"",
                "[Tiny Translator]",
                "=\";\n\n// Shown on the button.\n\"Translate\" = \"",
                "[Translate]",
                "=\";\n",
            ]
        );
    }

    #[test]
    fn layout_and_single_quotes_and_names_are_read() {
        // `a/*x*/` would be the name `a/`, as Core Foundation reads it too.
        let text = "\u{feff}a /*x*/=/*y*/'Single';\"b\"\t=\n\"Two\nlines\" ; CFBundleName = MyApp;\n\"only.key\";";
        assert_eq!(
            show(text, &[]),
            [
                "=\u{feff}a /*x*/=/*y*/'",
                "[Single]",
                "=';\"b\"\t=\n\"",
                "[Two\nlines]",
                "=\" ; CFBundleName = MyApp;\n\"only.key\";",
            ]
        );
    }

    #[test]
    fn a_text_that_is_not_strings_is_refused() {
        for (text, line) in [
            ("\"a\" = \"b\"", 1),
            ("\"a\" = \"b\";\n\"c\" = \"d", 2),
            ("\"a\" = ;", 1),
            ("/* open\n\"a\" = \"b\";", 1),
            ("{\"a\" = \"b\";}", 1),
            ("let x = 1;", 1),
        ] {
            let err = segments(text, &[])
                .err()
                .unwrap_or_else(|| panic!("{text:?}"));
            assert!(
                err.starts_with(&format!("not .strings: line {line}: ")),
                "{text:?}: {err}"
            );
        }
    }

    #[test]
    fn an_empty_text_is_a_strings_file_without_entries_and_not_told_as_one() {
        assert_eq!(show("", &[]), Vec::<String>::new());
        assert!(!is_strings(""));
        assert!(!is_strings("/* nothing */\n"));
        assert!(!is_strings("a = b;"));
        assert!(is_strings("\"a\" = \"b\";"));
        assert!(!is_strings("Hello, world."));
        assert!(!is_strings("x = \"a\"; y"));
    }

    #[test]
    fn escapes_are_undone_for_the_engine_and_the_original_kept_as_written() {
        let unit =
            the_unit(r#""k" = "Say \"hi\"\n\tto \U00e9t\u00E9 \UD83D\UDE00 \101 \' \\ back";"#);
        assert_eq!(unit.text, "Say \"hi\"\n\tto été 😀 A ' \\ back");
        assert_eq!(
            unit.original,
            r#"Say \"hi\"\n\tto \U00e9t\u00E9 \UD83D\UDE00 \101 \' \\ back"#
        );
    }

    #[test]
    fn placeholders_go_as_kept_elements() {
        assert_eq!(
            show(r#""files" = "%1$@ has %lld files, %d%% done";"#, &[]),
            [
                "=\"files\" = \"",
                "[<code i=\"1\">%1$@</code> has <code i=\"2\">%lld</code> files, <code i=\"3\">%d</code><code i=\"4\">%%</code> done]",
                "=\";",
            ]
        );
    }

    #[test]
    fn values_without_letters_and_untranslated_keys_are_kept() {
        let text = "\"a\" = \"\";\n\"b\" = \"%@\";\n\"c\" = \"42\";\n\"id\" = \"home\";\n\"i\\U0064s\" = \"x\";\n\"d\" = \"Sent\";";
        assert_eq!(
            show(text, &["id", "ids"]),
            [
                "=\"a\" = \"\";\n\"b\" = \"%@\";\n\"c\" = \"42\";\n\"id\" = \"home\";\n\"i\\U0064s\" = \"x\";\n\"d\" = \"",
                "[Sent]",
                "=\";",
            ]
        );
    }

    #[test]
    fn whitespace_around_a_value_stays_outside_the_unit() {
        assert_eq!(
            show(r#""a" = "Name: "; "b" = "\n Two words\t";"#, &[]),
            [
                "=\"a\" = \"",
                "[Name:]",
                "= \"; \"b\" = \"\\n ",
                "[Two words]",
                "=\\t\";"
            ]
        );
    }

    #[test]
    fn escaping_writes_what_a_strings_reader_reads_back() {
        let original = "quote \" apostrophe ' back \\ line\nreturn\r tab\t bell\u{7} 日本語 😀";
        for quote in ['"', '\''] {
            let mut out = format!("{{\"k\" = {quote}");
            for c in original.chars() {
                escape(&mut out, c, quote);
            }
            out.push_str(&format!("{quote};}}"));
            let read: plist::Dictionary = plist::from_reader_ascii(out.as_bytes()).unwrap();
            assert_eq!(
                read.get("k").and_then(plist::Value::as_string),
                Some(original),
                "{out}"
            );
        }
    }
}
