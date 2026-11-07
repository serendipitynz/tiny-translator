//! Cutting a text into 翻訳単位, and those into smaller pieces (doc-1 §7.1, §7.3).
//!
//! Every split keeps the whitespace around its pieces, so the translation is put back together
//! with the original's line breaks and indentation between the translated pieces.

/// A text as the pieces to translate, each followed by the whitespace after it.
#[derive(Debug, PartialEq, Eq)]
pub struct Split<'a> {
    /// The whitespace before the first piece: all of the text when it has no piece.
    pub leading: &'a str,
    pub pieces: Vec<(&'a str, &'a str)>,
}

impl<'a> Split<'a> {
    /// The pieces at `ranges` (in order, without overlap), with the text between them as the
    /// whitespace that follows each.
    fn from_ranges(text: &'a str, ranges: &[(usize, usize)]) -> Self {
        let leading = &text[..ranges.first().map_or(text.len(), |range| range.0)];
        let pieces = ranges
            .iter()
            .enumerate()
            .map(|(i, &(start, end))| {
                let next = ranges.get(i + 1).map_or(text.len(), |range| range.0);
                (&text[start..end], &text[end..next])
            })
            .collect();
        Split { leading, pieces }
    }
}

/// プレーンテキストの翻訳単位 (doc-1 §7.1): the paragraphs between blank lines. A line holding
/// only whitespace is blank too.
pub fn paragraphs(text: &str) -> Split<'_> {
    let mut ranges = Vec::new();
    let mut current: Option<(usize, usize)> = None;
    for (start, end) in line_bodies(text) {
        match (&mut current, start == end) {
            (_, true) => ranges.extend(current.take()),
            (Some(paragraph), false) => paragraph.1 = end,
            (None, false) => current = Some((start, end)),
        }
    }
    ranges.extend(current);
    Split::from_ranges(text, &ranges)
}

/// Each line that is not blank.
pub fn lines(text: &str) -> Split<'_> {
    let ranges: Vec<_> = line_bodies(text)
        .filter(|(start, end)| start != end)
        .collect();
    Split::from_ranges(text, &ranges)
}

/// The range of each line without the whitespace around it; empty for a blank line.
fn line_bodies(text: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
    let mut offset = 0;
    text.split_inclusive('\n').map(move |line| {
        let start = offset;
        offset += line.len();
        let body = line.trim();
        if body.is_empty() {
            return (start, start);
        }
        let body_start = start + (line.len() - line.trim_start().len());
        (body_start, body_start + body.len())
    })
}

/// The sentences of `text`. A sentence ends at 。！？ (with any closing brackets or quotes after
/// them), and at `.` `!` `?` only before whitespace, so `3.14` and `example.com` stay whole.
/// A line break alone does not end one: hard-wrapped prose breaks lines inside sentences.
pub fn sentences(text: &str) -> Split<'_> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let byte_at = |i: usize| chars.get(i).map_or(text.len(), |(at, _)| *at);
    let mut ends = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !is_terminal(chars[i].1) {
            i += 1;
            continue;
        }
        // A run such as `?!` or `。。` ends the sentence once, after its last mark.
        let mut wide = false;
        while i < chars.len() && is_terminal(chars[i].1) {
            wide |= is_wide_terminal(chars[i].1);
            i += 1;
        }
        let marks_end = i;
        while i < chars.len() && is_closing(chars[i].1) {
            i += 1;
        }
        let ends_here = match chars.get(i) {
            None => false,
            Some((_, next)) if next.is_whitespace() => true,
            // 「はい。」と言った。 is one sentence; 「はい。」「いいえ。」 is two.
            Some((_, next)) => wide && (i == marks_end || is_opening(*next)),
        };
        if ends_here {
            ends.push(byte_at(i));
        }
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    for end in ends.into_iter().chain([text.len()]) {
        ranges.extend(trimmed(text, start, end));
        start = end;
    }
    Split::from_ranges(text, &ranges)
}

fn is_terminal(c: char) -> bool {
    is_wide_terminal(c) || matches!(c, '.' | '!' | '?')
}

/// Marks that end a sentence wherever they stand: Japanese and Chinese do not put a space after
/// them.
fn is_wide_terminal(c: char) -> bool {
    matches!(c, '。' | '｡' | '．' | '！' | '？')
}

fn is_closing(c: char) -> bool {
    matches!(
        c,
        '」' | '』' | '）' | '】' | '］' | '〉' | '》' | ')' | ']' | '"' | '\'' | '”' | '’'
    )
}

fn is_opening(c: char) -> bool {
    matches!(
        c,
        '「' | '『' | '（' | '【' | '［' | '〈' | '《' | '(' | '[' | '“' | '‘'
    )
}

/// `text[start..end]` without the whitespace around it, or `None` when it is all whitespace.
fn trimmed(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let part = &text[start..end];
    let body = part.trim();
    if body.is_empty() {
        return None;
    }
    let body_start = start + (part.len() - part.trim_start().len());
    Some((body_start, body_start + body.len()))
}

/// `text` (without whitespace around it) as consecutive pieces of at most `max` tokens by
/// `count`. Cut, from the most to the least natural, at the end of a sentence, at a line break,
/// and as a last resort into pieces of equal length — for text such as minified data that has
/// neither. Neighbouring pieces are then joined again as far as `max` allows, so a piece carries as
/// much of its surroundings as fits.
pub fn fit<E>(
    text: &str,
    max: usize,
    mut count: impl FnMut(&str) -> Result<usize, E>,
) -> Result<Vec<(&str, &str)>, E> {
    let tokens = count(text)?;
    if tokens <= max {
        return Ok(vec![(text, "")]);
    }
    let mut pieces = Vec::new();
    refine(
        text,
        0,
        tokens,
        Level::Sentences,
        max,
        &mut count,
        &mut pieces,
    )?;

    let mut chunks: Vec<Piece> = Vec::new();
    for piece in pieces {
        match chunks.last_mut() {
            // The whitespace that joins them counts at its length in bytes, which no token count
            // exceeds: a long run of it, such as a wide indent, must not fit by being counted as one.
            Some(chunk) if chunk.tokens + (piece.start - chunk.end) + piece.tokens <= max => {
                chunk.tokens += (piece.start - chunk.end) + piece.tokens;
                chunk.end = piece.end;
            }
            _ => chunks.push(piece),
        }
    }
    let ranges: Vec<_> = chunks
        .iter()
        .map(|chunk| (chunk.start, chunk.end))
        .collect();
    Ok(Split::from_ranges(text, &ranges).pieces)
}

#[derive(Debug, Clone, Copy)]
struct Piece {
    start: usize,
    end: usize,
    tokens: usize,
}

#[derive(Debug, Clone, Copy)]
enum Level {
    Sentences,
    Lines,
    Characters,
}

/// Appends the pieces of `text` (at `base` in the whole, `tokens` long) that fit in `max`,
/// cutting the ones that do not at the next level.
fn refine<E>(
    text: &str,
    base: usize,
    tokens: usize,
    level: Level,
    max: usize,
    count: &mut impl FnMut(&str) -> Result<usize, E>,
    out: &mut Vec<Piece>,
) -> Result<(), E> {
    let (split, next) = match level {
        Level::Sentences => (sentences(text), Level::Lines),
        Level::Lines => (lines(text), Level::Characters),
        Level::Characters => return by_characters(text, base, tokens, max, count, out),
    };
    if split.pieces.len() < 2 {
        return refine(text, base, tokens, next, max, count, out);
    }
    for (piece, _) in split.pieces {
        let start = base + offset_in(text, piece);
        let tokens = count(piece)?;
        if tokens <= max {
            out.push(Piece {
                start,
                end: start + piece.len(),
                tokens,
            });
        } else {
            refine(piece, start, tokens, next, max, count, out)?;
        }
    }
    Ok(())
}

/// `text` cut into the fewest pieces of equal character counts that each fit in `max`.
fn by_characters<E>(
    text: &str,
    base: usize,
    tokens: usize,
    max: usize,
    count: &mut impl FnMut(&str) -> Result<usize, E>,
    out: &mut Vec<Piece>,
) -> Result<(), E> {
    let bounds: Vec<usize> = text
        .char_indices()
        .map(|(at, _)| at)
        .chain([text.len()])
        .collect();
    let characters = bounds.len() - 1;
    let mut parts = tokens.div_ceil(max.max(1)).clamp(1, characters.max(1));
    loop {
        let mut pieces = Vec::with_capacity(parts);
        let mut fits = true;
        for i in 0..parts {
            let Some((start, end)) = trimmed(
                text,
                bounds[i * characters / parts],
                bounds[(i + 1) * characters / parts],
            ) else {
                continue;
            };
            let tokens = count(&text[start..end])?;
            fits &= tokens <= max;
            pieces.push(Piece {
                start: base + start,
                end: base + end,
                tokens,
            });
        }
        // One character apiece is as small as a cut can make them.
        if fits || parts >= characters {
            out.extend(pieces);
            return Ok(());
        }
        parts += 1;
    }
}

/// Where `part`, a slice of `whole`, starts in it.
fn offset_in(whole: &str, part: &str) -> usize {
    part.as_ptr() as usize - whole.as_ptr() as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    fn split<'a>(leading: &'a str, pieces: &[(&'a str, &'a str)]) -> Split<'a> {
        Split {
            leading,
            pieces: pieces.to_vec(),
        }
    }

    /// Joins a split back into the text it came from.
    fn rejoin(split: &Split<'_>) -> String {
        let mut text = split.leading.to_owned();
        for (piece, after) in &split.pieces {
            text.push_str(piece);
            text.push_str(after);
        }
        text
    }

    #[test]
    fn paragraphs_are_separated_by_blank_lines() {
        let text = "First line\nstill first.\n\nSecond.\n \t\n\n  Third, indented.";
        assert_eq!(
            paragraphs(text),
            split(
                "",
                &[
                    ("First line\nstill first.", "\n\n"),
                    ("Second.", "\n \t\n\n  "),
                    ("Third, indented.", ""),
                ]
            )
        );
    }

    #[test]
    fn paragraphs_keep_the_whitespace_around_the_text() {
        let text = "\n\n  一段落目。\r\n\r\n二段落目。\n\n";
        let parts = paragraphs(text);
        assert_eq!(
            parts,
            split(
                "\n\n  ",
                &[("一段落目。", "\r\n\r\n"), ("二段落目。", "\n\n")]
            )
        );
        assert_eq!(rejoin(&parts), text);
    }

    #[test]
    fn a_line_of_full_width_spaces_is_blank() {
        assert_eq!(
            paragraphs("前。\n\u{3000}\n後。").pieces,
            [("前。", "\n\u{3000}\n"), ("後。", "")]
        );
    }

    #[test]
    fn a_text_of_whitespace_has_no_paragraph() {
        assert_eq!(paragraphs(" \n\n "), split(" \n\n ", &[]));
        assert_eq!(paragraphs(""), split("", &[]));
    }

    #[test]
    fn sentences_end_at_japanese_marks_without_a_space() {
        assert_eq!(
            sentences("今日は晴れ。明日は雨？本当に！").pieces,
            [("今日は晴れ。", ""), ("明日は雨？", ""), ("本当に！", "")]
        );
    }

    #[test]
    fn sentences_end_at_latin_marks_only_before_whitespace() {
        assert_eq!(
            sentences("Pi is 3.14. See example.com now!  Really?! Yes.").pieces,
            [
                ("Pi is 3.14.", " "),
                ("See example.com now!", "  "),
                ("Really?!", " "),
                ("Yes.", ""),
            ]
        );
    }

    #[test]
    fn closing_quotes_stay_with_their_sentence() {
        assert_eq!(
            sentences("「はい。」と言った。「いいえ。」「本当？」 He said \"No.\" Then left.")
                .pieces,
            [
                ("「はい。」と言った。", ""),
                ("「いいえ。」", ""),
                ("「本当？」", " "),
                ("He said \"No.\"", " "),
                ("Then left.", ""),
            ]
        );
    }

    #[test]
    fn a_line_break_inside_a_sentence_does_not_end_it() {
        assert_eq!(
            sentences("A sentence wrapped\nacross lines. 次の文。\n最後").pieces,
            [
                ("A sentence wrapped\nacross lines.", " "),
                ("次の文。", "\n"),
                ("最後", ""),
            ]
        );
    }

    #[test]
    fn lines_are_each_line_that_is_not_blank() {
        assert_eq!(
            lines("- one\n  - two\n\n- three").pieces,
            [("- one", "\n  "), ("- two", "\n\n"), ("- three", "")]
        );
    }

    /// One token per character, which keeps the expected pieces easy to read.
    fn characters(text: &str) -> Result<usize, Infallible> {
        Ok(text.chars().count())
    }

    #[test]
    fn a_text_that_fits_is_one_piece() {
        assert_eq!(
            fit("Short enough.", 13, characters).unwrap(),
            [("Short enough.", "")]
        );
    }

    #[test]
    fn a_text_too_long_is_cut_at_sentences_and_joined_back_up_to_the_limit() {
        // 6 + 1 + 6 = 13 fits, a third sentence would not.
        let text = "One a. Two b. Six c. End.";
        assert_eq!(
            fit(text, 13, characters).unwrap(),
            [("One a. Two b.", " "), ("Six c. End.", "")]
        );
    }

    #[test]
    fn the_whitespace_between_sentences_counts_when_they_are_joined() {
        // Joined, the two sentences and the spaces between them would be 24 tokens.
        let text = format!("One a.{}Two b.", " ".repeat(12));
        assert_eq!(
            fit(&text, 20, characters).unwrap(),
            [("One a.", &" ".repeat(12)[..]), ("Two b.", "")]
        );
        assert_eq!(fit(&text, 24, characters).unwrap(), [(&text[..], "")]);
    }

    #[test]
    fn a_sentence_too_long_is_cut_at_line_breaks() {
        let text = "short.\nline one of a list\nline two of a list";
        assert_eq!(
            fit(text, 18, characters).unwrap(),
            [
                ("short.", "\n"),
                ("line one of a list", "\n"),
                ("line two of a list", ""),
            ]
        );
    }

    #[test]
    fn a_text_without_sentences_or_lines_is_cut_into_equal_lengths() {
        let text = "a".repeat(25);
        let pieces = fit(&text, 10, characters).unwrap();
        assert_eq!(pieces.len(), 3);
        assert!(pieces.iter().all(|(piece, _)| piece.len() <= 10));
        assert_eq!(
            pieces.iter().map(|(piece, _)| *piece).collect::<String>(),
            text
        );
    }

    #[test]
    fn every_piece_fits_and_the_pieces_rejoin_into_the_text() {
        let text = "これは長い文章です。とても長いので、分けないと入りません。\n\
                    Another paragraph line follows here. And one more sentence!\n\
                    {\"minified\":\"data\",\"without\":\"any\",\"break\":\"at\",\"all\":1}";
        for max in [8, 20, 40, 64] {
            let pieces = fit(text, max, characters).unwrap();
            let mut joined = String::new();
            for (piece, after) in &pieces {
                assert!(piece.chars().count() <= max, "{piece:?} over {max}");
                joined.push_str(piece);
                joined.push_str(after);
            }
            assert_eq!(joined, text, "max {max}");
        }
    }

    #[test]
    fn a_counting_error_is_passed_on() {
        let result = fit("One. Two.", 2, |_| Err::<usize, _>("no engine"));
        assert_eq!(result, Err("no engine"));
    }
}
