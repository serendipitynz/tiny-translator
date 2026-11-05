//! Measure how well whatlang identifies the source language of short texts.
//!
//! Reads `samples.tsv`, runs whatlang 0.18 on every text, and applies the decision
//! rule the app is meant to use (doc-1 §6.1): kana first, then whatlang, and no
//! decision when whatlang's confidence is below a threshold. Prints Markdown tables:
//! one row per sample, a summary per group and length, the adopted threshold per
//! group and length, and a threshold sweep that also shows the target language the
//! default priorities (ja, then en) would pick.

use std::collections::BTreeMap;

const SAMPLES: &str = include_str!("../samples.tsv");
const THRESHOLDS: [f64; 7] = [0.0, 0.3, 0.5, 0.7, 0.8, 0.9, 1.0];
const ADOPTED_THRESHOLD: f64 = 0.5;
const FIRST_PRIORITY: &str = "ja";
const SECOND_PRIORITY: &str = "en";

struct Sample {
    group: String,
    expected: String,
    length: String,
    text: String,
}

struct Detection {
    whatlang_lang: Option<String>,
    script: Option<String>,
    confidence: f64,
    has_kana: bool,
}

fn parse_samples() -> Vec<Sample> {
    SAMPLES
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.splitn(4, '\t').collect();
            assert_eq!(fields.len(), 4, "malformed sample line: {line}");
            Sample {
                group: fields[0].to_owned(),
                expected: fields[1].to_owned(),
                length: fields[2].to_owned(),
                text: fields[3].to_owned(),
            }
        })
        .collect()
}

fn has_kana(text: &str) -> bool {
    // U+30FB (katakana middle dot) is excluded: it is punctuation, and Chinese text
    // copied from Japanese sources sometimes carries it without any kana.
    text.chars().any(|c| {
        matches!(c, '\u{3041}'..='\u{309F}' | '\u{30A0}'..='\u{30FA}' | '\u{30FC}'..='\u{30FF}')
            || matches!(c, '\u{31F0}'..='\u{31FF}' | '\u{FF66}'..='\u{FF9F}')
    })
}

fn short_code(lang: whatlang::Lang) -> String {
    match lang {
        whatlang::Lang::Eng => "en".to_owned(),
        whatlang::Lang::Jpn => "ja".to_owned(),
        whatlang::Lang::Cmn => "zh".to_owned(),
        whatlang::Lang::Kor => "ko".to_owned(),
        other => other.code().to_owned(),
    }
}

fn detect(text: &str) -> Detection {
    let info = whatlang::detect(text);
    Detection {
        whatlang_lang: info.as_ref().map(|i| short_code(i.lang())),
        script: info.as_ref().map(|i| i.script().name().to_owned()),
        confidence: info.as_ref().map_or(0.0, |i| i.confidence()),
        has_kana: has_kana(text),
    }
}

/// `None` means the app leaves the source language out of the prompt.
fn decide(detection: &Detection, threshold: f64) -> Option<String> {
    if detection.has_kana {
        return Some("ja".to_owned());
    }
    match &detection.whatlang_lang {
        Some(lang) if detection.confidence >= threshold => Some(lang.clone()),
        _ => None,
    }
}

fn target_for(source: Option<&str>) -> &'static str {
    if source == Some(FIRST_PRIORITY) {
        SECOND_PRIORITY
    } else {
        FIRST_PRIORITY
    }
}

fn truncate(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out.replace('|', "\\|")
}

#[derive(Default)]
struct Tally {
    total: usize,
    correct: usize,
    undecided: usize,
    wrong: usize,
    target_ok: usize,
}

impl Tally {
    fn add(&mut self, expected: &str, decided: Option<&str>) {
        self.total += 1;
        match decided {
            Some(lang) if lang == expected => self.correct += 1,
            Some(_) => self.wrong += 1,
            None => self.undecided += 1,
        }
        if target_for(decided) == target_for(Some(expected)) {
            self.target_ok += 1;
        }
    }
}

fn print_samples(samples: &[Sample], detections: &[Detection]) {
    println!("## Per sample\n");
    println!("| group | length | text | whatlang | script | confidence | kana | kana + whatlang |");
    println!("|---|---|---|---|---|---|---|---|");
    for (sample, d) in samples.iter().zip(detections) {
        let decided = decide(d, 0.0);
        let mark = if decided.as_deref() == Some(sample.expected.as_str()) {
            ""
        } else {
            " ✗"
        };
        println!(
            "| {} | {} | {} | {} | {} | {:.2} | {} | {}{} |",
            sample.group,
            sample.length,
            truncate(&sample.text, 24),
            d.whatlang_lang.as_deref().unwrap_or("-"),
            d.script.as_deref().unwrap_or("-"),
            d.confidence,
            if d.has_kana { "yes" } else { "no" },
            decided.as_deref().unwrap_or("-"),
            mark,
        );
    }
    println!();
}

fn print_summary(samples: &[Sample], detections: &[Detection]) {
    println!("## Correct per group and length (no threshold)\n");
    println!("| group | length | n | whatlang alone | kana + whatlang |");
    println!("|---|---|---|---|---|");
    let mut rows: BTreeMap<(&str, &str), (usize, usize, usize)> = BTreeMap::new();
    for (sample, d) in samples.iter().zip(detections) {
        let row = rows.entry((&sample.group, &sample.length)).or_default();
        row.0 += 1;
        if d.whatlang_lang.as_deref() == Some(sample.expected.as_str()) {
            row.1 += 1;
        }
        if decide(d, 0.0).as_deref() == Some(sample.expected.as_str()) {
            row.2 += 1;
        }
    }
    for ((group, length), (n, alone, combined)) in rows {
        println!("| {group} | {length} | {n} | {alone} | {combined} |");
    }
    println!();
}

fn print_adopted(samples: &[Sample], detections: &[Detection]) {
    println!("## Adopted rule (kana + whatlang, threshold {ADOPTED_THRESHOLD:.1})\n");
    println!("correct / undecided / wrong per group and length.\n");
    println!("| group | word | sentence | paragraph |");
    println!("|---|---|---|---|");
    let mut rows: BTreeMap<&str, BTreeMap<&str, Tally>> = BTreeMap::new();
    for (sample, d) in samples.iter().zip(detections) {
        rows.entry(&sample.group)
            .or_default()
            .entry(&sample.length)
            .or_default()
            .add(&sample.expected, decide(d, ADOPTED_THRESHOLD).as_deref());
    }
    for (group, lengths) in rows {
        let cell = |length: &str| {
            lengths.get(length).map_or("-".to_owned(), |t| {
                format!("{}/{}/{} of {}", t.correct, t.undecided, t.wrong, t.total)
            })
        };
        println!(
            "| {group} | {} | {} | {} |",
            cell("word"),
            cell("sentence"),
            cell("paragraph")
        );
    }
    println!();
}

fn print_threshold_sweep(samples: &[Sample], detections: &[Detection]) {
    println!("## Threshold sweep (kana + whatlang)\n");
    println!(
        "correct / undecided / wrong; target = the target language the priorities \
         ({FIRST_PRIORITY}, {SECOND_PRIORITY}) pick, counted as right when it matches \
         the one the expected language would give.\n"
    );
    let groups: Vec<&str> = {
        let mut seen = Vec::new();
        for sample in samples {
            if !seen.contains(&sample.group.as_str()) {
                seen.push(&sample.group);
            }
        }
        seen
    };
    print!("| threshold | all | target ok |");
    for group in &groups {
        print!(" {group} |");
    }
    println!();
    println!("|---|---|---|{}", "---|".repeat(groups.len()));
    for threshold in THRESHOLDS {
        let mut all = Tally::default();
        let mut per_group: BTreeMap<&str, Tally> = BTreeMap::new();
        for (sample, d) in samples.iter().zip(detections) {
            let decided = decide(d, threshold);
            all.add(&sample.expected, decided.as_deref());
            per_group
                .entry(&sample.group)
                .or_default()
                .add(&sample.expected, decided.as_deref());
        }
        print!(
            "| {threshold:.1} | {}/{}/{} of {} | {}/{} |",
            all.correct, all.undecided, all.wrong, all.total, all.target_ok, all.total
        );
        for group in &groups {
            let t = &per_group[group];
            print!(" {}/{}/{} |", t.correct, t.undecided, t.wrong);
        }
        println!();
    }
    println!();
}

fn main() {
    let samples = parse_samples();
    let detections: Vec<Detection> = samples.iter().map(|s| detect(&s.text)).collect();
    println!("# whatlang 0.18.0 on {} short texts\n", samples.len());
    print_summary(&samples, &detections);
    print_adopted(&samples, &detections);
    print_threshold_sweep(&samples, &detections);
    print_samples(&samples, &detections);
}
