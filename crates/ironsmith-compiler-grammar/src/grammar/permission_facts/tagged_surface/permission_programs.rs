use super::*;

pub(super) fn parse_for_as_long_as_play_cast_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::phrase(&["for", "as", "long", "as"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::comma()))
        .parse_next(input)?;
    primitives::comma().parse_next(input)?;
    parse_permission_lead_lexed.parse_next(input)?;
    sentence_body_tokens(input)?;
    primitives::sentence_end().parse_next(input)
}
