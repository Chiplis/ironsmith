use super::super::grammar::primitives as grammar;
use super::super::lexer::OwnedLexToken;
use super::super::util::parse_target_phrase;
use crate::cards::builders::{CardTextError, EffectAst};
use crate::effect::Until;
use crate::object::CounterType;

pub(crate) fn parse_exile_then_meld_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["exile", "them"]).is_none() {
        return Ok(None);
    }
    let Some(meld_idx) = crate::cards::builders::parser::grammar::primitives::find_phrase_start(
        tokens,
        &["then", "meld", "them", "into"],
    ) else {
        return Ok(None);
    };
    let result_words = &clause_words[meld_idx + 4..];
    if result_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing meld result name (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(Some(EffectAst::Meld {
        result_name: result_words.join(" "),
        enters_tapped: false,
        enters_attacking: false,
    }))
}

pub(crate) fn parse_if_damage_would_be_dealt_put_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["if", "damage", "would", "be", "dealt", "to"])
        .is_none()
    {
        return Ok(None);
    }

    let Some(this_turn_rel) = crate::cards::builders::parser::grammar::primitives::find_phrase_start(
        &tokens[6..],
        &["this", "turn"],
    ) else {
        return Ok(None);
    };
    let this_turn_idx = 6 + this_turn_rel;
    let tail = &clause_words[this_turn_idx + 2..];
    let valid_tail = matches!(
        tail,
        [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "that", "creature"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "that", "creature"
        ]
    );
    if !valid_tail {
        return Ok(None);
    }

    let target_tokens = &tokens[6..this_turn_idx];
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(target_tokens)?;

    Ok(Some(EffectAst::PreventDamageToTargetPutCounters {
        amount: None,
        target,
        duration: Until::EndOfTurn,
        counter_type: CounterType::PlusOnePlusOne,
    }))
}
