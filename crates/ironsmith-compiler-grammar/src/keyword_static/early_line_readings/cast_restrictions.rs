//! The "you can't cast this [spell] ..." cast restrictions an early static
//! line reads: a condition the cast requires or forbids, and the
//! first-turns timing prohibition.

use super::*;

/// "You can't cast this spell during your first, second, or third turns of
/// the game." (Serra Avenger): a cast prohibition that applies only while the
/// caster is the active player in one of their first three turns.
/// "You can't cast this spell unless there are seven or more cards in your
/// graveyard." (Proft): castable only while the condition holds (CR 601.3).
pub(super) fn read_cant_cast_this_unless(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = input.tokens;
    let words = crate::lexer::token_word_refs(tokens);
    let Some(rest) = words
        .strip_prefix(&["you", "can't", "cast", "this"][..])
        .or_else(|| words.strip_prefix(&["you", "cant", "cast", "this"][..]))
    else {
        return Ok(None);
    };
    let skipped = match rest.first() {
        Some(&("creature" | "spell" | "card" | "planeswalker")) => 5,
        _ => 4,
    };
    // "unless <condition>" requires the condition; "if <condition>" (Rock
    // Jockey: "You can't cast this if you've played a land this turn")
    // forbids casting while it holds.
    let negated = match words.get(skipped) {
        Some(&"unless") => false,
        Some(&"if") => true,
        _ => return Ok(None),
    };
    let keyword = if negated { "if" } else { "unless" };
    let Some(unless_idx) = tokens.iter().position(|token| token.is_word(keyword)) else {
        return Ok(None);
    };
    let Some(condition) = super::static_condition_clause_bound(&tokens[unless_idx + 1..]) else {
        return Ok(None);
    };
    // "You can't cast this [spell] unless/if": the restriction frame read here.
    crate::word_claims::claim_tokens(&tokens[..=unless_idx]);
    let condition = if negated {
        crate::ConditionExpr::Not(Box::new(condition))
    } else {
        condition
    };
    let mut kind = crate::static_abilities::ThisSpellCastRestrictionKind::only_if(condition);
    // Only the "unless" form keeps the prohibition surface; the negated
    // "if" form (Rock Jockey) reads through its `Not` condition.
    kind.cant_cast_unless_surface = !negated;
    kind.cant_cast_if_surface = negated;
    Ok(Some(vec![
        StaticAbility::this_spell_cast_restriction(
            kind,
            crate::lexer::render_token_slice(tokens).trim().to_string(),
        )
        .into(),
    ]))
}
pub(super) fn read_cant_cast_this_during_first_turns(
    input: &EarlyLine<'_>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = crate::lexer::token_word_refs(input.tokens);
    let Some(rest) = words
        .strip_prefix(&["you", "can't", "cast", "this"][..])
        .or_else(|| words.strip_prefix(&["you", "cant", "cast", "this"][..]))
    else {
        return Ok(None);
    };
    let rest = match rest.first() {
        Some(&("creature" | "spell" | "card" | "planeswalker")) => &rest[1..],
        _ => rest,
    };
    if !rest.iter().copied().eq([
        "during", "your", "first", "second", "or", "third", "turns", "of", "the", "game",
    ]) {
        return Ok(None);
    }
    crate::word_claims::claim_words(&words);
    Ok(Some(vec![
        StaticAbility::this_spell_cast_restriction(
            crate::static_abilities::ThisSpellCastRestrictionKind::timing(
                ironsmith_core::ThisSpellCastTiming::NotDuringYourFirstTurns(3),
            ),
            "You can't cast this spell during your first, second, or third turns of the game.",
        )
        .into(),
    ]))
}
