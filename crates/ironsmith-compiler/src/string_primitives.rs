pub(crate) fn contains(text: &str, needle: &str) -> bool {
    text.contains(needle)
}

pub(crate) fn contains_char(text: &str, needle: char) -> bool {
    text.contains(needle)
}

pub(crate) fn starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

pub(crate) fn starts_with_char(text: &str, expected: char) -> bool {
    text.starts_with(expected)
}

pub(crate) fn ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

pub(crate) fn ends_with_char(text: &str, expected: char) -> bool {
    text.ends_with(expected)
}

pub(crate) fn ends_with_any_char(text: &str, expected: &[char]) -> bool {
    text.chars()
        .next_back()
        .is_some_and(|ch| expected.contains(&ch))
}

pub(crate) fn find(text: &str, needle: &str) -> Option<usize> {
    text.find(needle)
}

pub(crate) fn find_char(text: &str, needle: char) -> Option<usize> {
    text.find(needle)
}

pub(crate) fn rfind(text: &str, needle: &str) -> Option<usize> {
    text.rfind(needle)
}

pub(crate) fn rfind_char(text: &str, needle: char) -> Option<usize> {
    text.rfind(needle)
}

pub(crate) fn strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)
}

pub(crate) fn strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    text.strip_suffix(suffix)
}

pub(crate) fn strip_suffix_char(text: &str, suffix: char) -> Option<&str> {
    let (last_idx, last_char) = text.char_indices().next_back()?;
    (last_char == suffix).then_some(&text[..last_idx])
}

pub(crate) fn split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    text.split_once(needle)
}

pub(crate) fn split_once_char(text: &str, needle: char) -> Option<(&str, &str)> {
    text.split_once(needle)
}
