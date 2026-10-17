//! "You may put a creature card from your hand onto the battlefield. If its
//! mana cost contains {X}, it enters with a number of +1/+1 counters on it
//! equal to the number of lands you control." (The First Tyrannic War): the
//! second sentence modifies how the put object enters (CR 614.1c), so the
//! conditional entry counters are a replacement registered for this
//! resolution before the put, restricted to an entering object that matches
//! the condition.
use super::*;
use crate::lexer::{OwnedLexToken, TokenKind};
use crate::util::trim_edge_punctuation_tokens;

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_words =
        crate::lexer::parser_token_word_refs(trim_edge_punctuation_tokens(first.lowered()));
    if !first_words.contains(&"put") || !first_words.ends_with(&["onto", "the", "battlefield"]) {
        return Ok(None);
    }
    let second_tokens = trim_edge_punctuation_tokens(second.lowered());
    if !second_tokens.first().is_some_and(|token| token.is_word("if")) {
        return Ok(None);
    }
    let Some(comma) = second_tokens.iter().position(|token| token.kind == TokenKind::Comma)
    else {
        return Ok(None);
    };
    let condition_tokens: &[OwnedLexToken] = &second_tokens[1..comma];
    let entry_tokens: &[OwnedLexToken] = &second_tokens[comma + 1..];
    if !crate::lexer::parser_token_word_refs(entry_tokens).starts_with(&["it", "enters", "with"]) {
        return Ok(None);
    }
    let Some(PredicateAst::ItMatches(filter)) = crate::grammar::primitives::probe_shape(
        crate::grammar::filters::parse_condition_predicate_lexed(condition_tokens),
    ) else {
        return Ok(None);
    };
    let Some(abilities) = crate::keyword_static::parse_enters_with_counters_line(entry_tokens)?
    else {
        return Ok(None);
    };
    let [ability] = abilities.as_slice() else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } =
        ability.payload.clone()
    else {
        return Ok(None);
    };
    let put = crate::effect_sentences::parse_effect_sentence_lexed(first.lowered())?;
    if put.is_empty() {
        return Ok(None);
    }
    let mut effects = vec![EffectAst::subject_verb_register_enter_with_counters_replacement(
        filter,
        counter,
        count,
        crate::effects::ReplacementApplyMode::Resolution,
    )];
    effects.extend(put);
    Ok(Some(effects))
}
