pub(crate) fn slice_starts_with<T: PartialEq>(items: &[T], prefix: &[T]) -> bool {
    items.len() >= prefix.len() && items[..prefix.len()] == *prefix
}

pub(crate) fn slice_ends_with<T: PartialEq>(items: &[T], suffix: &[T]) -> bool {
    items.len() >= suffix.len() && items[items.len() - suffix.len()..] == *suffix
}

pub(crate) fn slice_contains<T: PartialEq>(items: &[T], expected: &T) -> bool {
    items.iter().any(|item| item == expected)
}

pub(crate) fn slice_contains_str(items: &[&str], expected: &str) -> bool {
    items.iter().any(|item| *item == expected)
}

pub(crate) fn slice_contains_any<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    expected
        .iter()
        .any(|candidate| slice_contains(items, candidate))
}

pub(crate) fn slice_contains_all<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    expected
        .iter()
        .all(|candidate| slice_contains(items, candidate))
}

pub(crate) fn iter_contains<I, T>(items: I, expected: &T) -> bool
where
    I: IntoIterator,
    I::Item: Borrow<T>,
    T: PartialEq + ?Sized,
{
    items.into_iter().any(|item| item.borrow() == expected)
}

pub(crate) fn slice_strip_prefix<'a, T: PartialEq>(
    items: &'a [T],
    prefix: &[T],
) -> Option<&'a [T]> {
    slice_starts_with(items, prefix).then(|| &items[prefix.len()..])
}

pub(crate) fn slice_strip_suffix<'a, T: PartialEq>(
    items: &'a [T],
    suffix: &[T],
) -> Option<&'a [T]> {
    slice_ends_with(items, suffix).then(|| &items[..items.len() - suffix.len()])
}

pub(crate) fn find_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn find_str_index(items: &[&str], expected: &str) -> Option<usize> {
    find_index(items, |item| *item == expected)
}

pub(crate) fn find_str_by(
    items: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn find_any_str_index(items: &[&str], expected: &[&str]) -> Option<usize> {
    find_index(items, |item| {
        expected.iter().any(|candidate| *item == *candidate)
    })
}

pub(crate) fn rfind_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate().rev() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn rfind_str_by(
    items: &[&str],
    mut predicate: impl FnMut(&str) -> bool,
) -> Option<usize> {
    for (idx, item) in items.iter().enumerate().rev() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn find_window_index<T: PartialEq>(items: &[T], window: &[T]) -> Option<usize> {
    if window.is_empty() {
        return Some(0);
    }
    if items.len() < window.len() {
        return None;
    }
    let mut start = 0usize;
    while start + window.len() <= items.len() {
        if items[start..start + window.len()] == *window {
            return Some(start);
        }
        start += 1;
    }
    None
}

pub(crate) fn find_window_by<T>(
    items: &[T],
    window_len: usize,
    mut predicate: impl FnMut(&[T]) -> bool,
) -> Option<usize> {
    if window_len == 0 {
        return Some(0);
    }
    if items.len() < window_len {
        return None;
    }
    let mut start = 0usize;
    while start + window_len <= items.len() {
        if predicate(&items[start..start + window_len]) {
            return Some(start);
        }
        start += 1;
    }
    None
}

pub(crate) fn contains_window<T: PartialEq>(items: &[T], window: &[T]) -> bool {
    find_window_index(items, window).is_some()
}

pub(crate) fn str_contains(text: &str, needle: &str) -> bool {
    text.contains(needle)
}

pub(crate) fn str_starts_with(text: &str, prefix: &str) -> bool {
    text.starts_with(prefix)
}

pub(crate) fn str_starts_with_char(text: &str, expected: char) -> bool {
    text.starts_with(expected)
}

pub(crate) fn str_ends_with(text: &str, suffix: &str) -> bool {
    text.ends_with(suffix)
}

pub(crate) fn str_ends_with_char(text: &str, expected: char) -> bool {
    text.ends_with(expected)
}

pub(crate) fn str_find(text: &str, needle: &str) -> Option<usize> {
    text.find(needle)
}

pub(crate) fn str_find_char(text: &str, needle: char) -> Option<usize> {
    text.find(needle)
}

pub(crate) fn str_strip_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.strip_prefix(prefix)
}

pub(crate) fn str_strip_suffix<'a>(text: &'a str, suffix: &str) -> Option<&'a str> {
    text.strip_suffix(suffix)
}

pub(crate) fn str_split_once<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    text.split_once(needle)
}

pub(crate) fn str_split_once_char<'a>(text: &'a str, needle: char) -> Option<(&'a str, &'a str)> {
    text.split_once(needle)
}
use std::borrow::Borrow;
