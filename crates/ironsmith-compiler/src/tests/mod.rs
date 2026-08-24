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

use crate::TokenWordView;
use crate::activation_and_restrictions::{
    parse_activate_only_timing_lexed, parse_activation_condition_lexed, parse_cost_reduction_line,
    parse_mana_usage_restriction_sentence_lexed, parse_triggered_times_each_turn_lexed,
};
use crate::compiler_pipeline::parse_text_with_annotations_lowered;
use crate::cst_lowering::recognize_activation_cost_cst as lower_activation_cost_cst;
use crate::effect_sentences::{
    parse_cant_effect_sentence_lexed, parse_effect_sentence_lexed, parse_restriction_duration_lexed,
};
use crate::grammar::activation_costs::{
    parse_activation_cost_rewrite, parse_activation_cost_tokens_rewrite,
};
use crate::grammar::values::{
    parse_mana_cost_rewrite, parse_mana_symbol_group_rewrite, parse_type_line_rewrite,
};
use crate::ir::{RewriteKeywordLineKind, RewriteSemanticDocument, RewriteSemanticItem};
use crate::lexer::{
    LexCursor, lex_line, render_token_slice, split_lexed_sentences, token_word_refs,
};
use crate::parse_context::ParseContext;
use crate::util::parse_value_expr_words;

fn parse_text_to_semantic_document(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(RewriteSemanticDocument, crate::cards::ParseAnnotations), CardTextError> {
    let mut context = crate::parse_context_for_builder(&builder, &text, allow_unsupported);
    crate::compiler_pipeline::parse_text_to_semantic_document_with_context(
        &mut context,
        builder,
        text,
    )
}

fn find_nested_effect<T: 'static>(effect: &crate::effect::Effect) -> Option<&T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found);
    }
    let mut found: Option<*const T> = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested_effect::<T>(child).map(|value| value as *const T);
        }
    });
    // The child effects are owned by `effect` for the duration of this call,
    // so the pointer remains valid for the returned borrow.
    unsafe { found.map(|pointer| &*pointer) }
}

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
                let crate::model::CompilerAbilityKindCore::Triggered(triggered) = parsed.kind()
                else {
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

fn rewrite_line_info(text: &str) -> crate::model::facts::LineInfo {
    crate::model::facts::LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: text.to_string(),
        source_tokens: crate::lexer::lex_line(text, 0).unwrap_or_default(),
        normalized: crate::model::facts::NormalizedLine {
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
    crate::modal_support::parse_modal_header(&info, &tokens)
}

fn parse_error_message<T>(result: Result<T, CardTextError>) -> String {
    match result {
        Ok(_) => panic!("expected parse error"),
        Err(CardTextError::ParseError(message)) => message,
        Err(other) => panic!("expected parse error, got {other:?}"),
    }
}

mod conditional_target_animation;
mod draw_step_search_player_provenance;
mod each_of_any_number;
mod each_player_unless_pays;
mod leading_then_discard_provenance;
mod self_replacement_damage;
mod shard_00;
mod shard_01;
mod shard_02;
mod shard_03;
mod shard_04;
mod shard_05;
mod shard_06;
mod shard_07;
mod shard_08;
mod target_aggregate_mana_value;
