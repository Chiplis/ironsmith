use super::*;

pub(super) fn strip_article(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut input = LexStream::new(tokens);
    if alt((primitives::kw("a"), primitives::kw("an")))
        .parse_next(&mut input)
        .is_err()
    {
        return tokens;
    }
    &tokens[tokens.len().saturating_sub(input.len())..]
}

pub(super) fn parse_any_prefix_word_count(words: &[&str], phrases: &[&[&str]]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_any_prefix_words(&mut input, phrases)?;
    words.len().checked_sub(input.len())
}

pub(super) fn parse_any_prefix_words<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    phrases: &[&[&str]],
) -> Option<()> {
    for phrase in phrases {
        let mut probe = *input;
        if parse_phrase_words(&mut probe, phrase).is_ok() {
            *input = probe;
            return Some(());
        }
    }
    None
}

pub(super) fn last_exact_suffix_offset(words: &[&str], tails: &[&[&str]]) -> Option<usize> {
    for start in (0..words.len()).rev() {
        for tail in tails {
            if matches_exact_word_slice(words.get(start..)?, tail) {
                return Some(start);
            }
        }
    }
    None
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
