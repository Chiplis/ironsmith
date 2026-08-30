use super::*;

pub(super) fn parse_revealed_top_library_permission_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RevealedTopLibraryPermissionFact<'a>> {
    primitives::phrase(&["until", "end", "of", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    primitives::any_phrase(&[
        &["that", "card"],
        &["that", "revealed", "card"],
        &["the", "revealed", "card"],
    ])
    .parse_next(input)?;
    primitives::phrase(&["remains", "on", "top", "of", "your", "library"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "play", "with", "the", "top", "card", "of", "your", "library", "revealed", "and",
    ])
    .parse_next(input)?;
    let permission_tokens = sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(RevealedTopLibraryPermissionFact { permission_tokens })
}

pub(super) fn parse_for_as_long_as_look_at_tagged_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ForAsLongAsLookAtTaggedFact<'a>> {
    parse_for_as_long_as_exiled_lexed.parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    let reference = alt((
        primitives::phrase(&["you", "may", "look", "at", "it"]).value(TaggedLookReference::It),
        primitives::phrase(&["you", "may", "look", "at", "that", "card"])
            .value(TaggedLookReference::ThatCard),
        primitives::phrase(&["you", "may", "look", "at", "them"]).value(TaggedLookReference::Them),
        primitives::phrase(&["you", "may", "look", "at", "those", "cards"])
            .value(TaggedLookReference::ThoseCards),
    ))
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    opt(primitives::kw("and")).parse_next(input)?;
    let permission_tokens = sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(ForAsLongAsLookAtTaggedFact {
        lifetime: PermissionLifetimeFact::ForAsLongAsExiled,
        reference,
        permission_tokens,
    })
}
