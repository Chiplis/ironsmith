use super::*;

pub(super) fn parse_unattach_chosen_tail_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<UnattachChosenShape<'a>> {
    let count = parse_optional_object_count(input);
    let filter_tokens = repeat_till(1.., any.void(), peek(primitives::kw("from").void()))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    primitives::kw("from").parse_next(input)?;
    let source_tokens = rest.parse_next(input)?;
    if filter_tokens.is_empty() || source_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "unattach cost",
            "object filter and source reference",
        ));
    }
    Ok(UnattachChosenShape {
        count,
        filter_tokens,
        source_tokens,
    })
}

pub(super) fn parse_tap_chosen_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TapChosenShape<'a>> {
    parse_tap_state_chosen_shape_lexed(input, false)
}

pub(super) fn parse_untap_chosen_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TapChosenShape<'a>> {
    parse_tap_state_chosen_shape_lexed(input, true)
}

fn parse_tap_state_chosen_shape_lexed<'a>(
    input: &mut LexStream<'a>,
    untap: bool,
) -> WResult<TapChosenShape<'a>> {
    primitives::kw(if untap { "untap" } else { "tap" }).parse_next(input)?;
    let count = if opt(primitives::kw("x")).parse_next(input)?.is_some() {
        ChoiceCount::dynamic_x()
    } else {
        ChoiceCount::exactly(parse_optional_object_count(input) as usize)
    };
    let other = alt((primitives::kw("other"), primitives::kw("another")))
        .parse_next(input)
        .is_ok();
    opt(primitives::kw(if untap { "tapped" } else { "untapped" })).parse_next(input)?;
    let filter_tokens = rest.parse_next(input)?;
    if filter_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "tap chosen cost",
            "complete object filter",
        ));
    }
    Ok(TapChosenShape {
        count,
        other,
        filter_tokens,
    })
}
