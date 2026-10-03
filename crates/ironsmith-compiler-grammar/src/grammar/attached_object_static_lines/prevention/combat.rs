use super::*;

pub(super) fn parse_general_put_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterPreventionSpec<'a>> {
    let (((), source_tokens), display_prefix_tokens) = (
        semantic_phrase(&["if", "damage", "would", "be", "dealt", "to"]),
        repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(alt((
                semantic_kw("while"),
                semantic_kw("prevent"),
                semantic_kw("put"),
            ))),
        )
        .map(|((), _)| ())
        .take(),
    )
        .with_taken()
        .parse_next(input)?;
    validate_source_reference(trim_lexed_commas(source_tokens))?;
    let condition_tokens = if peek(semantic_kw("while")).parse_next(input).is_ok() {
        semantic_kw("while").parse_next(input)?;
        let condition_tokens = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(alt((semantic_kw("prevent"), semantic_kw("put")))),
        )
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
        Some(trim_lexed_commas(condition_tokens))
    } else {
        None
    };
    let (prevents_damage, effect_tokens) = alt((
        (
            semantic_phrase(&[
                "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters",
                "on",
            ]),
            parse_counter_destination,
        )
            .value(true),
        (
            semantic_phrase(&["put", "that", "many", "+1/+1", "counters", "on"]),
            parse_counter_destination,
            semantic_kw("instead"),
        )
            .value(false),
    ))
    .with_taken()
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(PutCounterPreventionSpec::General {
        prevents_damage,
        condition_tokens,
        display_prefix_tokens: trim_lexed_commas(display_prefix_tokens),
        effect_tokens: trim_lexed_commas(effect_tokens),
    })
}

pub(super) fn parse_noncombat_put_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterPreventionSpec<'a>> {
    semantic_phrase(&[
        "if",
        "noncombat",
        "damage",
        "would",
        "be",
        "dealt",
        "to",
        "this",
        "creature",
        "prevent",
        "that",
        "damage",
        "put",
        "a",
        "+1/+1",
        "counter",
        "on",
        "this",
        "creature",
        "for",
        "each",
        "1",
        "damage",
        "prevented",
        "this",
        "way",
    ])
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(PutCounterPreventionSpec::Noncombat)
}

pub(super) fn parse_creature_combat_put_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterPreventionSpec<'a>> {
    semantic_phrase(&[
        "if", "a", "creature", "would", "deal", "combat", "damage", "to", "this", "creature",
        "prevent", "that", "damage", "and", "put", "a", "+1/+1", "counter", "on", "this",
        "creature",
    ])
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(PutCounterPreventionSpec::CreatureCombat)
}

/// The additional counter effect scales with actual prevented damage, not the
/// original unpreventable amount. Signed and named counters share this grammar.
pub(super) fn parse_per_prevented_amount_counter_prevention_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<PutCounterPreventionSpec<'a>> {
    semantic_phrase(&["if", "damage", "would", "be", "dealt", "to"]).parse_next(input)?;
    parse_this_source(input)?;
    semantic_phrase(&["prevent", "that", "damage"]).parse_next(input)?;
    opt(semantic_kw("and")).parse_next(input)?;
    semantic_kw("put").parse_next(input)?;
    alt((semantic_kw("a"), semantic_kw("an"))).parse_next(input)?;
    let counter_type = parse_counter_type_before_counter_noun(input, "per-prevented damage counter")?;
    semantic_kw("on").parse_next(input)?;
    parse_counter_destination(input)?;
    semantic_phrase(&["for", "each", "1", "damage", "prevented", "this", "way"]).parse_next(input)?;
    semantic_finish(input)?;
    Ok(PutCounterPreventionSpec::PerPreventedAmount { counter_type })
}
