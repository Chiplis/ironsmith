use super::*;

pub fn is_next_cast_spell_or_loyalty_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trimmed(tokens),
        alt((
            semantic_phrase(&[
                "you", "next", "cast", "an", "instant", "spell", "cast", "a", "sorcery", "spell",
                "or", "activate", "a", "loyalty", "ability",
            ]),
            semantic_phrase(&[
                "you", "next", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate",
                "a", "loyalty", "ability",
            ]),
        )),
        "next spell-or-loyalty trigger",
    )
    .is_ok()
}

pub fn delayed_trigger_has_next_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(trimmed(tokens), || primitives::kw("next")).is_some()
}

pub fn delayed_trigger_has_first_time_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::split_lexed_once_before_suffix(trimmed(tokens), 1, || {
        (primitives::phrase(&["for", "the", "first", "time"]), eof).void()
    })
    .is_some()
}
