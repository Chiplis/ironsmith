//! English cardinal helpers shared by parser and renderer code.

fn small_cardinal_word(n: u32) -> Option<&'static str> {
    match n {
        0 => Some("zero"),
        1 => Some("one"),
        2 => Some("two"),
        3 => Some("three"),
        4 => Some("four"),
        5 => Some("five"),
        6 => Some("six"),
        7 => Some("seven"),
        8 => Some("eight"),
        9 => Some("nine"),
        10 => Some("ten"),
        11 => Some("eleven"),
        12 => Some("twelve"),
        13 => Some("thirteen"),
        14 => Some("fourteen"),
        15 => Some("fifteen"),
        16 => Some("sixteen"),
        17 => Some("seventeen"),
        18 => Some("eighteen"),
        19 => Some("nineteen"),
        _ => None,
    }
}

fn tens_cardinal_word(tens: u32) -> Option<&'static str> {
    match tens {
        2 => Some("twenty"),
        3 => Some("thirty"),
        4 => Some("forty"),
        5 => Some("fifty"),
        6 => Some("sixty"),
        7 => Some("seventy"),
        8 => Some("eighty"),
        9 => Some("ninety"),
        _ => None,
    }
}

/// Return the English cardinal phrase for numbers 0 through 100.
pub fn cardinal_word(n: u32) -> Option<String> {
    match n {
        0..=19 => small_cardinal_word(n).map(str::to_string),
        20..=99 => {
            let tens = n / 10;
            let ones = n % 10;
            if ones == 0 {
                tens_cardinal_word(tens).map(str::to_string)
            } else {
                Some(format!(
                    "{}-{}",
                    tens_cardinal_word(tens)?,
                    small_cardinal_word(ones)?
                ))
            }
        }
        100 => Some("one hundred".to_string()),
        _ => None,
    }
}

fn parse_numeric_cardinal_word(word: &str) -> Option<u32> {
    word.parse::<u32>().ok()
}

fn parse_single_cardinal_word(word: &str) -> Option<u32> {
    let word = word.to_ascii_lowercase();
    if matches!(word.as_str(), "a" | "an") {
        return Some(1);
    }
    if let Some(value) = parse_numeric_cardinal_word(&word) {
        return Some(value);
    }
    (0..=100).find(|&value| cardinal_word(value).as_deref() == Some(word.as_str()))
}

/// Parse one cardinal token, accepting hyphenated words and raw numeric forms.
pub fn parse_cardinal_word(word: &str) -> Option<u32> {
    parse_single_cardinal_word(word)
}

/// Parse a cardinal phrase from the start of a word slice.
///
/// Returns the cardinal value and the number of consumed words. This accepts
/// hyphenated forms like `twenty-one`, split forms like `twenty one`, and
/// `one hundred`.
pub fn parse_cardinal_words(words: &[&str]) -> Option<(u32, usize)> {
    let first = words.first().copied()?;
    if words.len() >= 2 {
        let first = first.to_ascii_lowercase();
        let second = words[1].to_ascii_lowercase();
        if matches!(first.as_str(), "a" | "an" | "one") && second == "hundred" {
            return Some((100, 2));
        }
        for tens in 2..=9 {
            if tens_cardinal_word(tens) == Some(first.as_str()) {
                for ones in 1..=9 {
                    if small_cardinal_word(ones) == Some(second.as_str()) {
                        return Some((tens * 10 + ones, 2));
                    }
                }
            }
        }
    }

    if let Some(value) = parse_cardinal_word(first) {
        return Some((value, 1));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
