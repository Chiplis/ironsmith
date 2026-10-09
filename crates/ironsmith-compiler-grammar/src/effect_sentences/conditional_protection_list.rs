//! "Until end of turn, creatures you control gain protection from white if
//! you control a Plains, from blue if you control an Island, ... and from
//! green if you control a Forest." (Dominaria's Judgment): one grant whose
//! protection qualities are each gated by their own condition, all checked as
//! the spell resolves (CR 608.2c). Each quality becomes its own conditional
//! grant over the shared subject and duration.
use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst, OwnedLexToken};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    // "... gain(s) protection" — the shared head.
    let Some(protection) = (1..tokens.len()).find(|&index| {
        tokens[index].is_word("protection")
            && tokens[index - 1].is_any_word(&["gain", "gains", "has", "have"])
    }) else {
        return Ok(None);
    };
    let head = &tokens[..=protection];
    // The rest: "from <quality> if <condition>" items separated by commas
    // and/or "and".
    let rest = &tokens[protection + 1..];
    let mut items: Vec<&[OwnedLexToken]> = Vec::new();
    let mut start = 0;
    for index in 0..=rest.len() {
        let boundary = index == rest.len()
            || (rest[index].is_word("from") && index > start)
            ;
        if boundary {
            let mut item = &rest[start..index];
            while item.last().is_some_and(|token| token.is_comma() || token.is_word("and")) {
                item = &item[..item.len() - 1];
            }
            if !item.is_empty() {
                items.push(item);
            }
            start = index;
        }
    }
    if items.len() < 2 {
        return Ok(None);
    }
    let mut effects = Vec::with_capacity(items.len());
    for item in items {
        if !item.first().is_some_and(|token| token.is_word("from")) {
            return Ok(None);
        }
        let Some(if_index) = item.iter().position(|token| token.is_word("if")) else {
            return Ok(None);
        };
        let condition = &item[if_index + 1..];
        if condition.is_empty() || if_index < 2 {
            return Ok(None);
        }
        let Ok(predicate) = crate::grammar::filters::parse_condition_predicate_lexed(condition)
        else {
            return Ok(None);
        };
        let mut grant = head.to_vec();
        grant.extend_from_slice(&item[..if_index]);
        let granted = super::parse_effect_chain_lexed(&grant)?;
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true: granted,
            if_false: Vec::new(),
        }));
    }
    Ok(Some(effects))
}
