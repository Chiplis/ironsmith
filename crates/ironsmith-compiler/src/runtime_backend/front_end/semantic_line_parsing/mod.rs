//! Front-end semantic parsing for complete Oracle lines.
//!
//! These parsers bridge typed CST facts and the semantic [`LineAst`] model.
//! They intentionally live on the front-end side of the CST -> semantic AST
//! boundary; preparation and runtime lowering consume their typed output.

use crate::Until;
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, PresentationLabel};
#[cfg(test)]
use crate::cards::builders::NormalizedLine;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LineAst, LineInfo,
    OptionalCost, ParsedAbility, ParsedCardItem, ParsedModalAst, ParsedModalModeAst,
    ParsedRestrictions, PlayerAst, PredicateAst, ReferenceImports, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
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

pub(crate) use activated::parse_activated_line;
#[cfg(test)]
pub(crate) use lines::{
    normalize_exert_followup_source_reference_tokens, parse_keyword_line_for_test,
    parse_keyword_line_with_full_tokens_for_test, parse_single_effect_lexed,
    strip_lexed_suffix_phrase,
};
pub(crate) use lines::{
    parse_exert_attack_keyword_line, parse_gift_keyword_line, parse_keyword_special_cases,
    parse_statement_token_groups_to_chunks, parse_static_line, parse_triggered_line,
    rewrite_modal_to_parsed_item,
};

use super::activation_and_restrictions::{
    is_any_player_may_activate_sentence_lexed, parse_activation_cost,
};
use super::clause_support::{
    parse_ability_line_lexed, parse_effect_sentences_lexed,
    parse_linked_attack_group_combat_triggered_line_lexed, parse_static_ability_ast_line_lexed,
    parse_trigger_clause_lexed, parse_triggered_line_lexed,
};
use super::compile_support::{
    compile_condition_from_predicate_ast_with_env,
    materialize_prepared_effects_with_trigger_context,
};
use super::grammar::activated_lines as activated_line_grammar;
use super::grammar::effects as effect_grammar;
use super::ir::{
    ChosenOptionContext, RewriteKeywordLine, RewriteKeywordLineKind, RewriteModalBlock,
    RewriteStatementLine, RewriteStaticLine, RewriteTriggeredLine,
};
use super::keyword_static::{
    parse_if_this_spell_costs_less_to_cast_line_lexed,
    parse_spell_and_player_activated_ability_cost_modifier_line,
    parse_spell_cost_increase_per_target_beyond_first_line, parse_spells_cost_modifier_line,
    parse_value_binding_clause,
};
use super::lexer::{
    OwnedLexToken, TokenKind, render_token_slice, split_lexed_sentences, token_word_refs,
    trim_lexed_commas,
};
#[cfg(test)]
use super::lexer::{TokenWordView, lex_line};
use super::lowering_support::{
    rewrite_parsed_triggered_ability, rewrite_prepare_effects_with_trigger_context_for_lowering,
};
use super::modal_support::{parse_modal_header, replace_modal_header_x_in_effects_ast};
use super::parser_support::split_tokens_for_parse;
use super::reference_model::ReferenceEnv;
use super::restriction_support::apply_pending_mana_restrictions;
use super::util::{join_sentences_with_period, parse_level_up_line_lexed};
