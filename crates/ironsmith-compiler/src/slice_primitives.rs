use std::borrow::Borrow;

pub fn starts_with<T: PartialEq>(items: &[T], prefix: &[T]) -> bool {
    items.len() >= prefix.len() && items[..prefix.len()] == *prefix
}

pub fn ends_with<T: PartialEq>(items: &[T], suffix: &[T]) -> bool {
    items.len() >= suffix.len() && items[items.len() - suffix.len()..] == *suffix
}

pub fn contains<T: PartialEq>(items: &[T], expected: &T) -> bool {
    items.iter().any(|item| item == expected)
}

pub fn contains_any<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    expected.iter().any(|candidate| contains(items, candidate))
}

pub fn contains_all<T: PartialEq>(items: &[T], expected: &[T]) -> bool {
    expected.iter().all(|candidate| contains(items, candidate))
}

pub fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) -> bool {
    if contains(items, &item) {
        false
    } else {
        items.push(item);
        true
    }
}

pub fn equals_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    patterns.contains(&items)
}

pub fn matching_value<T: PartialEq, V: Clone>(items: &[T], patterns: &[(&[T], V)]) -> Option<V> {
    patterns
        .iter()
        .find_map(|(pattern, value)| (items == *pattern).then(|| value.clone()))
}

pub fn starts_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    patterns.iter().any(|pattern| starts_with(items, pattern))
}

pub fn ends_with_any<T: PartialEq>(items: &[T], patterns: &[&[T]]) -> bool {
    patterns.iter().any(|pattern| ends_with(items, pattern))
}

pub fn iter_contains<I, T>(items: I, expected: &T) -> bool
where
    I: IntoIterator,
    I::Item: Borrow<T>,
    T: PartialEq + ?Sized,
{
    items.into_iter().any(|item| item.borrow() == expected)
}

pub fn iter_eq<I, J>(left: I, right: J) -> bool
where
    I: IntoIterator,
    J: IntoIterator,
    I::Item: PartialEq<J::Item>,
{
    left.into_iter().eq(right)
}

pub fn strip_prefix<'a, T: PartialEq>(items: &'a [T], prefix: &[T]) -> Option<&'a [T]> {
    starts_with(items, prefix).then(|| &items[prefix.len()..])
}

pub fn strip_suffix<'a, T: PartialEq>(items: &'a [T], suffix: &[T]) -> Option<&'a [T]> {
    ends_with(items, suffix).then(|| &items[..items.len() - suffix.len()])
}

pub fn strip_any_prefix<'a, 'p, T: PartialEq>(
    items: &'a [T],
    patterns: &'p [&'p [T]],
) -> Option<(&'p [T], &'a [T])> {
    patterns
        .iter()
        .find_map(|pattern| strip_prefix(items, pattern).map(|tail| (*pattern, tail)))
}

pub fn strip_any_suffix<'a, 'p, T: PartialEq>(
    items: &'a [T],
    patterns: &'p [&'p [T]],
) -> Option<(&'p [T], &'a [T])> {
    patterns
        .iter()
        .find_map(|pattern| strip_suffix(items, pattern).map(|head| (*pattern, head)))
}

pub fn find_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub fn rfind_index<T>(items: &[T], mut predicate: impl FnMut(&T) -> bool) -> Option<usize> {
    for (idx, item) in items.iter().enumerate().rev() {
        if predicate(item) {
            return Some(idx);
        }
    }
    None
}

pub fn find_window_index<T: PartialEq>(items: &[T], window: &[T]) -> Option<usize> {
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

pub fn find_window_by<T>(
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

pub fn contains_sequence<T: PartialEq>(items: &[T], window: &[T]) -> bool {
    find_window_index(items, window).is_some()
}
