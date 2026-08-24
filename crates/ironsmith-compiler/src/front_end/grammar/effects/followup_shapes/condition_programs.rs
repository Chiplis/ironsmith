use super::*;

pub(super) fn conditional_followup_prefix<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalFollowupKind> {
    alt((
        primitives::phrase(&[
            "when", "one", "or", "more", "cards", "are", "milled", "this", "way",
        ])
        .value(ConditionalFollowupKind::WhenMilledThisWay),
        primitives::phrase(&["if", "no", "one", "does"])
            .value(ConditionalFollowupKind::IfNoOneDoes),
        (
            primitives::phrase(&["if", "you", "win"]),
            alt((
                primitives::phrase(&["the", "clash"]),
                primitives::phrase(&["that", "clash"]),
            )),
            peek(primitives::comma()),
        )
            .value(ConditionalFollowupKind::IfYouWinClash),
        (
            primitives::phrase(&["if", "you", "win"]),
            primitives::phrase(&["the", "flip"]),
            peek(primitives::comma()),
        )
            .value(ConditionalFollowupKind::IfYouWinFlip),
        (
            primitives::phrase(&["if", "you", "win"]),
            peek(primitives::comma()),
        )
            .value(ConditionalFollowupKind::IfYouWin),
    ))
    .parse_next(input)
}

pub(super) fn parse_conditional_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ConditionalFollowupShape<'a>> {
    let kind = conditional_followup_prefix.parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::comma()))
        .void()
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let continuation_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ConditionalFollowupShape {
        kind,
        continuation_tokens,
    })
}

pub fn parse_conditional_followup(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalFollowupShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_conditional_followup_lexed,
        "conditional subject-verb followup",
    )
    .ok()
}
