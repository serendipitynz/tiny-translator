//! プレースホルダー in the strings of an i18n resource (doc-1 §7.1, JSON).
//!
//! An app fills a placeholder in at run time, so a translation must hold each one exactly as the
//! original writes it. Three families are recognised, each written as the libraries that read them
//! write it:
//! - one pair of braces: `{name}`, `{0}`, `{0:.2f}`, `{n, number}`, `{}`, and with `$` or `%`
//!   before it, `${name}`, `%{name}`;
//! - two pairs of braces: `{{count}}`, `{{- name}}`, `{{count, number}}`;
//! - printf: `%s`, `%d`, `%1$s`, `%@`, `%.2f`, `%ld`, `%(name)s`, `%%`. A space is not read as a
//!   flag, so the `% o` of "50% off" is text.
//!
//! An ICU argument that chooses a message (`{n, plural, one {# item} other {# items}}`, `select`,
//! `selectordinal`) holds text to translate: its markup stays as it is, and each of its messages is
//! translated on its own (2026-11-11 オーナー判断). Inside the messages of `plural` and
//! `selectordinal`, `#` stands for the number and is a placeholder too.

use std::ops::Range;

/// A part of a message to translate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(Range<usize>),
    Placeholder(Range<usize>),
}

/// The stretches of `text` to translate, in order, each as its text and placeholders. What lies
/// between them is the markup of ICU arguments that choose a message, which stays as it is.
pub fn messages(text: &str) -> Vec<Vec<Inline>> {
    let (nodes, _) = message(text, 0, false, false);
    let mut out = Vec::new();
    flatten(nodes, &mut out);
    out
}

#[derive(Debug)]
enum Node {
    Inline(Inline),
    /// The messages of an ICU argument that chooses one.
    Choice(Vec<Vec<Node>>),
}

fn flatten(nodes: Vec<Node>, out: &mut Vec<Vec<Inline>>) {
    let mut current = Vec::new();
    for node in nodes {
        match node {
            Node::Inline(inline) => current.push(inline),
            Node::Choice(messages) => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                for message in messages {
                    flatten(message, out);
                }
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
}

/// Reads a message from `at` to the end of `text`, or when `nested` to the `}` that closes it, and
/// answers its parts and where it stopped. `pound` reads `#` as a placeholder.
fn message(text: &str, mut at: usize, pound: bool, nested: bool) -> (Vec<Node>, usize) {
    let mut nodes: Vec<Node> = Vec::new();
    while let Some(c) = text[at..].chars().next() {
        let found = match c {
            '}' if nested => break,
            '{' => {
                if let Some((messages, end)) = choice(text, at, pound) {
                    nodes.push(Node::Choice(messages));
                    at = end;
                    continue;
                }
                double_braces(text, at).or_else(|| braces(text, at))
            }
            '$' => text[at + 1..]
                .starts_with('{')
                .then(|| braces(text, at + 1))
                .flatten(),
            '%' => {
                if text[at + 1..].starts_with('{') {
                    braces(text, at + 1)
                } else {
                    printf(text, at)
                }
            }
            '#' if pound => Some(at + 1),
            _ => None,
        };
        match found {
            Some(end) => {
                nodes.push(Node::Inline(Inline::Placeholder(at..end)));
                at = end;
            }
            None => {
                let end = at + c.len_utf8();
                match nodes.last_mut() {
                    Some(Node::Inline(Inline::Text(range))) if range.end == at => range.end = end,
                    _ => nodes.push(Node::Inline(Inline::Text(at..end))),
                }
                at = end;
            }
        }
    }
    (nodes, at)
}

/// The ICU argument that chooses a message at `at` (a `{`), as its messages and where it ends.
fn choice(text: &str, at: usize, pound: bool) -> Option<(Vec<Vec<Node>>, usize)> {
    let mut p = skip_spaces(text, at + 1);
    p = identifier(text, p)?;
    p = skip_spaces(text, p);
    p = expect(text, p, ",")?;
    p = skip_spaces(text, p);
    let start = p;
    p = identifier(text, p)?;
    let pound = match &text[start..p] {
        "plural" | "selectordinal" => true,
        "select" => pound,
        _ => return None,
    };
    p = skip_spaces(text, p);
    p = expect(text, p, ",")?;
    p = skip_spaces(text, p);
    if pound && let Some(after) = text[p..].strip_prefix("offset:") {
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        if digits == 0 {
            return None;
        }
        p = skip_spaces(text, p + "offset:".len() + digits);
    }
    let mut messages = Vec::new();
    loop {
        p = skip_spaces(text, p);
        if text[p..].starts_with('}') {
            return (!messages.is_empty()).then_some((messages, p + 1));
        }
        p = match text[p..].strip_prefix('=') {
            Some(after) => {
                let digits =
                    after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
                (digits > 0).then_some(p + 1 + digits)?
            }
            None => identifier(text, p)?,
        };
        p = skip_spaces(text, p);
        p = expect(text, p, "{")?;
        let (nodes, end) = message(text, p, pound, true);
        p = expect(text, end, "}")?;
        messages.push(nodes);
    }
}

/// `{{…}}` at `at`, with something other than braces and line breaks inside: where it ends.
fn double_braces(text: &str, at: usize) -> Option<usize> {
    let inside = text[at..].strip_prefix("{{")?;
    let len = inside.find("}}")?;
    let content = &inside[..len];
    (!content.trim().is_empty() && !content.contains(['{', '}', '\n'])).then_some(at + 2 + len + 2)
}

/// `{name}`, `{0}`, `{}` or a name followed by a format (`{0:.2f}`, `{n, number}`, `{x!r}`) at
/// `at`: where it ends.
fn braces(text: &str, at: usize) -> Option<usize> {
    let mut p = skip_spaces(text, at + 1);
    let named = identifier(text, p);
    if let Some(end) = named {
        p = skip_spaces(text, end);
    }
    let rest = &text[p..];
    if named.is_some() && rest.starts_with([',', ':', '!']) {
        let len = rest.find(['{', '}', '\n'])?;
        p += len;
    }
    expect(text, p, "}")
}

/// A printf conversion at `at` (a `%`): where it ends.
fn printf(text: &str, at: usize) -> Option<usize> {
    let mut p = at + 1;
    if text[p..].starts_with('%') {
        return Some(p + 1);
    }
    if let Some(after) = text[p..].strip_prefix('(') {
        let end = identifier(after, 0)?;
        p = expect(text, p + 1 + end, ")")?;
    } else {
        let digits = ascii_digits(&text[p..]);
        if digits > 0 && text[p + digits..].starts_with('$') {
            p += digits + 1;
        }
    }
    p += text[p..].len()
        - text[p..]
            .trim_start_matches(['-', '+', '#', '0', '\''])
            .len();
    p += if text[p..].starts_with('*') {
        1
    } else {
        ascii_digits(&text[p..])
    };
    if let Some(after) = text[p..].strip_prefix('.') {
        p += 1 + if after.starts_with('*') {
            1
        } else {
            ascii_digits(after)
        };
    }
    for length in ["hh", "ll", "h", "l", "L", "q", "j", "z", "t"] {
        if text[p..].starts_with(length) {
            p += length.len();
            break;
        }
    }
    let conversion = text[p..].chars().next()?;
    "diouxXeEfFgGaAcspn@".contains(conversion).then_some(p + 1)
}

fn ascii_digits(text: &str) -> usize {
    text.len() - text.trim_start_matches(|c: char| c.is_ascii_digit()).len()
}

fn skip_spaces(text: &str, at: usize) -> usize {
    at + text[at..].len() - text[at..].trim_start_matches([' ', '\t']).len()
}

/// The end of the name at `at`: letters, digits and `_`, then also `.` and `-` (`{user.name}`).
fn identifier(text: &str, at: usize) -> Option<usize> {
    let rest = &text[at..];
    if !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let len = rest.len()
        - rest
            .trim_start_matches(|c: char| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'))
            .len();
    Some(at + len)
}

fn expect(text: &str, at: usize, wanted: &str) -> Option<usize> {
    text[at..].starts_with(wanted).then_some(at + wanted.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each message as its parts, text as it is and placeholders in brackets.
    fn show(text: &str) -> Vec<String> {
        messages(text)
            .into_iter()
            .map(|inlines| {
                inlines
                    .into_iter()
                    .map(|inline| match inline {
                        Inline::Text(range) => text[range].to_owned(),
                        Inline::Placeholder(range) => format!("[{}]", &text[range]),
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn placeholders_of_each_family_are_found() {
        for (text, shown) in [
            ("Hi {name}!", "Hi [{name}]!"),
            ("{0} of {1}", "[{0}] of [{1}]"),
            (
                "Pi is {0:.2f}, { name }, {}",
                "Pi is [{0:.2f}], [{ name }], [{}]",
            ),
            (
                "{n, number} on {d, date, short}",
                "[{n, number}] on [{d, date, short}]",
            ),
            ("{user.first_name} {x!r}", "[{user.first_name}] [{x!r}]"),
            ("${name} and %{name}", "[${name}] and [%{name}]"),
            ("{{count}} or {{- html}}", "[{{count}}] or [{{- html}}]"),
            ("{{count, number}} items", "[{{count, number}}] items"),
            ("%s has %d of %1$s", "[%s] has [%d] of [%1$s]"),
            (
                "%@ at %.2f, %ld, %-5s, %lld",
                "[%@] at [%.2f], [%ld], [%-5s], [%lld]",
            ),
            ("%(name)s is 100%%", "[%(name)s] is 100[%%]"),
            ("{名前} さん", "[{名前}] さん"),
        ] {
            assert_eq!(show(text), [shown], "{text}");
        }
    }

    #[test]
    fn text_that_only_looks_like_a_placeholder_stays_text() {
        for text in [
            "50% off",
            "100%.",
            "a { b c } d",
            "if (a) { return b; }",
            "{a b}",
            "{{a\nb}}",
            "%",
            "%5",
            "# not a number here",
            "$ and $x",
        ] {
            assert_eq!(show(text), [text], "{text}");
        }
    }

    #[test]
    fn the_messages_of_a_choice_are_translated_and_its_markup_is_not() {
        let text = "You have {count, plural, =0 {no items} one {# item} other {# items}}.";
        assert_eq!(
            show(text),
            ["You have ", "no items", "[#] item", "[#] items", "."]
        );
        let text = "{gender, select, female {She} male {He} other {They}} liked {name}'s post";
        assert_eq!(show(text), ["She", "He", "They", " liked [{name}]'s post"]);
    }

    #[test]
    fn a_pound_sign_is_a_placeholder_only_inside_plural_and_selectordinal() {
        assert_eq!(
            show("{n, selectordinal, one {#st} other {#th}} #1"),
            ["[#]st", "[#]th", " #1"]
        );
        assert_eq!(show("{s, select, other {# tag}}"), ["# tag"]);
        // A select inside a plural reads `#` as the plural's number.
        assert_eq!(
            show("{n, plural, offset:1 other {{g, select, other {# more by {name}}}}}"),
            ["[#] more by [{name}]"]
        );
    }

    #[test]
    fn a_choice_that_does_not_read_as_icu_is_one_message() {
        // No messages, an unknown type, or a message not closed: nothing is cut out of the
        // message, and the braces that still read as a placeholder are one.
        assert_eq!(show("{n, plural,}"), ["[{n, plural,}]"]);
        assert_eq!(show("{n, ordinal, one {a}}"), ["{n, ordinal, one [{a}]}"]);
        assert_eq!(show("{n, plural, one {a}"), ["{n, plural, one [{a}]"]);
        // A message left empty is not one.
        assert_eq!(show("{n, plural, one {} other {x}}"), ["x"]);
    }
}
