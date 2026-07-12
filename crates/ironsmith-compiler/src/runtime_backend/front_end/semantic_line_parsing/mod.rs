//! Front-end semantic parsing for complete Oracle lines.
//!
//! These parsers bridge typed CST facts and the semantic [`LineAst`] model.
//! They intentionally live on the front-end side of the CST -> semantic AST
//! boundary; preparation and runtime lowering consume their typed output.

use crate::Until;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, PresentationLabel};
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

mod activated;
mod chosen_options;
mod effect_programs;
mod lines;
mod static_chunks;
mod triggered_chunks;

pub(crate) use chosen_options::condition_for_chosen_option;
use chosen_options::wrap_chosen_option_static_chunk;
use effect_programs::*;
use static_chunks::*;
pub(crate) use triggered_chunks::{
    apply_chosen_option_to_triggered_chunk, apply_explicit_intervening_if_to_triggered_chunk,
    infer_triggered_ability_functional_zones_from_facts,
};

pub(crate) use activated::{
    ParsedActivatedLine, align_rewrite_activated_parse_sentences, parse_activated_line,
};
#[cfg(test)]
pub(crate) use lines::{
    normalize_exert_followup_source_reference_tokens, parse_keyword_line_for_test,
    parse_keyword_line_with_full_tokens_for_test, parse_single_effect_lexed,
    strip_lexed_suffix_phrase,
};
pub(crate) use lines::{
    parse_exert_attack_keyword_line, parse_gift_keyword_line, parse_keyword_special_cases,
    parse_special_triggered_line, parse_statement_token_groups_to_chunks, parse_static_line,
    parse_triggered_line, rewrite_modal_to_parsed_item, try_parse_optional_behold_additional_cost,
    try_parse_optional_cost_with_cast_trigger,
};

use super::activation_and_restrictions::{
    is_any_player_may_activate_sentence_lexed, parse_activation_cost,
    parse_mana_spend_bonus_sentence_lexed, parse_mana_usage_restriction_sentence_lexed,
};
use super::activation_and_restrictions::{
    parse_channel_line_lexed, parse_cycling_line_lexed, parse_equip_line_lexed,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
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
use super::grammar::activated_lines as activated_line_grammar;
use super::grammar::effects as effect_grammar;
use super::grammar::filters::parse_spell_filter_with_grammar_entrypoint_lexed;
use super::ir::{
    ChosenOptionContext, RewriteActivatedLine, RewriteKeywordLine, RewriteKeywordLineKind,
    RewriteLevelHeader, RewriteModalBlock, RewriteSagaChapterLine, RewriteSemanticDocument,
    RewriteSemanticItem, RewriteStatementLine, RewriteStaticLine, RewriteTriggeredLine,
};
use super::keyword_static::{
    parse_if_this_spell_costs_less_to_cast_line_lexed,
    parse_spell_and_player_activated_ability_cost_modifier_line,
    parse_spell_cost_increase_per_target_beyond_first_line, parse_spells_cost_modifier_line,
    parse_value_binding_clause,
};
use super::lexer::{
    OwnedLexToken, TokenKind, TokenWordView, lex_line, render_token_slice, split_lexed_sentences,
    token_word_refs, trim_lexed_commas,
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
use super::parser_support::{split_text_for_parse, split_tokens_for_parse};
use super::reference_model::{LoweredEffects, ReferenceEnv, ReferenceExports};
use super::restriction_support::{
    apply_pending_mana_restriction, apply_pending_mana_restrictions,
    apply_pending_restrictions_to_ability, is_restrictable_ability,
};
use super::util::{
    find_first_sacrifice_cost_choice_tag, find_last_exile_cost_choice_tag,
    join_sentences_with_period, parse_additional_cost_choice_options_lexed,
    parse_bargain_line_lexed, parse_bestow_line_lexed, parse_buyback_line_lexed,
    parse_cast_this_spell_only_line_lexed, parse_entwine_line_lexed, parse_escape_line_lexed,
    parse_flashback_line_lexed, parse_harmonize_line_lexed,
    parse_if_conditional_alternative_cost_line_lexed, parse_kicker_line_lexed,
    parse_level_up_line_lexed, parse_madness_line_lexed, parse_mana_symbol,
    parse_morph_keyword_line_lexed, parse_multikicker_line_lexed, parse_number_or_x_value_lexed,
    parse_offspring_line_lexed, parse_reinforce_line_lexed, parse_scryfall_mana_cost,
    parse_self_free_cast_alternative_cost_line_lexed, parse_squad_line_lexed,
    parse_transmute_line_lexed, parse_warp_line_lexed,
    parse_you_may_rather_than_spell_cost_line_lexed, trim_commas, words,
};
