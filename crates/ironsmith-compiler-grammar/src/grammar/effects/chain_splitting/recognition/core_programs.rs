use super::*;

pub(super) fn exact_tail_from_any_word(
    tokens: &[OwnedLexToken],
    words: &'static [&'static str],
    tails: &'static [&'static [&'static str]],
) -> bool {
    find_any_word(tokens, words).is_some_and(|(idx, _, _)| exact_any(&tokens[idx..], tails))
}

pub(super) fn find_any_word<'a>(
    tokens: &'a [OwnedLexToken],
    words: &'static [&'static str],
) -> Option<(usize, (), &'a [OwnedLexToken])> {
    primitives::find_prefix(tokens, || {
        move |input: &mut LexStream<'a>| {
            for word in words {
                let mut probe = input.clone();
                if primitives::kw(word).parse_next(&mut probe).is_ok() {
                    *input = probe;
                    return Ok(());
                }
            }
            Err(ErrMode::Backtrack(ContextError::new()))
        }
    })
}

pub(super) fn starts_any(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::match_word_prefix(tokens, phrase).is_some())
}

pub(super) fn ends_any(tokens: &[OwnedLexToken], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::match_word_suffix(tokens, phrase).is_some())
}

pub(super) fn exact_any(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    phrases.iter().any(|phrase| {
        primitives::parse_all(
            tokens,
            (primitives::phrase(phrase), primitives::sentence_end()).void(),
            "chain exact phrase",
        )
        .is_ok()
    })
}

pub(super) fn has_any_phrase(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    phrases
        .iter()
        .any(|phrase| primitives::find_phrase_start(tokens, phrase).is_some())
}

pub(super) fn contains_any(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    words
        .iter()
        .any(|word| primitives::contains_word(tokens, word))
}

pub(super) fn contains_all(tokens: &[OwnedLexToken], words: &'static [&'static str]) -> bool {
    words
        .iter()
        .all(|word| primitives::contains_word(tokens, word))
}

pub(super) fn first_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if let Some(word) = token.as_word() {
            return Some(word);
        }
    }
}

pub(super) fn nth_word(tokens: &[OwnedLexToken], wanted: usize) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut index = 0usize;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = parsed.ok()?;
        if let Some(word) = token.as_word() {
            if index == wanted {
                return Some(word);
            }
            index += 1;
        }
    }
}

pub(super) fn last_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut last = None;
    loop {
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            return last;
        };
        if let Some(word) = token.as_word() {
            last = Some(word);
        }
    }
}

pub(super) fn is_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}
