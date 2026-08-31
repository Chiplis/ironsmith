use super::*;

pub(super) fn parse_redirect_next_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    alt((
        parse_all_to_you_and_permanents,
        parse_all_by_source,
        parse_all_to_target_by_choice,
        parse_next_time,
        parse_next_amount,
    ))
    .parse_next(input)
}

pub fn parse_redirect_next_damage_tokens(
    tokens: &[OwnedLexToken],
) -> Option<RedirectNextDamageShape<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_redirect_next_damage_lexed,
        "redirect next damage",
    )
}
