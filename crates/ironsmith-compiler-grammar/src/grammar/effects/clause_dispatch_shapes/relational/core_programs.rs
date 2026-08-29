use super::*;

pub fn parse_passive_goad_shape(tokens: &[OwnedLexToken]) -> Option<PassiveGoadShape<'_>> {
    let (subject_tokens, tail_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::kw("is").void())?;
    let for_rest_of_game = primitives::parse_all(
        trim_lexed_commas(tail_tokens),
        (
            alt((primitives::kw("goaded"), primitives::kw("goad"))),
            opt(primitives::any_phrase(&[
                &["for", "the", "rest", "of", "the", "game"],
                &["for", "the", "rest", "of", "this", "game"],
            ]))
            .map(|duration| duration.is_some()),
            primitives::sentence_end(),
        )
            .map(|(_, for_rest_of_game, _)| for_rest_of_game),
        "passive goad shape",
    )
    .ok()?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    if subject_tokens.is_empty() {
        return None;
    }
    let tagged = primitives::parse_all(
        subject_tokens,
        (
            primitives::any_phrase(&[&["the", "token"], &["the", "tokens"]]),
            primitives::sentence_end(),
        )
            .void(),
        "goad token reference",
    )
    .is_ok();
    Some(PassiveGoadShape {
        target: if tagged {
            GoadTargetShape::TaggedToken
        } else {
            GoadTargetShape::Target(subject_tokens)
        },
        for_rest_of_game,
    })
}
