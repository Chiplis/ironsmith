use super::*;

pub(super) fn parse_redirect_next_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<RedirectNextDamageShape<'a>> {
    alt((
        parse_scoped_all_damage,
        parse_all_to_you_and_permanents,
        parse_all_by_source,
        parse_all_to_target_by_choice,
        parse_next_time,
        parse_next_amount,
        parse_next_amount_by_chosen_source,
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


fn parse_scoped_all_damage<'a>(input: &mut LexStream<'a>) -> WResult<RedirectNextDamageShape<'a>> {
    let mode = opt((
        primitives::kw("until"),
        alt((primitives::phrase(&["end", "of", "turn"]).value(ironsmith_core::ReplacementApplyMode::UntilEndOfTurn),
             primitives::phrase(&["your", "next", "turn"]).value(ironsmith_core::ReplacementApplyMode::UntilYourNextTurn))),
        primitives::comma(),
    ).map(|(_, mode, _)| mode)).parse_next(input)?;
    primitives::kw("all").parse_next(input)?;
    let combat_only = opt(primitives::kw("combat")).parse_next(input)?.is_some();
    primitives::phrase(&["damage", "that", "would", "be", "dealt", "to"]).parse_next(input)?;
    let left = one_or_more_tokens_before(input, primitives::phrase(&["is", "dealt", "to"]))?;
    primitives::phrase(&["is", "dealt", "to"]).parse_next(input)?;
    let destination = one_or_more_tokens_before(input, primitives::kw("instead").void())?.to_vec();
    primitives::kw("instead").parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let bad = || winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new());
    let times: Vec<_> = left.windows(2).enumerate().filter_map(|(index, tokens)|
        (tokens[0].is_word("this") && tokens[1].is_word("turn")).then_some(index)).collect();
    let (mode, left) = match (mode, times.as_slice()) {
        (Some(mode), []) => (mode, left.to_vec()),
        (None, [index]) => {
            let mut complete = left[..*index].to_vec(); complete.extend_from_slice(&left[*index + 2..]);
            (ironsmith_core::ReplacementApplyMode::UntilEndOfTurn, complete)
        }
        _ => return Err(bad()),
    };
    if left.iter().any(|token| token.is_word("turn") || token.is_word("until") || token.is_word("may")) { return Err(bad()); }
    let (recipient, source) = match left.iter().position(|token| token.is_word("by")) {
        Some(index) => (left[..index].to_vec(), Some(left[index + 1..].to_vec())),
        None => (left, None),
    };
    if recipient.is_empty() || source.as_ref().is_some_and(|source| source.is_empty() || has_source_of_your_choice(source)) { return Err(bad()); }
    Ok(RedirectNextDamageShape::ScopedAll(ScopedAllDamageRedirectionShape { recipient, source, destination, combat_only, mode }))
}
