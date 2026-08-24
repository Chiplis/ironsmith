use super::*;

pub(super) fn classify_next_time_destination(
    tokens: &[OwnedLexToken],
) -> Option<RedirectDamageDestinationShape<'_>> {
    if primitives::parse_all(
        tokens,
        (source_reference, winnow::combinator::eof).void(),
        "redirect source destination",
    )
    .is_ok()
    {
        return Some(RedirectDamageDestinationShape::SourceObject);
    }
    if primitives::parse_all(
        tokens,
        (primitives::kw("you"), winnow::combinator::eof).void(),
        "redirect controller destination",
    )
    .is_ok()
    {
        return Some(RedirectDamageDestinationShape::Controller);
    }
    let is_target = primitives::parse_prefix(tokens, primitives::kw("target")).is_some();
    let mentions_choice = {
        let mut input = LexStream::new(tokens);
        repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), primitives::kw("choice"))
            .parse_next(&mut input)
            .is_ok()
    };
    if is_target && mentions_choice {
        Some(RedirectDamageDestinationShape::TargetOfChoice(tokens))
    } else {
        is_target.then_some(RedirectDamageDestinationShape::Target(tokens))
    }
}

pub(super) fn classify_next_amount_destination(
    tokens: &[OwnedLexToken],
) -> RedirectDamageDestinationShape<'_> {
    if primitives::parse_all(
        tokens,
        (primitives::kw("you"), winnow::combinator::eof).void(),
        "redirect amount controller destination",
    )
    .is_ok()
    {
        RedirectDamageDestinationShape::Controller
    } else {
        RedirectDamageDestinationShape::Target(tokens)
    }
}
