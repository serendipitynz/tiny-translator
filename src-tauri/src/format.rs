//! 形式の判定 (doc-1 §7.2).
//!
//! Each format is told by the parser that translates it, so a text detected as JSON is one the
//! JSON translation reads, and what counts as a block element or a heading is what the HTML and
//! Markdown translations take for one.

use std::ops::Range;

use crate::{html, markdown};

/// The format `text` is read as when the screen leaves the choice to the app, by its name in
/// `src/lib/translate.ts`: `"json"`, `"html"`, `"markdown"` or `"text"`.
pub fn detect(text: &str) -> &'static str {
    if is_json(text) {
        return "json";
    }
    let markdown = markdown::signs(text);
    // A Markdown document showing HTML in its code (```html …```) is not HTML for that.
    let html = html::signs(&without(text, &markdown.code));
    // A README may open with a centred `<div>` of a logo and a title, and is Markdown all the
    // same: its blocks of HTML are fewer than its headings, lists and code blocks.
    if html.document || (html.closed_blocks >= 2 && html.closed_blocks > markdown.syntax) {
        return "html";
    }
    if markdown.syntax > 0 {
        return "markdown";
    }
    "text"
}

/// Whether `text` is JSON holding an object or an array. A lone `true`, `null` or number is JSON
/// too, but as a source it is the English word or the figure, which JSON would leave untranslated.
fn is_json(text: &str) -> bool {
    matches!(
        serde_json::from_str::<serde_json::Value>(text),
        Ok(serde_json::Value::Object(_) | serde_json::Value::Array(_))
    )
}

/// `text` with the bytes in `ranges` made spaces.
fn without(text: &str, ranges: &[Range<usize>]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for range in ranges {
        out.push_str(&text[from..range.start]);
        out.extend(std::iter::repeat_n(' ', range.len()));
        from = range.end;
    }
    out.push_str(&text[from..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_object_or_an_array_is_json() {
        assert_eq!(detect(r#"{"title": "Hello"}"#), "json");
        assert_eq!(detect("[\n  \"one\",\n  \"two\"\n]\n"), "json");
    }

    #[test]
    fn a_lone_json_value_is_not_json() {
        assert_eq!(detect("true"), "text");
        assert_eq!(detect("null"), "text");
        assert_eq!(detect("42"), "text");
        assert_eq!(detect(r#""Hello""#), "text");
    }

    #[test]
    fn json_that_does_not_parse_is_read_otherwise() {
        // JSONC is not read by the JSON translation (doc-1 §7.1), so it is not detected as JSON.
        assert_eq!(detect("{\n  // comment\n  \"a\": \"b\"\n}"), "text");
        assert_eq!(detect("{\"a\": \"b\",}"), "text");
    }

    #[test]
    fn a_document_is_html() {
        assert_eq!(detect("<!DOCTYPE html>\n<title>Hello</title>"), "html");
        assert_eq!(detect("<!doctype html><p>Hello"), "html");
        assert_eq!(detect("<html lang=en><body>Hello</body></html>"), "html");
    }

    #[test]
    fn closed_blocks_make_html() {
        assert_eq!(detect("<p>Hello</p>\n<p>World</p>"), "html");
        assert_eq!(
            detect("<ul>\n  <li>One\n  <li>Two\n</ul>\n<p>End</p>"),
            "html"
        );
    }

    #[test]
    fn one_closed_block_is_not_enough() {
        assert_eq!(detect("<p>Hello <b>world</b></p>"), "text");
        assert_eq!(detect("Press <kbd>Ctrl</kbd> and <b>C</b>."), "text");
    }

    #[test]
    fn markdown_with_some_html_stays_markdown() {
        let readme = "<div align=\"center\">\n  <h1>tiny</h1>\n  <p>A translator</p>\n</div>\n\n\
                      ## Install\n\n- macOS\n- Windows\n\n## Use\n\n```sh\npnpm tauri dev\n```\n";
        assert_eq!(detect(readme), "markdown");
    }

    #[test]
    fn html_in_markdown_code_is_not_counted() {
        let text = "# Example\n\n```html\n<!DOCTYPE html>\n<p>One</p>\n<p>Two</p>\n```\n\n\
                    Write `</p>` and `</div>` to close them.\n";
        assert_eq!(detect(text), "markdown");
    }

    #[test]
    fn blocks_inside_unread_content_are_not_counted() {
        assert_eq!(detect("<svg><g></g><g></g></svg>"), "text");
        assert_eq!(detect("<pre><div>a</div><div>b</div></pre>"), "text");
        assert_eq!(detect("<pre>a</pre>\n<pre>b</pre>"), "html");
    }

    #[test]
    fn headings_lists_and_fences_make_markdown() {
        assert_eq!(detect("# Title\n\nSome text."), "markdown");
        assert_eq!(detect("Shopping:\n\n- milk\n- eggs"), "markdown");
        assert_eq!(detect("1. First\n2. Second"), "markdown");
        assert_eq!(detect("Run:\n\n```\nmake\n```"), "markdown");
    }

    #[test]
    fn an_underlined_heading_is_not_counted() {
        assert_eq!(detect("Best regards\n--\nTaro"), "text");
        assert_eq!(detect("Title\n=====\n\nText."), "text");
    }

    #[test]
    fn prose_is_text() {
        assert_eq!(detect("Hello, world.\n\nThis is a test."), "text");
        assert_eq!(detect("これは #1 の件です。a < b のとき"), "text");
        assert_eq!(detect(""), "text");
    }
}
