use crate::ability::AbilityKind;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, ChoiceCount, EffectAst, LineAst, ParsedLineAst,
    ParsedModalHeader, SubjectVerbActionAst, TriggerSpec,
};
use crate::color::ColorSet;
use crate::effect::Value;
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::static_abilities::{StaticAbilityId, StaticAbilityPayload};
use crate::triggers::TriggerKind;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use std::fs;
use std::path::{Path, PathBuf};

use super::TokenWordView;
use super::lexer::{LexCursor, render_token_slice};
use super::{
    RewriteKeywordLineKind, RewriteSemanticItem, lex_line, lower_activation_cost_cst,
    parse_activate_only_timing_lexed, parse_activation_condition_lexed,
    parse_activation_cost_rewrite, parse_activation_cost_tokens_rewrite,
    parse_cant_effect_sentence_lexed, parse_cost_reduction_line, parse_effect_sentence_lexed,
    parse_mana_cost_rewrite, parse_mana_symbol_group_rewrite,
    parse_mana_usage_restriction_sentence_lexed, parse_restriction_duration_lexed,
    parse_text_to_semantic_document, parse_text_with_annotations_lowered,
    parse_triggered_times_each_turn_lexed, parse_type_line_rewrite, split_lexed_sentences,
    token_word_refs,
};
use crate::runtime_backend::util::parse_value_expr_words;

fn rewrite_parsed_line(item: &RewriteSemanticItem) -> Option<&ParsedLineAst> {
    match item {
        RewriteSemanticItem::ParsedLine(line) => Some(line),
        _ => None,
    }
}

fn rewrite_item_is_triggered(item: &RewriteSemanticItem) -> bool {
    rewrite_parsed_line(item).is_some_and(|line| {
        line.chunks.iter().any(|chunk| match chunk {
            LineAst::Triggered { .. } => true,
            LineAst::Ability(parsed) => parsed.trigger_spec.is_some(),
            _ => false,
        })
    })
}

fn rewrite_direct_triggered_chunk(
    item: &RewriteSemanticItem,
) -> Option<(&TriggerSpec, &[EffectAst], Option<u32>)> {
    rewrite_parsed_line(item)?
        .chunks
        .iter()
        .find_map(|chunk| match chunk {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => Some((trigger, effects.as_slice(), *max_triggers_per_turn)),
            LineAst::Ability(parsed) => {
                let AbilityKind::Triggered(triggered) = parsed.kind() else {
                    return None;
                };
                Some((
                    parsed.trigger_spec.as_ref()?,
                    parsed.effects_ast.as_deref()?,
                    triggered
                        .intervening_if
                        .as_ref()
                        .and_then(trigger_frequency_limit),
                ))
            }
            _ => None,
        })
}

fn trigger_frequency_limit(condition: &crate::ConditionExpr) -> Option<u32> {
    match condition {
        crate::ConditionExpr::FirstTimeThisTurn
        | crate::ConditionExpr::SourceFirstCrewedThisTurn => Some(1),
        crate::ConditionExpr::MaxTimesEachTurn(limit)
        | crate::ConditionExpr::DoThisMaxTimesEachTurn(limit) => Some(*limit),
        crate::ConditionExpr::And(left, right) | crate::ConditionExpr::Or(left, right) => {
            trigger_frequency_limit(left).or_else(|| trigger_frequency_limit(right))
        }
        _ => None,
    }
}

fn rewrite_line_info(text: &str) -> super::LineInfo {
    super::LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: text.to_string(),
        source_tokens: super::lexer::lex_line(text, 0).unwrap_or_default(),
        normalized: super::NormalizedLine {
            original: text.to_string(),
            normalized: text.to_string(),
            char_map: Vec::new(),
        },
        semantic_facts: Default::default(),
    }
}

fn parse_modal_header_for_test(text: &str) -> Result<Option<ParsedModalHeader>, CardTextError> {
    let info = rewrite_line_info(text);
    let tokens = lex_line(&info.normalized.normalized, info.line_index)?;
    super::modal_support::parse_modal_header(&info, &tokens)
}

fn parse_error_message<T>(result: Result<T, CardTextError>) -> String {
    match result {
        Ok(_) => panic!("expected parse error"),
        Err(CardTextError::ParseError(message)) => message,
        Err(other) => panic!("expected parse error, got {other:?}"),
    }
}

mod shard_00;
mod shard_01;
mod shard_02;
mod shard_03;
mod shard_04;
mod shard_05;
mod shard_06;
