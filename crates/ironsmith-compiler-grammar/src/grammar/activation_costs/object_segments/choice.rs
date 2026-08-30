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
    primitives::kw("tap").parse_next(input)?;
    let count = parse_optional_object_count(input);
    let other = alt((primitives::kw("other"), primitives::kw("another")))
        .parse_next(input)
        .is_ok();
    primitives::kw("untapped").parse_next(input)?;
    let filter_tokens = rest.parse_next(input)?;
    if filter_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "tap chosen cost",
            "object filter after untapped",
        ));
    }
    Ok(TapChosenShape {
        count,
        other,
        filter_tokens,
    })
}
