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
    // One sentence only: a trailing sentence is not under the condition.
    if remainder
        .iter()
        .rev()
        .skip_while(|token| token.is_period())
        .any(|token| token.is_period())
    {
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
            _ => return Ok(None),
        }
    };
    let condition = match condition {
        LeadingCondition::YourTurn => PredicateAst::YourTurn,
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
