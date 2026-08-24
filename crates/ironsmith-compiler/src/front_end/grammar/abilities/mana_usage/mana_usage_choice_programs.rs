use super::*;

pub(super) fn first_phrase_choice<'a>(
    words: &[&str],
    phrases: &'a [&'a [&'a str]],
) -> Option<(&'a [&'a str], usize)> {
    let mut best = None;
    for phrase in phrases {
        let Some(offset) = phrase_offset_words(words, phrase) else {
            continue;
        };
        if best.is_none_or(|(_, current)| offset < current) {
            best = Some((*phrase, offset));
        }
    }
    best
}

pub(super) fn first_word_choice(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut best = None;
    for word in expected {
        let Some(offset) = phrase_offset_words(words, &[*word]) else {
            continue;
        };
        best = Some(best.map_or(offset, |current: usize| current.min(offset)));
    }
    best
}
