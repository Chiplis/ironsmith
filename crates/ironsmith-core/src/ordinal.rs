//! English ordinal helpers shared by parser and renderer code.

fn small_ordinal_word(n: u32) -> Option<&'static str> {
    match n {
        1 => Some("first"),
        2 => Some("second"),
        3 => Some("third"),
        4 => Some("fourth"),
        5 => Some("fifth"),
        6 => Some("sixth"),
        7 => Some("seventh"),
        8 => Some("eighth"),
        9 => Some("ninth"),
        10 => Some("tenth"),
        11 => Some("eleventh"),
        12 => Some("twelfth"),
        13 => Some("thirteenth"),
        14 => Some("fourteenth"),
        15 => Some("fifteenth"),
        16 => Some("sixteenth"),
        17 => Some("seventeenth"),
        18 => Some("eighteenth"),
        19 => Some("nineteenth"),
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

fn tens_ordinal_word(tens: u32) -> Option<&'static str> {
    match tens {
        2 => Some("twentieth"),
        3 => Some("thirtieth"),
        4 => Some("fortieth"),
        5 => Some("fiftieth"),
        6 => Some("sixtieth"),
        7 => Some("seventieth"),
        8 => Some("eightieth"),
        9 => Some("ninetieth"),
        _ => None,
    }
}

/// Return the English ordinal phrase for numbers 1 through 100.
pub fn ordinal_word(n: u32) -> Option<String> {
    match n {
        1..=19 => small_ordinal_word(n).map(str::to_string),
        20..=99 => {
            let tens = n / 10;
            let ones = n % 10;
            if ones == 0 {
                tens_ordinal_word(tens).map(str::to_string)
            } else {
                Some(format!(
                    "{}-{}",
                    tens_cardinal_word(tens)?,
                    small_ordinal_word(ones)?
                ))
            }
        }
        100 => Some("one hundredth".to_string()),
        _ => None,
    }
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

fn parse_single_ordinal_word(word: &str) -> Option<u32> {
    let word = word.to_ascii_lowercase();
    if let Some(value) = parse_numeric_ordinal_word(&word) {
        return Some(value);
    }
    for value in 1..=100 {
        if ordinal_word(value).as_deref() == Some(word.as_str()) {
            return Some(value);
        }
    }
    if word == "hundredth" {
        return Some(100);
    }
    None
}

/// Parse one ordinal token, accepting hyphenated words and numeric forms.
pub fn parse_ordinal_word(word: &str) -> Option<u32> {
    parse_single_ordinal_word(word)
}

/// Parse an ordinal phrase from the start of a word slice.
///
/// Returns the ordinal value and the number of consumed words. This accepts
/// both hyphenated forms like `twenty-first` and split forms like
/// `twenty first`.
pub fn parse_ordinal_words(words: &[&str]) -> Option<(u32, usize)> {
    let first = words.first().copied()?;
    if let Some(value) = parse_ordinal_word(first) {
        return Some((value, 1));
    }

    if words.len() >= 2 {
        let first = first.to_ascii_lowercase();
        let second = words[1].to_ascii_lowercase();
        if first == "one" && second == "hundredth" {
            return Some((100, 2));
        }
        for tens in 2..=9 {
            if tens_cardinal_word(tens) == Some(first.as_str()) {
                for ones in 1..=9 {
                    if small_ordinal_word(ones) == Some(second.as_str()) {
                        return Some((tens * 10 + ones, 2));
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

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
