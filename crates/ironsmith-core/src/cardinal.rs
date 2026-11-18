//! English cardinal helpers shared by parser and renderer code.

use crate::number_words::CARDINAL;

/// Return the English cardinal phrase for numbers 0 through 100.
pub fn cardinal_word(n: u32) -> Option<String> {
    CARDINAL.spell(n)
}

/// Parse one cardinal token, accepting hyphenated words and raw numeric forms.
pub fn parse_cardinal_word(word: &str) -> Option<u32> {
    if word.eq_ignore_ascii_case("a") || word.eq_ignore_ascii_case("an") {
        return Some(1);
    }
    word.parse().ok().or_else(|| CARDINAL.read(word))
}

/// Parse a cardinal phrase from the start of a word slice.
///
/// Returns the cardinal value and the number of consumed words. This accepts
/// hyphenated forms like `twenty-one`, split forms like `twenty one`, and
/// `one hundred`.
pub fn parse_cardinal_words(words: &[&str]) -> Option<(u32, usize)> {
    let first = words.first().copied()?;
    if let Some(&second) = words.get(1) {
        if second.eq_ignore_ascii_case("hundred")
            && ["a", "an", "one"]
                .iter()
                .any(|word| first.eq_ignore_ascii_case(word))
        {
            return Some((100, 2));
        }
        if let Some(n) = CARDINAL.compound(first, second) {
            return Some((n, 2));
        }
    }
    Some((parse_cardinal_word(first)?, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_word_value_inverts_every_spelling() {
        for value in 0..=100 {
            let word = cardinal_word(value).unwrap();
            assert_eq!(parse_cardinal_word(&word), Some(value), "{word}");
        }
        for word in [
            "twenty-",
            "-one",
            "twenty-zero",
            "twenty-ten",
            "one-twenty",
            "hundred",
            "",
        ] {
            assert_eq!(parse_cardinal_word(word), None, "{word}");
        }
    }

    #[test]
    fn cardinal_word_covers_zero_to_one_hundred() {
        assert_eq!(cardinal_word(0).as_deref(), Some("zero"));
        assert_eq!(cardinal_word(1).as_deref(), Some("one"));
        assert_eq!(cardinal_word(12).as_deref(), Some("twelve"));
        assert_eq!(cardinal_word(20).as_deref(), Some("twenty"));
        assert_eq!(cardinal_word(21).as_deref(), Some("twenty-one"));
        assert_eq!(cardinal_word(42).as_deref(), Some("forty-two"));
        assert_eq!(cardinal_word(100).as_deref(), Some("one hundred"));
        assert_eq!(cardinal_word(101), None);
    }

    #[test]
    fn parse_cardinal_words_accepts_common_forms() {
        assert_eq!(parse_cardinal_word("three"), Some(3));
        assert_eq!(parse_cardinal_word("21"), Some(21));
        assert_eq!(parse_cardinal_word("101"), Some(101));
        assert_eq!(parse_cardinal_word("twenty-one"), Some(21));
        assert_eq!(parse_cardinal_words(&["twenty", "one"]), Some((21, 2)));
        assert_eq!(parse_cardinal_words(&["one", "hundred"]), Some((100, 2)));
        assert_eq!(parse_cardinal_words(&["a", "hundred"]), Some((100, 2)));
        assert_eq!(parse_cardinal_word("a"), Some(1));
        assert_eq!(parse_cardinal_word("an"), Some(1));
    }
}
