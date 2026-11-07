//! Noticing that a translation has started repeating itself (doc-1 §7.3).
//!
//! A small model decoding greedily can fall into a cycle and emit the same few words until the
//! context is full (doc-2 §1.1). The check runs on characters, not on the stream's pieces: a piece
//! is usually one token, but llama-server holds back a token that ends inside a UTF-8 character
//! and sends it with the next, so the same output does not always arrive in the same pieces.
//!
//! For every period up to [`MAX_PERIOD`], [`Repetition`] keeps how many characters at the end equal
//! the one a period earlier, which costs a constant per period and character.

/// The longest cycle noticed, in characters: a sentence or two. Longer ones run until the
/// context is full, which ends the translation as well.
const MAX_PERIOD: usize = 256;

/// A cycle has to come round this many times in a row to count.
const MIN_REPEATS: usize = 3;

/// The shortest repeated span that counts, in characters, when the input repeats nothing itself.
/// Long enough that 「はいはいはい」 or a short row of `=` is left alone.
const MIN_SPAN: usize = 64;

/// A cycle at the end of the output, in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loop {
    /// Where the repeated span starts.
    pub start: usize,
    pub period: usize,
}

impl Loop {
    /// How much of the output to keep: up to the end of the cycle's first time round.
    pub fn keep(&self) -> usize {
        self.start + self.period
    }
}

#[derive(Debug, Clone)]
pub struct Repetition {
    chars: Vec<char>,
    /// `runs[p - 1]`: how many characters at the end equal the one `p` earlier.
    runs: Vec<usize>,
    min_span: usize,
}

impl Repetition {
    /// A check for the translation of `input`. A span counts as a loop only when it is well beyond
    /// any repetition the input has of its own (a ruler of dashes, a refrain), which the
    /// translation is expected to keep.
    pub fn for_input(input: &str) -> Self {
        Repetition {
            chars: Vec::new(),
            runs: vec![0; MAX_PERIOD],
            min_span: MIN_SPAN.max(2 * longest_periodic_span(input)),
        }
    }

    /// Adds the next character of the output, and answers the loop the output now ends in.
    pub fn push(&mut self, c: char) -> Option<Loop> {
        let n = self.chars.len();
        let mut found = None;
        for period in 1..=MAX_PERIOD.min(n) {
            let run = &mut self.runs[period - 1];
            *run = if self.chars[n - period] == c {
                *run + 1
            } else {
                0
            };
            let span = *run + period;
            // The shortest period first, so the loop found starts as early as it can.
            if found.is_none() && span >= MIN_REPEATS * period && span >= self.min_span {
                found = Some(Loop {
                    start: n + 1 - span,
                    period,
                });
            }
        }
        self.chars.push(c);
        found
    }
}

/// The longest span of `text` that is a cycle coming round at least twice.
fn longest_periodic_span(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut runs = vec![0usize; MAX_PERIOD];
    let mut longest = 0;
    for (n, c) in chars.iter().enumerate() {
        for period in 1..=MAX_PERIOD.min(n) {
            let run = &mut runs[period - 1];
            *run = if chars[n - period] == *c { *run + 1 } else { 0 };
            if *run >= period {
                longest = longest.max(*run + period);
            }
        }
    }
    longest
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `output` and answers the first loop found, with how many characters it took.
    fn first_loop(input: &str, output: &str) -> Option<(Loop, usize)> {
        let mut check = Repetition::for_input(input);
        output
            .chars()
            .enumerate()
            .find_map(|(i, c)| check.push(c).map(|found| (found, i + 1)))
    }

    #[test]
    fn a_sentence_repeated_over_and_over_is_a_loop() {
        let sentence = "これは繰り返される文です。";
        let output = format!("最初に{}", sentence.repeat(10));
        let (found, at) = first_loop("This is the input.", &output).unwrap();
        assert_eq!(found.period, sentence.chars().count());
        assert_eq!(found.start, "最初に".chars().count());
        // Noticed as soon as the span is both three times round and MIN_SPAN long.
        assert_eq!(at, found.start + MIN_SPAN.max(3 * found.period));
        let kept: String = output.chars().take(found.keep()).collect();
        assert_eq!(kept, format!("最初に{sentence}"));
    }

    #[test]
    fn one_character_repeated_is_a_loop_once_it_is_long_enough() {
        let output = format!("答え{}", "ー".repeat(100));
        let (found, at) = first_loop("Answer", &output).unwrap();
        assert_eq!(
            found,
            Loop {
                start: 2,
                period: 1
            }
        );
        assert_eq!(at, 2 + MIN_SPAN);
        assert!(first_loop("Answer", &format!("答え{}", "ー".repeat(MIN_SPAN - 1))).is_none());
    }

    #[test]
    fn ordinary_text_is_not_a_loop() {
        let output = "The weather is fine today, so we went to the park. \
                      Yes, yes, yes — it was a very, very good day. \
                      はいはいはい。とてもとても良い一日でした。";
        assert!(first_loop("今日は晴れ", output).is_none());
    }

    #[test]
    fn repetition_the_input_has_is_not_a_loop() {
        let ruler = "-".repeat(80);
        let input = format!("Title\n{ruler}\nBody");
        let output = format!("題名\n{ruler}\n本文");
        assert!(first_loop(&input, &output).is_none());
        // Past twice the input's own, it is.
        let runaway = format!("題名\n{}", "-".repeat(200));
        let (found, at) = first_loop(&input, &runaway).unwrap();
        assert_eq!(found.period, 1);
        assert_eq!(at, "題名\n".chars().count() + 2 * 80);
    }

    #[test]
    fn a_cycle_of_two_sentences_is_found_at_its_shortest_period() {
        let cycle = "A cat sat. A dog ran. ";
        let output = format!("Begin—{}", cycle.repeat(6));
        let (found, _) = first_loop("input", &output).unwrap();
        assert_eq!(found.period, cycle.chars().count());
        assert_eq!(found.start, "Begin—".chars().count());
    }

    #[test]
    fn the_longest_periodic_span_needs_two_rounds() {
        assert_eq!(longest_periodic_span("abc"), 0);
        assert_eq!(longest_periodic_span("abab"), 4);
        assert_eq!(longest_periodic_span("x====y"), 4);
    }
}
