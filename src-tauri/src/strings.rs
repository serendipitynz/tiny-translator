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
/// is written, read as Core Foundation reads them, since a key is matched against 翻訳しないキー by
/// what it reads as: `\U` and up to four hex digits as a UTF-16 code unit, a backslash and up to
/// three octal digits as a NeXTSTEP byte, the C escapes as C does, and a backslash before any other
/// character — `u` included — as that character.
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
            'U' => unicode_escape(&raw[at..]),
            '0'..='7' => {
                let digits = raw[at + 1..]
                    .bytes()
                    .take(3)
                    .take_while(|b| (b'0'..=b'7').contains(b))
                    .count();
                let value = u32::from_str_radix(&raw[at + 1..at + 1 + digits], 8).unwrap_or(0);
                // `\400` and above wrap round to a byte, as Core Foundation's reader does.
                (next_step((value & 0xFF) as u8), 1 + digits)
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
    // A `\U` without digits is NUL to Core Foundation.
    let Some((high, digits)) = unit(2) else {
        return ('\0', 2);
    };
    let len = 2 + digits;
    if (0xD800..0xDC00).contains(&high)
        && raw.get(len..len + 2) == Some("\\U")
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

/// The character NeXTSTEP's encoding writes as `byte`, after Unicode's mapping
/// (MAPPINGS/VENDORS/NEXT/NEXTSTEP.TXT). Core Foundation reads 0xFE and 0xFF, which the encoding
/// leaves undefined, as NUL.
fn next_step(byte: u8) -> char {
    const HIGH: [char; 126] = [
        '\u{A0}', '\u{C0}', '\u{C1}', '\u{C2}', '\u{C3}', '\u{C4}', '\u{C5}', '\u{C7}', '\u{C8}',
        '\u{C9}', '\u{CA}', '\u{CB}', '\u{CC}', '\u{CD}', '\u{CE}', '\u{CF}', '\u{D0}', '\u{D1}',
        '\u{D2}', '\u{D3}', '\u{D4}', '\u{D5}', '\u{D6}', '\u{D9}', '\u{DA}', '\u{DB}', '\u{DC}',
        '\u{DD}', '\u{DE}', '\u{B5}', '\u{D7}', '\u{F7}', '\u{A9}', '\u{A1}', '\u{A2}', '\u{A3}',
        '\u{2044}', '\u{A5}', '\u{192}', '\u{A7}', '\u{A4}', '\u{2019}', '\u{201C}', '\u{AB}',
        '\u{2039}', '\u{203A}', '\u{FB01}', '\u{FB02}', '\u{AE}', '\u{2013}', '\u{2020}',
        '\u{2021}', '\u{B7}', '\u{A6}', '\u{B6}', '\u{2022}', '\u{201A}', '\u{201E}', '\u{201D}',
        '\u{BB}', '\u{2026}', '\u{2030}', '\u{AC}', '\u{BF}', '\u{B9}', '\u{2CB}', '\u{B4}',
        '\u{2C6}', '\u{2DC}', '\u{AF}', '\u{2D8}', '\u{2D9}', '\u{A8}', '\u{B2}', '\u{2DA}',
        '\u{B8}', '\u{B3}', '\u{2DD}', '\u{2DB}', '\u{2C7}', '\u{2014}', '\u{B1}', '\u{BC}',
        '\u{BD}', '\u{BE}', '\u{E0}', '\u{E1}', '\u{E2}', '\u{E3}', '\u{E4}', '\u{E5}', '\u{E7}',
        '\u{E8}', '\u{E9}', '\u{EA}', '\u{EB}', '\u{EC}', '\u{C6}', '\u{ED}', '\u{AA}', '\u{EE}',
        '\u{EF}', '\u{F0}', '\u{F1}', '\u{141}', '\u{D8}', '\u{152}', '\u{BA}', '\u{F2}', '\u{F3}',
        '\u{F4}', '\u{F5}', '\u{F6}', '\u{E6}', '\u{F9}', '\u{FA}', '\u{FB}', '\u{131}', '\u{FC}',
        '\u{FD}', '\u{142}', '\u{F8}', '\u{153}', '\u{DF}', '\u{FE}', '\u{FF}',
    ];
    match byte {
        0..0x80 => char::from(byte),
        0xFE | 0xFF => '\0',
        _ => HIGH[usize::from(byte - 0x80)],
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
        let unit = the_unit(
            r#""k" = "Say \"hi\"\n\tto \U00e9t\U00E9 \UD83D\UDE00 \101 \351 \u0041 \' \\ back";"#,
        );
        // `\351` is NeXTSTEP's Ø, and `\u` is no escape to Core Foundation.
        assert_eq!(unit.text, "Say \"hi\"\n\tto été 😀 A Ø u0041 ' \\ back");
        assert_eq!(
            unit.original,
            r#"Say \"hi\"\n\tto \U00e9t\U00E9 \UD83D\UDE00 \101 \351 \u0041 \' \\ back"#
        );
    }

    /// Every octal byte, and `\U` without digits, as Apple's own reader reads them.
    #[cfg(target_os = "macos")]
    #[test]
    fn escapes_are_read_as_plutil_reads_them() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut written: Vec<String> = (0..=0o777).map(|byte| format!("\\{byte:o}x")).collect();
        for raw in [r"\U", r"\Ux", r"\U0041", r"\u0041", r"\UD83D\UDE00"] {
            written.push(raw.to_owned());
        }
        let file: String = written
            .iter()
            .enumerate()
            .map(|(i, raw)| format!("\"k{i}\" = \"{raw}\";\n"))
            .collect();
        let mut child = Command::new("plutil")
            .args(["-convert", "json", "-o", "-", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(file.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        let read: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        for (i, raw) in written.iter().enumerate() {
            let ours: String = escaped_chars(raw).into_iter().map(|(_, c)| c).collect();
            assert_eq!(read[format!("k{i}")].as_str(), Some(ours.as_str()), "{raw}");
        }
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
        // Core Foundation reads `caf\351` as `cafØ`, and `i\u0064` as `iu0064`, not `id`.
        let text = "\"a\" = \"\";\n\"b\" = \"%@\";\n\"c\" = \"42\";\n\"id\" = \"home\";\n\"i\\U0064s\" = \"x\";\n\"caf\\351\" = \"Kept\";\n\"i\\u0064\" = \"Sent\";";
        assert_eq!(
            show(text, &["id", "ids", "cafØ"]),
            [
                "=\"a\" = \"\";\n\"b\" = \"%@\";\n\"c\" = \"42\";\n\"id\" = \"home\";\n\"i\\U0064s\" = \"x\";\n\"caf\\351\" = \"Kept\";\n\"i\\u0064\" = \"",
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
