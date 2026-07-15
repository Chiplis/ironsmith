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

/// Parse one complete effect body while retaining every source sentence whose
/// boundary is stable under the same joint parse.
///
/// Parsing each sentence in isolation loses the discourse context needed by
/// followups such as "it", "those cards", and "this way". Instead, parse
/// successively longer prefixes and compare them with the corresponding
/// prefix of the whole-body AST. This keeps one shared semantic parse while
/// proving exactly where a later sentence did not rewrite or absorb an
/// earlier effect. Any cross-sentence structural rewrite falls back to the
/// ordinary flat program.
fn parse_effect_sentences_preserving_source_boundaries(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let parsed_together = parse_effect_sentences_lexed(tokens)?;
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    if sentences.len() < 2 {
        let Some(sentence) = sentences.first() else {
            return Ok(parsed_together);
        };
        return Ok(
            crate::runtime_backend::effect_sentences::preserve_coordinated_effect_chain_surface(
                sentence,
                parsed_together,
            ),
        );
    }

    let mut groups = Vec::with_capacity(sentences.len());
    let mut previous_effect_count = 0usize;
    for prefix_len in 1..=sentences.len() {
        let prefix_tokens = join_sentences_with_period(&sentences[..prefix_len]);
        let Ok(parsed_prefix) = parse_effect_sentences_lexed(&prefix_tokens) else {
            return Ok(parsed_together);
        };
        let prefix_effect_count = parsed_prefix.len();
        if prefix_effect_count <= previous_effect_count
            || prefix_effect_count > parsed_together.len()
            || parsed_prefix.as_slice() != &parsed_together[..prefix_effect_count]
        {
            return Ok(parsed_together);
        }

        let sentence_effects = parsed_together[previous_effect_count..prefix_effect_count].to_vec();
        let sentence_effects =
            crate::runtime_backend::effect_sentences::preserve_coordinated_effect_chain_surface(
                &sentences[prefix_len - 1],
                sentence_effects,
            );
        groups.push(EffectAst::SourceSentence {
            effects: sentence_effects,
        });
        previous_effect_count = prefix_effect_count;
    }

    if previous_effect_count == parsed_together.len() {
        Ok(groups)
    } else {
        Ok(parsed_together)
    }
}
