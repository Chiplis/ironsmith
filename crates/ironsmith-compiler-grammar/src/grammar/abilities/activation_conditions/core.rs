use super::*;

pub(super) fn exact_tail_offset(words: &[&str], tails: &[&[&str]]) -> Option<usize> {
    let mut best = None;
    for tail in tails {
        let Some(offset) = phrase_offset_words(words, tail) else {
            continue;
        };
        if !matches_exact_word_slice(words.get(offset..)?, tail) {
            continue;
        }
        best = Some(best.map_or(offset, |current: usize| current.min(offset)));
    }
    best
}

pub(super) fn matches_exact_word_slice(words: &[&str], phrase: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    (
        |input: &mut primitives::WordSliceInput<'_>| parse_phrase_words(input, phrase),
        eof,
    )
        .void()
        .parse_next(&mut input)
        .is_ok()
}
