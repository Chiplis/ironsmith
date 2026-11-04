//! A source-local optional price recipe carried by ordinary static grants.
use super::*;
use ironsmith_core::static_ability_model::{GrantedOptionalCostPrice, GrantedOptionalCostTiming};

/// The bounded header binds a recipient-relative generic price. The consequence
/// remains an independently parsed effect program, rather than a custom keyword.
pub fn parse_recipient_optional_spell_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(period) = tokens.iter().position(|token| token.kind == TokenKind::Period) else {
        return Ok(None);
    };
    let header = &tokens[..period];
    let words = TokenWordView::new(header);
    let header_refs = words.to_word_refs();
    let lead = ["as", "an", "additional", "cost", "to", "cast", "this", "spell", "you", "may", "pay"];
    if !header_refs.starts_with(&lead) {
        return Ok(None);
    }
    // The price is either the recipient-relative generic mana value or an
    // ordinary authored cost ("you may pay {2}{R}").
    let price = if header_refs[lead.len()..] == ["mana", "equal", "to", "this", "spell's", "mana", "value"] {
        GrantedOptionalCostPrice::GenericManaValue
    } else {
        let Some(pay) = header.iter().position(|token| token.is_word("pay")) else {
            return Ok(None);
        };
        let price_tokens = &header[pay + 1..];
        if price_tokens.is_empty() || !price_tokens.iter().all(|token| token.kind == TokenKind::ManaGroup || token.kind == TokenKind::Comma) {
            return Ok(None);
        }
        let Ok(cost) = crate::activation_and_restrictions::parse_compiler_activation_cost(price_tokens) else {
            return Ok(None);
        };
        GrantedOptionalCostPrice::Fixed(cost)
    };
    let mut tail = &tokens[period + 1..];
    let mut name = None;
    // An explicit local label survives heterogeneous grants without guessing
    // which of the recipient's independent identity markers owns this price.
    let final_start = tail.iter().rposition(|token| token.kind == TokenKind::Period).map(|i| i + 1).unwrap_or(0);
    let rider = &tail[final_start..];
    let rider_words = TokenWordView::new(rider);
    let rider_refs = rider_words.to_word_refs();
    let name_prefix = ["this", "optional", "additional", "cost", "is", "named"];
    if rider_refs.starts_with(&name_prefix) && rider_refs.len() > name_prefix.len() {
        if rider.iter().any(|token| token.kind != TokenKind::Word && token.kind != TokenKind::Quote) {
            return Ok(None);
        }
        name = Some(rider_refs[name_prefix.len()..].join(" "));
        crate::word_claims::claim_tokens(rider);
        tail = &tail[..final_start];
    }
    let tail_words = TokenWordView::new(tail);
    let prefix = ["when", "you", "cast", "this", "spell", "if", "you", "paid", "this", "additional", "cost"];
    let refs = tail_words.to_word_refs();
    if !refs.starts_with(&prefix) { return Ok(None); }
    let Some(body_range) = tail_words.token_span_for_words(prefix.len(), refs.len()) else {
        return Ok(None);
    };
    let body = crate::lexer::trim_lexed_commas(&tail[body_range]);
    let effects = crate::clause_support::parse_effect_sentences_lexed(body)?;
    if effects.is_empty() { return Ok(None); }
    let mut reference = ironsmith_core::cost_model::OptionalCostRef::new(ironsmith_core::cost_model::OptionalCostKind::Additional);
    reference.custom_keyword_attributable = true;
    reference.custom_keyword = name;
    // Only the validated frame is owned here; consequence leaves own claims.
    crate::word_claims::claim_tokens(header);
    let Some(frame) = tail_words.token_span_for_words(0, prefix.len()) else { return Ok(None); };
    crate::word_claims::claim_tokens(&tail[frame]);
    Ok(Some(StaticAbility::grant_optional_spell_cost(
        ObjectFilter::source(), price,
        reference, effects, GrantedOptionalCostTiming::CastTrigger,
        crate::lexer::render_token_slice(tokens),
    )))
}
