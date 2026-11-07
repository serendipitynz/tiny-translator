//! Telling the source language of a text, for the translation's direction (doc-1 §6.1).
//!
//! The model would detect the source language itself when the prompt leaves it out, but nothing
//! outside it can read what it decided, and the target language depends on it.

/// The language code [`detect`] answers for Chinese. whatlang gives the same language for
/// simplified and traditional Chinese, so it cannot name either of PLaMo's `zh-hans` and `zh-hant`;
/// the screen shows it as Chinese and leaves the source language out of the prompt.
pub const CHINESE: &str = "zh";

/// whatlang's confidence below which the source language is left undecided. TASK-2 measured the
/// wrong answers on English, French, German and Spanish sentences at 0.27 at most.
const MIN_CONFIDENCE: f64 = 0.5;

/// The source language of `text` as PLaMo's language code ([`CHINESE`] for Chinese), or `None`
/// when the app leaves it undecided (doc-1 §6.1's 判定なし): whatlang is not confident enough, or
/// the language is not one PLaMo translates.
pub fn detect(text: &str) -> Option<&'static str> {
    // Kana settle Japanese before whatlang is asked: whatlang reads Japanese mixed with more Latin
    // letters than kana as Romanian or Slovak (doc-1 §6.1).
    if has_kana(text) {
        return Some("ja");
    }
    // whatlang counts the middle dot as katakana and calls a Han text Japanese once 5% of it is
    // kana, which turns short Chinese with a middle dot Japanese; it is kept out here as well.
    let info = whatlang::detect(&text.replace(MIDDLE_DOT, " "))?;
    if info.confidence() < MIN_CONFIDENCE {
        return None;
    }
    plamo_code(info.lang())
}

/// The katakana middle dot, which is not counted as kana: it is punctuation, and Chinese copied
/// from Japanese sources carries it without kana.
const MIDDLE_DOT: char = '\u{30FB}';

/// Hiragana and katakana, the half-width forms included, but not [`MIDDLE_DOT`].
fn has_kana(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(
            c,
            '\u{3041}'..='\u{309F}'
                | '\u{30A0}'..='\u{30FA}'
                | '\u{30FC}'..='\u{30FF}'
                | '\u{31F0}'..='\u{31FF}'
                | '\u{FF66}'..='\u{FF9F}'
        )
    })
}

/// PLaMo's code for each whatlang language PLaMo translates (the model card's list of 53). The
/// rest of whatlang's languages (Portuguese, Turkish, Ukrainian and others) have none.
fn plamo_code(lang: whatlang::Lang) -> Option<&'static str> {
    use whatlang::Lang::*;
    Some(match lang {
        Jpn => "ja",
        Eng => "en",
        Cmn => CHINESE,
        Kor => "ko",
        Ara => "ar",
        Aze => "az",
        Bel => "be",
        Bul => "bg",
        Ben => "bn",
        Cat => "ca",
        Deu => "de",
        Spa => "es",
        Est => "et",
        Pes => "fa",
        Fra => "fr",
        Guj => "gu",
        Heb => "he",
        Hin => "hi",
        Hrv => "hr",
        Hun => "hu",
        Hye => "hy",
        Ind => "id",
        Ita => "it",
        Jav => "jv",
        Kat => "ka",
        Khm => "km",
        Lit => "lt",
        Lav => "lv",
        Mal => "ml",
        Mar => "mr",
        Mya => "my",
        Nep => "ne",
        Nld => "nl",
        Pan => "pa",
        Pol => "pl",
        Rus => "ru",
        Sin => "si",
        Slk => "sk",
        Slv => "sl",
        Srp => "sr",
        Tam => "ta",
        Tel => "te",
        Tha => "th",
        Tgl => "tl",
        Urd => "ur",
        Uzb => "uz",
        Vie => "vi",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kana_make_japanese() {
        assert_eq!(detect("こんにちは"), Some("ja"));
        assert_eq!(detect("React の useEffect が 2 回呼ばれる"), Some("ja"));
        assert_eq!(detect("ｶﾀｶﾅ"), Some("ja"));
    }

    #[test]
    fn a_middle_dot_alone_is_not_kana() {
        assert_eq!(detect("北京・上海之间的高速铁路"), Some(CHINESE));
    }

    #[test]
    fn chinese_and_korean_are_told_by_their_script() {
        assert_eq!(detect("谢谢你的帮助"), Some(CHINESE));
        assert_eq!(detect("謝謝你的幫助"), Some(CHINESE));
        assert_eq!(detect("감사합니다"), Some("ko"));
    }

    #[test]
    fn a_confident_sentence_is_decided() {
        assert_eq!(
            detect("The meeting has been moved to next Monday because the manager is travelling."),
            Some("en")
        );
        assert_eq!(
            detect(
                "La réunion a été déplacée à lundi prochain parce que le directeur est en voyage."
            ),
            Some("fr")
        );
    }

    #[test]
    fn an_unsure_guess_is_left_undecided() {
        // TASK-2: whatlang names a single English word as Danish or French with confidence < 0.25.
        assert_eq!(detect("Settings"), None);
        assert_eq!(detect(""), None);
    }

    #[test]
    fn a_language_plamo_does_not_translate_is_left_undecided() {
        let text = "A reunião foi transferida para a próxima segunda-feira porque o gerente está viajando.";
        let info = whatlang::detect(text).unwrap();
        assert_eq!(info.lang(), whatlang::Lang::Por);
        assert!(info.confidence() >= MIN_CONFIDENCE);
        assert_eq!(detect(text), None);
    }
}
