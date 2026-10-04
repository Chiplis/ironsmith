//! Complete unit-fraction prefixes shared by resource quantities.

/// `half` or `a/one <ordinal> of`, retaining the denominator rather than
/// mistaking the article for the integer one.
pub fn unit_fraction_prefix(words: &[&str]) -> Option<(i32, usize)> {
    if words.first() == Some(&"half") {
        return Some((2, 1));
    }
    if !matches!(words.first(), Some(&"a" | &"one")) {
        return None;
    }
    let (denominator, used) = ironsmith_core::parse_ordinal_words(&words[1..])?;
    if denominator < 2 || words.get(used + 1) != Some(&"of") {
        return None;
    }
    Some((i32::try_from(denominator).ok()?, used + 2))
}

pub fn without_rounding_suffix<'a>(words: &'a [&'a str]) -> (&'a [&'a str], bool) {
    if let Some(body) = words.strip_suffix(&["rounded", "up"]) {
        (body, true)
    } else {
        (
            words.strip_suffix(&["rounded", "down"]).unwrap_or(words),
            false,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractions_are_denominators_not_cardinal_article_counts() {
        assert_eq!(
            unit_fraction_prefix(&["a", "third", "of", "their", "life"]),
            Some((3, 3))
        );
        assert_eq!(
            unit_fraction_prefix(&["one", "fourth", "of", "your", "life"]),
            Some((4, 3))
        );
        assert_eq!(
            unit_fraction_prefix(&["half", "your", "life"]),
            Some((2, 1))
        );
        for words in [
            &["a", "first", "of", "their", "life"][..],
            &["a", "third", "creature"],
            &["two", "thirds", "of", "their", "life"],
        ] {
            assert!(unit_fraction_prefix(words).is_none());
        }
    }
}
