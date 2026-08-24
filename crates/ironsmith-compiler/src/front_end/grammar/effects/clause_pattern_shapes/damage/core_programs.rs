use super::*;

pub(super) fn next_time_tail<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    alt((
        primitives::phrase(&["that", "damage", "is", "dealt", "to"]),
        primitives::phrase(&["that", "source", "deals", "that", "damage", "to"]),
    ))
    .parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(destination_tokens)
}

pub(super) fn parse_next_time<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next", "time"]).parse_next(input)?;
    let source_tokens = one_or_more_tokens_before(input, primitives::kw("would").void())?;
    primitives::phrase(&["would", "deal", "damage", "to"]).parse_next(input)?;
    let target_tokens = one_or_more_tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let destination_tokens = next_time_tail.parse_next(input)?;
    let destination = classify_next_time_destination(destination_tokens)
        .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?;
    Ok(RedirectNextDamageShape::NextTime {
        source: classify_damage_source(source_tokens)
            .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?,
        target_tokens,
        destination,
    })
}

pub(super) fn parse_next_amount<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let amount_tokens = any.void().take().parse_next(input)?;
    primitives::phrase(&["damage", "that", "would", "be", "dealt", "to"]).parse_next(input)?;
    let protected_shape = if peek((source_reference, primitives::phrase(&["this", "turn"])))
        .parse_next(input)
        .is_ok()
    {
        source_reference.parse_next(input)?;
        None
    } else {
        Some(one_or_more_tokens_before(
            input,
            primitives::phrase(&["this", "turn"]),
        )?)
    };
    primitives::phrase(&["this", "turn", "is", "dealt", "to"]).parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::NextAmount {
        amount_tokens,
        protected_tokens: protected_shape,
        destination: classify_next_amount_destination(destination_tokens),
    })
}
