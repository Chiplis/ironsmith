use super::*;

pub(super) fn exact_phrase_occurs(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_exact_phrase(&mut candidate, expected).is_ok() {
            return true;
        }
        if take_word_slice_any(&mut input).is_err() {
            return false;
        }
    }
}

pub(super) fn exact_word_occurs(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    while let Ok(word) = take_word_slice_any(&mut input) {
        if expected.contains(&word) {
            return true;
        }
    }
    false
}

pub(super) fn parse_normalized_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for word in expected {
        parse_normalized_word(input, word)?;
    }
    Ok(())
}

pub(super) fn parse_exact_phrase<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &[&str],
) -> WResult<()> {
    for expected_word in expected {
        let word = take_word_slice_any(input)?;
        if word != *expected_word {
            return Err(winnow::error::ErrMode::Backtrack(
                winnow::error::ContextError::new(),
            ));
        }
    }
    Ok(())
}

pub(super) fn parse_normalized_word<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &str,
) -> WResult<()> {
    let word = take_word_slice_any(input)?;
    if normalized_word_matches(word, expected) {
        Ok(())
    } else {
        Err(winnow::error::ErrMode::Backtrack(
            winnow::error::ContextError::new(),
        ))
    }
}

pub(super) fn normalized_word_matches(word: &str, expected: &str) -> bool {
    let mut input = word;
    let parsed: WResult<()> = (
        literal(expected),
        alt((
            eof.value(()),
            (literal("'s"), eof).void(),
            (literal("’s"), eof).void(),
            (literal("s'"), eof).void(),
            (literal("s’"), eof).void(),
        )),
    )
        .void()
        .parse_next(&mut input);
    parsed.is_ok()
}

pub(super) fn trim_commas_ref(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}
