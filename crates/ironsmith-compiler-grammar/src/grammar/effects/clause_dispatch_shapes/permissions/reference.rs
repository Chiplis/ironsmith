use super::*;

pub fn parse_cast_target_without_paying_shape(
    tokens: &[OwnedLexToken],
) -> Option<CastTargetWithoutPayingShape<'_>> {
    let (head, ()) = primitives::split_lexed_once_before_suffix(tokens, 2, || {
        (
            primitives::phrase(&["without", "paying", "its", "mana", "cost"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let (_, target_tokens) = primitives::parse_prefix(head, primitives::kw("cast"))?;
    primitives::parse_prefix(target_tokens, primitives::kw("target"))?;
    Some(CastTargetWithoutPayingShape {
        target_tokens: trim_lexed_commas(target_tokens),
    })
}

pub fn parse_cast_target_from_your_graveyard_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<CastTargetFromYourGraveyardThisTurnShape<'_>> {
    let (_, rest) = primitives::parse_prefix(
        trim_lexed_commas(tokens),
        alt((
            primitives::phrase(&["you", "may", "cast"]),
            primitives::kw("cast").void(),
        )),
    )?;
    let (target_tokens, ()) = primitives::split_lexed_once_before_suffix(rest, 2, || {
        (
            primitives::phrase(&["this", "turn"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    let target_tokens = trim_lexed_commas(target_tokens);
    let (_, target_body) = primitives::parse_prefix(target_tokens, primitives::kw("target"))?;
    if target_body.is_empty() {
        return None;
    }
    primitives::split_lexed_once_before_suffix(target_tokens, 2, || {
        (primitives::phrase(&["from", "your", "graveyard"]), eof).void()
    })?;
    Some(CastTargetFromYourGraveyardThisTurnShape { target_tokens })
}
