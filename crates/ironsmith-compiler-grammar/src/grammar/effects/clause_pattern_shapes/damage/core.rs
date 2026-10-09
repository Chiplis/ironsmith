use super::*;

pub(super) fn next_time_tail<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    alt((
        primitives::phrase(&["that", "damage", "is", "dealt", "to"]),
        primitives::phrase(&["that", "source", "deals", "that", "damage", "to"]),
        primitives::phrase(&["that", "spell", "deals", "that", "damage", "to"]),
        primitives::phrase(&["that", "creature", "deals", "that", "damage", "to"]),
        primitives::phrase(&["it", "deals", "that", "damage", "to"]),
    ))
    .parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(destination_tokens)
}

pub(super) fn parse_next_time<'a>(input: &mut LexStream<'a>) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next", "time"]).parse_next(input)?;
    let (source, combat_only) = if peek((opt(primitives::kw("combat")), primitives::phrase(&["damage", "would", "be", "dealt"])))
        .parse_next(input).is_ok()
    {
        let combat = opt(primitives::kw("combat")).parse_next(input)?.is_some();
        primitives::phrase(&["damage", "would", "be", "dealt", "to"]).parse_next(input)?;
        (DamageSourceShape::Filter(ObjectFilter::default()), combat)
    } else {
        let source_tokens = one_or_more_tokens_before(input, primitives::kw("would").void())?;
        primitives::phrase(&["would", "deal"]).parse_next(input)?;
        let combat = opt(primitives::kw("combat")).parse_next(input)?.is_some();
        primitives::kw("damage").parse_next(input)?;
        opt(primitives::kw("to")).parse_next(input)?;
        (classify_damage_source(source_tokens)
            .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?, combat)
    };
    let target_tokens = tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let destination_tokens = next_time_tail.parse_next(input)?;
    let destination = classify_next_time_destination(destination_tokens)
        .ok_or_else(|| winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new()))?;
    Ok(RedirectNextDamageShape::NextTime { source, combat_only, target_tokens, destination })
}

pub(super) fn parse_next_amount<'a>(input: &mut LexStream<'a>) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let amount_tokens = any.void().take().parse_next(input)?;
    primitives::phrase(&["damage", "that", "would", "be", "dealt"]).parse_next(input)?;
    let early_duration = opt(primitives::phrase(&["this", "turn"])).parse_next(input)?.is_some();
    primitives::kw("to").parse_next(input)?;
    let protected_tokens = if early_duration {
        one_or_more_tokens_before(input, primitives::phrase(&["is", "dealt", "to"]))?
    } else {
        let protected = one_or_more_tokens_before(input, primitives::phrase(&["this", "turn"]))?;
        primitives::phrase(&["this", "turn"]).parse_next(input)?;
        protected
    };
    let protected_tokens = if primitives::parse_all(protected_tokens,
        (source_reference, winnow::combinator::eof).void(), "redirect protected source").is_ok()
    { None } else { Some(protected_tokens) };
    primitives::phrase(&["is", "dealt", "to"]).parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::NextAmount {
        amount_tokens, protected_tokens,
        destination: classify_next_amount_destination(destination_tokens),
        source_of_your_choice: false,
    })
}

/// "The next N damage that a source of your choice would deal to <recipient>
/// this turn is dealt to <destination> instead." (Harm's Way, Shining Shoal):
/// CR 614.9 redirection restricted to one source chosen on resolution.
pub(super) fn parse_next_amount_by_chosen_source<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let amount_tokens = any.void().take().parse_next(input)?;
    primitives::phrase(&["damage", "that"]).parse_next(input)?;
    source_of_your_choice.parse_next(input)?;
    primitives::phrase(&["would", "deal", "to"]).parse_next(input)?;
    let protected_tokens = one_or_more_tokens_before(input, primitives::phrase(&["this", "turn"]))?;
    primitives::phrase(&["this", "turn", "is", "dealt", "to"]).parse_next(input)?;
    let destination_tokens = one_or_more_tokens_before(input, primitives::kw("instead").void())?;
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RedirectNextDamageShape::NextAmount {
        amount_tokens,
        protected_tokens: Some(protected_tokens),
        destination: classify_next_amount_destination(destination_tokens),
        source_of_your_choice: true,
    })
}
