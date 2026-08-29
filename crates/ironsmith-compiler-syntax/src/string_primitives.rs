pub fn contains(text: &str, needle: &str) -> bool {
    text.contains(needle)
}

pub fn contains_char(text: &str, needle: char) -> bool {
    text.contains(needle)
}

pub fn starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

pub fn starts_with_char(text: &str, expected: char) -> bool {
    text.starts_with(expected)
}

pub fn ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

pub fn ends_with_char(text: &str, expected: char) -> bool {
    text.ends_with(expected)
}

pub fn ends_with_any_char(text: &str, expected: &[char]) -> bool {
    text.chars()
        .next_back()
        .is_some_and(|ch| expected.contains(&ch))
}

pub fn find(text: &str, needle: &str) -> Option<usize> {
    text.find(needle)
}

pub fn find_char(text: &str, needle: char) -> Option<usize> {
    text.find(needle)
}

pub fn rfind(text: &str, needle: &str) -> Option<usize> {
    text.rfind(needle)
}

pub fn rfind_char(text: &str, needle: char) -> Option<usize> {
    text.rfind(needle)
}

pub fn strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)
}

pub fn strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    text.strip_suffix(suffix)
}

pub fn strip_suffix_char(text: &str, suffix: char) -> Option<&str> {
    let (last_idx, last_char) = text.char_indices().next_back()?;
    (last_char == suffix).then_some(&text[..last_idx])
}

pub fn split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    text.split_once(needle)
}

pub fn split_once_char(text: &str, needle: char) -> Option<(&str, &str)> {
    text.split_once(needle)
}
