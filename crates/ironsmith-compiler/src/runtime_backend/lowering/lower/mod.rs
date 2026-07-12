use crate::Until;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, PresentationLabel};
use crate::cards::builders::{
    CardDefinition, CardDefinitionBuilder, CardTextError, ChoiceCount, EffectAst, GiftTimingAst,
    IT_TAG, InsteadSemantics, LibraryBottomOrderAst, LineAst, LineInfo, NormalizedLine,
    OptionalCost, ParseAnnotations, ParsedAbility, ParsedCardItem, ParsedLevelAbilityAst,
    ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalAst, ParsedModalModeAst,
    ParsedRestrictions, PlayerAst, PredicateAst, ReferenceImports, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_core::CostComponent;

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
    align_rewrite_activated_parse_sentences, normalize_exert_followup_source_reference_tokens,
    parse_keyword_line_for_test, parse_single_effect_lexed, parse_triggered_line,
    strip_lexed_suffix_phrase,
};
pub(crate) use normalization_support::normalize_rewrite_line_ast_standalone;
pub(crate) use normalization_support::{
    prepare_parsed_card_ast_for_lowering, rewrite_document_to_normalized_card_ast,
};

pub(crate) use damage_and_cost_rewrites::*;
pub(crate) use modal_and_level_lowering::*;
pub(crate) use rewrite_support::infer_triggered_ability_functional_zones_from_facts;
use rewrite_support::{
    rewrite_finalize_lowered_card, rewrite_normalize_additional_cost_sacrifice_tags,
    runtime_effects_to_costs,
};
pub(crate) use rewrite_text_helpers::*;

use super::compile_support::{
    collect_tag_spans_from_effects_with_context, compile_condition_from_predicate_ast_with_env,
    materialize_prepared_effects_with_trigger_context,
    trigger_binds_player_reference_context as rewrite_trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    LoweredCardDocument, NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst,
    NormalizedCardItem, NormalizedLineAst, NormalizedLineChunk, NormalizedModalAst,
    NormalizedModalModeAst, NormalizedOverloadBranch, NormalizedParsedAbility,
    NormalizedPreparedAbility, ParsedCardAst, ParsedOverloadBranch,
};
use super::ir::{
    ChosenOptionContext, RewriteKeywordLine, RewriteKeywordLineKind, RewriteLevelHeader,
    RewriteModalBlock, RewriteSagaChapterLine, RewriteSemanticDocument, RewriteSemanticItem,
    RewriteStatementLine, RewriteStaticLine, RewriteTriggeredLine,
};
use super::lowering_support::{
    rewrite_apply_delayed_trigger_followup_statement_to_last_ability,
    rewrite_apply_instead_followup_statement_to_last_ability, rewrite_lower_prepared_ability,
    rewrite_lower_prepared_additional_cost_choice_modes_with_exports,
    rewrite_lower_prepared_statement_effects, rewrite_lower_static_abilities_ast,
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability,
    rewrite_prepare_additional_cost_effects_for_lowering, rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
    rewrite_prepare_triggered_effects_for_lowering, rewrite_static_ability_for_keyword_action,
    rewrite_validate_iterated_player_bindings_in_lowered_effects,
};
use super::modal_support::{parse_modal_header, replace_modal_header_x_in_effects_ast};
use super::reference_model::LoweredEffects;
use super::reference_model::ReferenceEnv;
use super::reference_model::ReferenceExports;
use super::restriction_support::{apply_pending_restrictions_to_ability, is_restrictable_ability};
use super::util::{find_first_sacrifice_cost_choice_tag, find_last_exile_cost_choice_tag};
