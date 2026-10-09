//! "During your turn, <static ability>." (Personal Sanctuary) and "As long as
//! <condition>, <static ability>." (Multiclass Baldric): a leading timing or
//! state condition over a complete, otherwise supported single-sentence static
//! ability. The condition gates whether the ability functions (CR 604.2 /
//! CR 611.3a), so the wrapped reading is the inner ability made conditional.
//!
//! The wrapper is a last resort: it only claims a line when no other static
//! rule reads the whole line, so specialised condition-aware productions keep
//! ownership of their surfaces.

use super::*;
use std::cell::Cell;

thread_local! {
    static WRAPPING_LEADING_CONDITION: Cell<bool> = const { Cell::new(false) };
}

struct WrappingGuard {
    previous: bool,
}

impl WrappingGuard {
    fn set(value: bool) -> Self {
        let previous = WRAPPING_LEADING_CONDITION.with(|flag| flag.replace(value));
        Self { previous }
    }
}

impl Drop for WrappingGuard {
    fn drop(&mut self) {
        let previous = self.previous;
        WRAPPING_LEADING_CONDITION.with(|flag| flag.set(previous));
    }
}

enum LeadingCondition<'a> {
    YourTurn,
    /// "During each opponent's end step, ..." (Final-Word Phantom).
    EachOpponentsEndStep,
    AsLongAs(&'a [OwnedLexToken]),
}

fn split_leading_condition(
    tokens: &[OwnedLexToken],
) -> Option<(LeadingCondition<'_>, &[OwnedLexToken])> {
    if let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["during", "your", "turn"]),
    ) {
        let remainder = trim_lexed_commas(rest);
        if remainder.len() < rest.len() && !remainder.is_empty() {
            return Some((LeadingCondition::YourTurn, remainder));
        }
        return None;
    }
    if let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::any_phrase(&[
            &["during", "each", "opponent's", "end", "step"],
            &["during", "each", "opponents", "end", "step"],
            &["during", "each", "opponent", "s", "end", "step"],
        ]),
    ) {
        let remainder = trim_lexed_commas(rest);
        if remainder.len() < rest.len() && !remainder.is_empty() {
            return Some((LeadingCondition::EachOpponentsEndStep, remainder));
        }
        return None;
    }
    let prefix = split_as_long_as_condition_prefix_lexed(tokens)?;
    Some((
        LeadingCondition::AsLongAs(prefix.condition_tokens),
        prefix.remainder_tokens,
    ))
}

pub fn parse_leading_condition_wrapped_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if WRAPPING_LEADING_CONDITION.with(Cell::get) {
        return Ok(None);
    }
    let tokens = trim_edge_punctuation(tokens);
    let Some((condition, remainder)) = split_leading_condition(&tokens) else {
        return Ok(None);
    };
    // A pronoun subject in the remainder ("..., it can't be blocked") is read
    // by the inner grammar as this object. That is only the author's meaning
    // when the condition is itself about this object ("As long as this
    // creature is enchanted, it gets ..."); otherwise the pronoun may name an
    // object from the condition (an enchanted creature) and must not be
    // rebound to the source.
    let remainder_has_pronoun_subject = remainder
        .first()
        .is_some_and(|token| token.is_any_word(&["it", "its", "it's", "they", "their"]));
    let condition_is_about_this_object = match &condition {
        LeadingCondition::YourTurn | LeadingCondition::EachOpponentsEndStep => false,
        LeadingCondition::AsLongAs(condition_tokens) => condition_tokens
            .first()
            .is_some_and(|token| token.is_word("this")),
    };
    if remainder_has_pronoun_subject && !condition_is_about_this_object {
        return Ok(None);
    }
    // One sentence only: a trailing sentence is not under the condition.
    let mut quoted = false;
    let mut unquoted_periods = Vec::new();
    for (idx, token) in remainder.iter().enumerate() {
        if token.is_quote() {
            quoted = !quoted;
        } else if !quoted && token.is_period() {
            unquoted_periods.push(idx);
        }
    }
    let last_content = remainder
        .iter()
        .rposition(|token| !token.is_period() && !token.is_quote())
        .unwrap_or(0);
    if unquoted_periods.iter().any(|&idx| idx < last_content) {
        return Ok(None);
    }
    {
        let _guard = WrappingGuard::set(true);
        if !matches!(parse_static_ability_ast_line_lexed(&tokens), Ok(None)) {
            return Ok(None);
        }
    }
    let inner = {
        let _guard = WrappingGuard::set(false);
        match parse_static_ability_ast_line_lexed(remainder) {
            Ok(Some(inner)) if !inner.is_empty() => inner,
            _ => match parse_conjoined_static_sentences(remainder) {
                Some(inner) => inner,
                None => return Ok(None),
            },
        }
    };
    let condition = match condition {
        LeadingCondition::YourTurn => PredicateAst::YourTurn,
        LeadingCondition::EachOpponentsEndStep => {
            PredicateAst::Bound(Box::new(crate::ConditionExpr::OpponentsEndStep))
        }
        LeadingCondition::AsLongAs(condition_tokens) => {
            parse_static_condition_clause(condition_tokens)?
        }
    };
    Ok(Some(
        inner
            .into_iter()
            .map(|ability| StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition: condition.clone(),
            })
            .collect(),
    ))
}

/// "creatures you control have first strike and equip abilities you activate
/// cost {1} less to activate" (Nahiri, Storm of Stone): two complete static
/// sentences joined by "and" under one leading condition. Only claimed when
/// the whole remainder is not itself one static ability, both halves are, and
/// the second half is not a bare keyword list (which would belong to the
/// first half's grant).
fn parse_conjoined_static_sentences(remainder: &[OwnedLexToken]) -> Option<Vec<StaticAbilityAst>> {
    let body = trim_edge_punctuation(remainder);
    let mut found = None;
    for (idx, token) in body.iter().enumerate() {
        if !token.is_word("and") || idx == 0 || idx + 1 >= body.len() {
            continue;
        }
        let (left, right) = (&body[..idx], &body[idx + 1..]);
        if parse_ability_line(right).is_some() {
            continue;
        }
        let Ok(Some(mut first)) = parse_static_ability_ast_line_lexed(left) else {
            continue;
        };
        let Ok(Some(second)) = parse_static_ability_ast_line_lexed(right) else {
            continue;
        };
        if first.is_empty() || second.is_empty() {
            continue;
        }
        if found.is_some() {
            // Two different splits read: ambiguous, claim neither.
            return None;
        }
        first.extend(second);
        found = Some(first);
    }
    found
}
