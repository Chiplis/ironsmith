use super::*;

pub(super) fn counter_group_removed<'a>(input: &mut LexStream<'a>) -> WResult<u32> {
    opt(primitives::kw("for")).parse_next(input)?;
    primitives::kw("each").parse_next(input)?;
    let group_size = leaf::parse_leaf_number_token_lexed.parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        (
            alt((primitives::kw("counter"), primitives::kw("counters"))),
            primitives::phrase(&["removed", "this", "way"]),
        )
            .void(),
    )
    .parse_next(input)?;
    Ok(group_size)
}

pub fn parse_counter_group_removed_shape(
    tokens: &[OwnedLexToken],
) -> Option<CounterGroupRemovedShape<'_>> {
    let (group_size, effect_tokens) = primitives::parse_prefix(tokens, counter_group_removed)?;
    Some(CounterGroupRemovedShape {
        group_size,
        effect_tokens: trim_lexed_commas(effect_tokens),
    })
}
