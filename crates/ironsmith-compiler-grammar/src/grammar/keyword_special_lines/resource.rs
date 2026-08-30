use super::*;

pub(super) fn parse_optional_keyword_additional_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<OptionalKeywordAdditionalCostShape<'a>> {
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
    primitives::phrase(&["you", "may"]).parse_next(input)?;
    let cost_tokens = (
        alt((
            primitives::kw("behold").value(OptionalKeywordCostKind::Behold),
            primitives::kw("blight").value(OptionalKeywordCostKind::Blight),
        )),
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::sentence_end()))
            .void(),
    )
        .take()
        .parse_next(input)?;
    let kind = if cost_tokens
        .first()
        .is_some_and(|token| token.is_word("behold"))
    {
        OptionalKeywordCostKind::Behold
    } else {
        OptionalKeywordCostKind::Blight
    };
    primitives::sentence_end().parse_next(input)?;
    Ok(OptionalKeywordAdditionalCostShape {
        kind,
        cost_tokens,
        behold_subtype: None,
    })
}

pub(super) fn parse_behold_and_exile_additional_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<&'a [OwnedLexToken]> {
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
    let behold_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(primitives::phrase(&["and", "exile", "it"])),
    )
    .void()
    .take()
    .parse_next(input)?;
    primitives::phrase(&["and", "exile", "it"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(behold_tokens)
}
