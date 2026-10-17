//! "For each of those creatures, its controller may pay {1} or {2}. If that
//! player doesn't, <A>. If that player pays only {1}, <B>." (Winter's Chill)
//!
//! A tiered optional payment: the payer chooses to pay the greater cost, the
//! lesser cost, or nothing, and each of the three outcomes is distinct. It is
//! the same decision as offering the greater payment first and, when it is
//! declined, offering the lesser one: declining both is "doesn't", paying the
//! second offer is "pays only" the lesser cost, and paying the first offer has
//! no consequence. The statements run once per iterated object, so the result
//! gates live inside the loop body next to the payments they read.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, ForEachEffectAst, IfResultPredicate,
};
use crate::lexer::OwnedLexToken;

pub(super) struct TieredPaymentGroup {
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

fn is_word_any(token: &OwnedLexToken, words: &[&str]) -> bool {
    words.iter().any(|word| token.is_word(word))
}

/// The `[pay, A..., or, B...]` split of a trailing "may pay {A} or {B}".
fn split_tiered_costs(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    let pay_idx = tokens.iter().rposition(|token| token.is_word("pay"))?;
    if pay_idx == 0 || !tokens[pay_idx - 1].is_word("may") {
        return None;
    }
    let tail = &tokens[pay_idx + 1..];
    let or_offset = tail.iter().position(|token| token.is_word("or"))?;
    let lesser = &tail[..or_offset];
    let greater = &tail[or_offset + 1..];
    let is_cost = |cost: &[OwnedLexToken]| {
        !cost.is_empty() && cost.iter().all(|token| token.as_word().is_none())
    };
    if !is_cost(lesser) || !is_cost(greater) {
        return None;
    }
    Some((pay_idx, pay_idx + 1 + or_offset))
}

fn loop_body_mut(effects: &mut [EffectAst]) -> Option<&mut Vec<EffectAst>> {
    match effects {
        [EffectAst::ForEach(ForEachEffectAst::ForEachTagged { effects, .. })]
        | [EffectAst::ForEach(ForEachEffectAst::ForEachObject { effects, .. })] => Some(effects),
        _ => None,
    }
}

fn parse_sentence(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    super::parse_effect_sentence_lexed(tokens).ok()
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<TieredPaymentGroup>, CardTextError> {
    let (Some(offer), Some(declined), Some(lesser_only)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let offer_tokens = trim(offer.lowered());
    if !offer_tokens.first().is_some_and(|token| token.is_word("for")) {
        return Ok(None);
    }
    let Some((pay_idx, or_idx)) = split_tiered_costs(offer_tokens) else {
        return Ok(None);
    };
    let lesser_cost = &offer_tokens[pay_idx + 1..or_idx];
    let greater_cost = &offer_tokens[or_idx + 1..];

    // "If that player doesn't, ..." / "If they don't, ..."
    let declined_tokens = trim(declined.lowered());
    let declined_opens = match declined_tokens {
        [if_word, that, player, doesnt, ..]
            if if_word.is_word("if")
                && that.is_word("that")
                && player.is_word("player")
                && is_word_any(doesnt, &["doesnt", "doesn't"]) =>
        {
            true
        }
        [if_word, they, dont, ..]
            if if_word.is_word("if")
                && they.is_word("they")
                && is_word_any(dont, &["dont", "don't"]) =>
        {
            true
        }
        _ => false,
    };
    if !declined_opens {
        return Ok(None);
    }

    // "If that player pays only {A}, ..." / "If they pay only {A}, ..."
    let lesser_tokens = trim(lesser_only.lowered());
    let head_len = match lesser_tokens {
        [if_word, that, player, pays, only, ..]
            if if_word.is_word("if")
                && that.is_word("that")
                && player.is_word("player")
                && pays.is_word("pays")
                && only.is_word("only") =>
        {
            5
        }
        [if_word, they, pay, only, ..]
            if if_word.is_word("if")
                && they.is_word("they")
                && pay.is_word("pay")
                && only.is_word("only") =>
        {
            4
        }
        _ => return Ok(None),
    };
    let after_head = &lesser_tokens[head_len..];
    if after_head.len() <= lesser_cost.len()
        || crate::lexer::render_token_slice(&after_head[..lesser_cost.len()])
            != crate::lexer::render_token_slice(lesser_cost)
    {
        return Ok(None);
    }
    let mut lesser_effect_tokens = &after_head[lesser_cost.len()..];
    if lesser_effect_tokens
        .first()
        .is_some_and(|token| token.as_word().is_none() && token.slice == ",")
    {
        lesser_effect_tokens = &lesser_effect_tokens[1..];
    }
    if lesser_effect_tokens.is_empty() {
        return Ok(None);
    }

    // Offer the greater payment first, then the lesser one.
    let mut greater_offer = offer_tokens[..pay_idx + 1].to_vec();
    greater_offer.extend_from_slice(greater_cost);
    let mut lesser_offer = offer_tokens[..pay_idx + 1].to_vec();
    lesser_offer.extend_from_slice(lesser_cost);
    let Some(mut outer) = parse_sentence(&greater_offer) else {
        return Ok(None);
    };
    let Some(mut inner) = parse_sentence(&lesser_offer) else {
        return Ok(None);
    };
    let Some(declined_effects) = parse_sentence(declined.lowered()) else {
        return Ok(None);
    };
    if !matches!(
        declined_effects.as_slice(),
        [EffectAst::Conditionals(ConditionalEffectAst::IfResult { .. })]
    ) {
        return Ok(None);
    }
    let Some(lesser_effects) = parse_sentence(lesser_effect_tokens) else {
        return Ok(None);
    };
    let Some(inner_body) = loop_body_mut(&mut inner) else {
        return Ok(None);
    };
    // "If they do, <B>. If they don't, <A>." order: both gates read the
    // lesser offer, as the familiar paired result statements do.
    let mut second_offer = std::mem::take(inner_body);
    second_offer.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: lesser_effects,
    }));
    second_offer.extend(declined_effects);
    let Some(outer_body) = loop_body_mut(&mut outer) else {
        return Ok(None);
    };
    outer_body.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_offer,
    }));
    Ok(Some(TieredPaymentGroup {
        effects: outer,
        first_sentence: sentence_idx,
        consumed: 3,
    }))
}

pub(super) fn continue_with(
    _group: &mut TieredPaymentGroup,
    _sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    Ok(false)
}

pub(super) fn finish(group: TieredPaymentGroup) -> Vec<EffectAst> {
    group.effects
}
