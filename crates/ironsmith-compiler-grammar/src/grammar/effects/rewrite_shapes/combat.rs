use super::*;

pub(super) fn parse_cant_be_blocked(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("can't"), primitives::kw("cant")))
        .void()
        .parse_next(input)?;
    primitives::phrase(&["be", "blocked"])
        .void()
        .parse_next(input)
}

pub(super) fn parse_cant_blocked_base_power_toughness_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantBlockedBasePowerToughnessShape<'a>> {
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_cant_be_blocked))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    parse_cant_be_blocked.parse_next(input)?;
    alt((
        primitives::phrase(&["this", "turn"]),
        primitives::phrase(&["until", "end", "of", "turn"]),
        primitives::phrase(&["until", "the", "end", "of", "turn"]),
    ))
    .void()
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    alt((primitives::kw("has"), primitives::kw("have"))).parse_next(input)?;
    primitives::phrase(&["base", "power", "and", "toughness"]).parse_next(input)?;
    let modifier = primitives::word_parser_text.parse_next(input)?;
    let (power, toughness) = super::super::super::leaf::parse_leaf_pt_modifier_values_complete(
        modifier,
    )
    .map_err(|_| {
        primitives::backtrack_err(
            "cant-be-blocked base power/toughness",
            "power/toughness value",
        )
    })?;
    alt((
        primitives::phrase(&["until", "end", "of", "turn"]),
        primitives::phrase(&["until", "the", "end", "of", "turn"]),
    ))
    .void()
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.void().parse_next(input)?;

    Ok(CantBlockedBasePowerToughnessShape {
        subject_tokens,
        power,
        toughness,
    })
}

pub fn parse_cant_blocked_base_power_toughness_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CantBlockedBasePowerToughnessShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_cant_blocked_base_power_toughness_lexed,
        "cant-be-blocked base-power/toughness",
    )
    .ok()
}
