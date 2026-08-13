use super::*;

fn tap_control_relation<'a>(input: &mut LexStream<'a>) -> WResult<TapControlRelation> {
    alt((
        (
            primitives::phrase(&["target", "player"]),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .value(TapControlRelation::TargetPlayer),
        (
            alt((
                primitives::phrase(&["that", "player"]),
                primitives::phrase(&["that", "players"]),
            )),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .value(TapControlRelation::ThatPlayer),
    ))
    .parse_next(input)
}

fn parse_tap_control_relation_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TapControlRelation> {
    repeat_till(0.., any.void(), tap_control_relation)
        .map(|((), relation)| relation)
        .parse_next(input)
}

pub(crate) fn parse_tap_control_relation_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TapControlRelation> {
    primitives::parse_prefix(tokens, parse_tap_control_relation_lexed).map(|(relation, _)| relation)
}

fn type_choice_qualifier<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["of", "the", "chosen", "type"]),
        primitives::phrase(&["of", "chosen", "type"]),
        primitives::phrase(&["of", "that", "type"]),
        primitives::phrase(&["that", "type"]),
    ))
    .parse_next(input)
}

fn parse_tap_type_choice_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TapTypeChoiceShape<'a>> {
    let before_tokens = repeat_till(0.., any.void(), peek(type_choice_qualifier))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    type_choice_qualifier.parse_next(input)?;
    let after_tokens = repeat::<_, _, (), _, _>(0.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(TapTypeChoiceShape {
        before_tokens,
        after_tokens,
    })
}

pub(crate) fn parse_tap_type_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TapTypeChoiceShape<'_>> {
    primitives::parse_all(tokens, parse_tap_type_choice_lexed, "tap-type-choice").ok()
}

fn inline_creature_type_choice_qualifier<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["of", "the", "creature", "type", "of", "your", "choice"]).parse_next(input)
}

fn parse_inline_creature_type_choice_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TapTypeChoiceShape<'a>> {
    let before_tokens = repeat_till(0.., any.void(), peek(inline_creature_type_choice_qualifier))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    inline_creature_type_choice_qualifier.parse_next(input)?;
    let after_tokens = repeat::<_, _, (), _, _>(0.., any.void())
        .take()
        .parse_next(input)?;
    eof.void().parse_next(input)?;
    Ok(TapTypeChoiceShape {
        before_tokens,
        after_tokens,
    })
}

/// Captures an inline creature-type selection while keeping it distinct from
/// back-references such as "of the chosen type" and "of that type". Callers
/// use this only when they must emit a new executable choice.
pub(crate) fn parse_inline_creature_type_choice_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TapTypeChoiceShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_inline_creature_type_choice_lexed,
        "inline creature-type choice",
    )
    .ok()
}

fn chosen_type_marker<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["chosen", "type"]),
        primitives::phrase(&["that", "type"]),
    ))
    .parse_next(input)
}

fn parse_chosen_type_marker_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), chosen_type_marker)
        .void()
        .parse_next(input)
}

pub(crate) fn tap_tokens_mention_chosen_type(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, parse_chosen_type_marker_lexed).is_some()
}
