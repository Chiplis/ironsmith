use crate::ability::{Ability, AbilityKind};
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, LineAst, ParseAnnotations, ParsedAbility,
    ParsedCardItem, ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedModalAst,
    ParsedRestrictions, PlayerAst, PredicateAst, ReferenceImports, SubjectVerbActionAst, TagKey,
    TriggerSpec,
};
use crate::model::ParsedCardAst;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

mod damage_and_cost_rewrites;
mod finalization_support;
mod line_ast_helpers;
mod line_lowering;
mod modal_and_level_lowering;
mod normalization_support;
mod restriction_support;
#[cfg(test)]
mod sentence_grouping_lowering;

#[cfg(test)]
use ironsmith_compiler::semantic_line_parsing::{
    normalize_exert_followup_source_reference_tokens, parse_keyword_line_for_test,
    parse_single_effect_lexed, parse_triggered_line, strip_lexed_suffix_phrase,
};
#[cfg(test)]
pub use normalization_support::document_to_normalized_card_ast;
pub use normalization_support::normalize_line_ast_standalone;
pub use normalization_support::normalize_parsed_card_ast_for_lowering;

pub use damage_and_cost_rewrites::*;
pub use finalization_support::derive_triggered_ability_functional_zones_from_facts;
use finalization_support::{
    finalize_lowered_card, normalize_selected_sacrifice_tags, runtime_effects_to_costs,
};
pub use line_ast_helpers::*;
pub use modal_and_level_lowering::*;

use super::compile_support::{
    bind_returned_attachment_history_to_triggering_object,
    compile_condition_from_predicate_ast_with_env, effect_references_tag,
    effects_reference_tag_in_object_position, materialize_prepared_effects_with_trigger_context,
    trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    LoweredCardDocument, NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst,
    NormalizedCardItem, NormalizedCleaveBranch, NormalizedLineAst, NormalizedLineChunk,
    NormalizedModalAst, NormalizedModalModeAst, NormalizedOverloadBranch, NormalizedParsedAbility,
    NormalizedPreparedAbility,
};
use super::lowering_support::{
    apply_delayed_trigger_followup_statement_to_last_ability,
    apply_instead_followup_statement_to_last_ability, assemble_parsed_triggered_ability,
    lower_keyword_action_to_object_abilities, lower_parsed_ability, lower_prepared_ability,
    lower_prepared_additional_cost_choice_modes_with_exports, lower_prepared_statement_effects,
    lower_static_abilities_ast, lower_static_ability_ast,
    runtime_static_ability_for_keyword_action, stage_additional_cost_effects_for_lowering,
    stage_effects_for_lowering, stage_effects_with_trigger_context_for_lowering,
    stage_owned_triggered_effects_for_lowering, stage_statement_effects_for_lowering,
    stage_triggered_effects_for_lowering, validate_iterated_player_bindings_in_lowered_effects,
};
use crate::model::reference_state::LoweredEffects;
use crate::model::reference_state::ReferenceExports;
use restriction_support::{apply_pending_restrictions_to_ability, is_restrictable_ability};
