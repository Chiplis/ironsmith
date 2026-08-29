use super::*;

pub(super) fn first_token_kind_from(
    tokens: &[OwnedLexToken],
    start: usize,
    expected: TokenKind,
) -> Option<usize> {
    let mut input = LexStream::new(tokens.get(start..)?);
    let initial_len = input.len();
    while let Ok(token) = take_token(&mut input) {
        if token.kind == expected {
            return Some(start + initial_len.saturating_sub(input.len() + 1));
        }
    }
    None
}

pub(super) fn token_phrase_prefix(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static str],
) -> bool {
    let mut input = LexStream::new(trim_anthem_clause_tokens(tokens));
    primitives::phrase(expected).parse_next(&mut input).is_ok()
}

pub(super) fn token_phrase_complete(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static str],
) -> bool {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(tokens, primitives::phrase(expected), "anthem-exact-phrase").is_ok()
}

pub(super) fn token_any_phrase_complete(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static [&'static str]],
) -> bool {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        primitives::any_phrase(expected),
        "anthem-exact-alternative",
    )
    .is_ok()
}
