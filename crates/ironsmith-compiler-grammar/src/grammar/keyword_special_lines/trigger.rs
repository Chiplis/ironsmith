use super::*;

pub(super) fn parse_optional_cost_with_cast_trigger_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<OptionalCostWithCastTriggerShape<'a>> {
    primitives::phrase(&[
        "as",
        "an",
        "additional",
        "cost",
        "to",
        "cast",
        "this",
        "spell",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;

    let label_tokens = (
        primitives::phrase(&["you", "may"]),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::period())).void(),
    )
        .take()
        .parse_next(input)?;
    let (_, optional_cost_effect_tokens) =
        primitives::parse_prefix(label_tokens, primitives::phrase(&["you", "may"]))
            .ok_or_else(|| primitives::backtrack_err("optional additional cost", "you may"))?;
    primitives::period().parse_next(input)?;
    // "When you do," and "When you cast this spell, if you paid this
    // additional cost," name the same payment-gated cast trigger.
    let trigger_frame_tokens = (
        alt((
            primitives::phrase(&["when", "you", "do"]),
            (
                primitives::phrase(&["when", "you", "cast", "this", "spell"]),
                opt(primitives::comma()),
                primitives::phrase(&["if", "you", "paid", "this", "additional", "cost"]),
            )
                .void(),
        )),
        opt(primitives::comma()),
    )
        .take()
        .parse_next(input)?;
    let followup_effect_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(OptionalCostWithCastTriggerShape {
        label_tokens,
        optional_cost_effect_tokens,
        trigger_frame_tokens,
        followup_effect_tokens,
    })
}
