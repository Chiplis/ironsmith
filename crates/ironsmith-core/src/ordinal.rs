//! English ordinal helpers shared by parser and renderer code.

use crate::number_words::ORDINAL;

/// Return the English ordinal phrase for numbers 1 through 100.
pub fn ordinal_word(n: u32) -> Option<String> {
    ORDINAL.spell(n)
}

fn parse_numeric_ordinal_word(word: &str) -> Option<u32> {
    let numeric = word
        .strip_suffix("st")
        .or_else(|| word.strip_suffix("nd"))
        .or_else(|| word.strip_suffix("rd"))
        .or_else(|| word.strip_suffix("th"))
        .unwrap_or(word);
    numeric
        .parse::<u32>()
        .ok()
        .filter(|value| (1..=100).contains(value))
}

/// Parse one ordinal token, accepting hyphenated words and numeric forms.
pub fn parse_ordinal_word(word: &str) -> Option<u32> {
    if word.eq_ignore_ascii_case("hundredth") {
        return Some(100);
    }
    parse_numeric_ordinal_word(&word.to_ascii_lowercase()).or_else(|| ORDINAL.read(word))
}

/// Parse an ordinal phrase from the start of a word slice.
///
/// Returns the ordinal value and the number of consumed words. This accepts
/// both hyphenated forms like `twenty-first` and split forms like
/// `twenty first`.
pub fn parse_ordinal_words(words: &[&str]) -> Option<(u32, usize)> {
    let first = words.first().copied()?;
    if let Some(&second) = words.get(1) {
        if first.eq_ignore_ascii_case("one") && second.eq_ignore_ascii_case("hundredth") {
            return Some((100, 2));
        }
        if let Some(n) = ORDINAL.compound(first, second) {
            return Some((n, 2));
        }
    }
    Some((parse_ordinal_word(first)?, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ordinal_round_trips_including_split_compounds() {
        for value in 1..=100 {
            let spelled = ordinal_word(value).unwrap();
            assert_eq!(parse_ordinal_word(&spelled), Some(value));
            let words: Vec<_> = spelled.split(['-', ' ']).collect();
            assert_eq!(parse_ordinal_words(&words), Some((value, words.len())));
        }
    }

    #[test]
    fn ordinal_word_covers_one_to_one_hundred() {
        assert_eq!(ordinal_word(1).as_deref(), Some("first"));
        assert_eq!(ordinal_word(2).as_deref(), Some("second"));
        assert_eq!(ordinal_word(3).as_deref(), Some("third"));
        assert_eq!(ordinal_word(11).as_deref(), Some("eleventh"));
        assert_eq!(ordinal_word(20).as_deref(), Some("twentieth"));
        assert_eq!(ordinal_word(21).as_deref(), Some("twenty-first"));
        assert_eq!(ordinal_word(42).as_deref(), Some("forty-second"));
        assert_eq!(ordinal_word(100).as_deref(), Some("one hundredth"));
        assert_eq!(ordinal_word(0), None);
        assert_eq!(ordinal_word(101), None);
    }

    #[test]
    fn parse_ordinal_words_accepts_common_forms() {
        assert_eq!(parse_ordinal_word("third"), Some(3));
        assert_eq!(parse_ordinal_word("21st"), Some(21));
        assert_eq!(parse_ordinal_word("twenty-first"), Some(21));
        assert_eq!(parse_ordinal_words(&["twenty", "first"]), Some((21, 2)));
        assert_eq!(parse_ordinal_words(&["one", "hundredth"]), Some((100, 2)));
        assert_eq!(parse_ordinal_word("hundredth"), Some(100));
    }
}
