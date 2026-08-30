use super::*;

pub(super) fn token_is_ignored(token: &OwnedLexToken) -> bool {
    primitives::parse_prefix(
        std::slice::from_ref(token),
        alt((
            primitives::kw("and"),
            primitives::kw("tapped"),
            primitives::kw("attacking"),
            primitives::kw("face"),
            primitives::kw("down"),
        ))
        .void(),
    )
    .is_some()
}

pub(super) fn word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut words = Vec::new();
    for token in tokens {
        if token.as_word().is_some()
            && !permission_shapes::exact_tokens_any(
                std::slice::from_ref(token),
                &[&["a"], &["an"], &["the"]],
            )
        {
            words.push(token.clone());
        }
    }
    words
}
