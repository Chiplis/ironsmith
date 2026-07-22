use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    ADDITIONAL_COST_OBJECT_TAG, CardDefinitionBuilder, CardTextError, EffectAst, IT_TAG, LineAst,
    ParseAnnotations, ParsedAbility, ParsedCardItem, ParsedLevelAbilityAst,
    ParsedLevelAbilityItemAst, ParsedModalAst, ParsedRestrictions, PlayerAst, PredicateAst,
    ReferenceImports, SubjectVerbActionAst, TagKey, TriggerSpec,
};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

mod damage_and_cost_rewrites;
mod line_lowering;
mod modal_and_level_lowering;
mod normalization_support;
#[cfg(test)]
mod rewrite_sentence_grouping;
mod rewrite_support;
mod rewrite_text_helpers;

#[cfg(test)]
use super::semantic_line_parsing::{
    normalize_exert_followup_source_reference_tokens, parse_keyword_line_for_test,
    parse_single_effect_lexed, parse_triggered_line, strip_lexed_suffix_phrase,
};
pub(crate) use normalization_support::normalize_rewrite_line_ast_standalone;
pub(crate) use normalization_support::prepare_parsed_card_ast_for_lowering;
#[cfg(test)]
pub(crate) use normalization_support::rewrite_document_to_normalized_card_ast;

pub(crate) use damage_and_cost_rewrites::*;
pub(crate) use modal_and_level_lowering::*;
pub(crate) use rewrite_support::infer_triggered_ability_functional_zones_from_facts;
use rewrite_support::{
    rewrite_finalize_lowered_card, rewrite_normalize_selected_sacrifice_tags,
    runtime_effects_to_costs,
};
pub(crate) use rewrite_text_helpers::*;

use super::compile_support::{
    collect_tag_spans_from_effects_with_context, compile_condition_from_predicate_ast_with_env,
    effect_references_tag, materialize_prepared_effects_with_trigger_context,
    trigger_binds_player_reference_context as rewrite_trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    LoweredCardDocument, NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst,
    NormalizedCardItem, NormalizedCleaveBranch, NormalizedLineAst, NormalizedLineChunk,
    NormalizedModalAst, NormalizedModalModeAst, NormalizedOverloadBranch, NormalizedParsedAbility,
    NormalizedPreparedAbility, ParsedCardAst,
};
use super::lowering_support::{
    rewrite_apply_delayed_trigger_followup_statement_to_last_ability,
    rewrite_apply_instead_followup_statement_to_last_ability,
    rewrite_lower_keyword_action_to_object_abilities, rewrite_lower_prepared_ability,
    rewrite_lower_prepared_additional_cost_choice_modes_with_exports,
    rewrite_lower_prepared_statement_effects, rewrite_lower_static_abilities_ast,
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability,
    rewrite_prepare_additional_cost_effects_for_lowering, rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
    rewrite_prepare_triggered_effects_for_lowering, rewrite_static_ability_for_keyword_action,
    rewrite_validate_iterated_player_bindings_in_lowered_effects,
};
use super::reference_model::LoweredEffects;
use super::reference_model::ReferenceExports;
use super::restriction_support::{apply_pending_restrictions_to_ability, is_restrictable_ability};
