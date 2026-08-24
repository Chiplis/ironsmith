use super::*;

pub(super) fn parse_source_controller_graveyard_filter(
    words: &[&str],
) -> Option<crate::target::ObjectFilter> {
    const POSSESSIVE_GRAVEYARD_SUFFIXES: &[&[&str]] = &[
        &["in", "its", "controller", "graveyard"],
        &["in", "its", "controllers", "graveyard"],
    ];
    let suffix = POSSESSIVE_GRAVEYARD_SUFFIXES
        .iter()
        .find(|suffix| permission_shapes::suffix_words(words, suffix))?;
    let object_words = words.get(..words.len().checked_sub(suffix.len())?)?;
    (!object_words.is_empty())
        .then(|| parse_object_filter_words(object_words, false).ok())
        .flatten()
}
