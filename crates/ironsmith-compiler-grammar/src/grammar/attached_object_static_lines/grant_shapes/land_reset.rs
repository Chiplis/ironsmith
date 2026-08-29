use super::*;

fn quoted_ability_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    primitives::quote().parse_next(input)?;
    let body = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::quote()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::quote().parse_next(input)?;
    Ok(trim_lexed_commas(body))
}

fn attached_land_ability_reset_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedLandAbilityResetSpec<'a>> {
    let (subject, subject_tokens) = parse_attached_subject_lexed
        .with_taken()
        .parse_next(input)?;
    if subject != AttachedSubject::EnchantedLand {
        return Err(primitives::backtrack_err(
            "attached land ability reset",
            "enchanted land",
        ));
    }
    semantic_phrase(&[
        "loses",
        "all",
        "land",
        "types",
        "and",
        "abilities",
        "and",
        "has",
    ])
    .parse_next(input)?;
    let first = quoted_ability_body.parse_next(input)?;
    let rest: Vec<&[OwnedLexToken]> = repeat(
        0..,
        (semantic_kw("and"), quoted_ability_body).map(|(_, body)| body),
    )
    .parse_next(input)?;
    semantic_finish(input)?;

    let mut granted_abilities = Vec::with_capacity(rest.len() + 1);
    granted_abilities.push(first);
    granted_abilities.extend(rest);
    Ok(AttachedLandAbilityResetSpec {
        subject_tokens,
        granted_abilities,
    })
}

pub fn parse_attached_land_ability_reset_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedLandAbilityResetSpec<'_>> {
    primitives::parse_all(
        tokens,
        attached_land_ability_reset_lexed,
        "attached land loses types and abilities then gains abilities",
    )
    .ok()
}
