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

pub(crate) use chosen_options::{condition_for_chosen_option, wrap_chosen_option_static_chunk};
use effect_programs::*;
use static_chunks::*;
pub(crate) use triggered_chunks::{
    apply_chosen_option_to_triggered_chunk, apply_explicit_intervening_if_to_triggered_chunk,
    infer_triggered_ability_functional_zones_from_facts,
};

pub(crate) use activated::parse_activated_line;
pub(crate) use lines::{
    dynamic_zone_change_group_token_creation_from_authored_trigger,
    exact_graveyard_card_copy_cast_sequence, exact_looked_hand_optional_cast_bundle,
    exact_target_same_name_graveyard_may_cast_bundle,
    has_created_token_reciprocal_lifecycle_surface,
    has_linked_created_token_next_turn_sacrifice_surface,
    is_authored_dynamic_exile_permission_bundle, is_authored_look_hand_optional_cast_bundle,
    is_exact_correlated_trigger_effect_bundle, parse_exert_attack_keyword_line,
    parse_gift_keyword_line, parse_keyword_special_cases,
    parse_library_origin_source_pump_unblockable_triggered_line,
    parse_statement_token_groups_to_chunks, parse_static_line, rewrite_modal_to_parsed_item,
};
#[cfg(test)]
pub(crate) use lines::{
    normalize_exert_followup_source_reference_tokens, parse_keyword_line_for_test,
    parse_keyword_line_with_full_tokens_for_test, parse_single_effect_lexed, parse_triggered_line,
    strip_lexed_suffix_phrase,
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
    parse_spell_additional_life_cost_per_target_line,
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
use super::restriction_support::apply_pending_mana_restrictions;
use super::token_primitives::strip_leading_if_you_do_lexed;
use super::util::{join_sentences_with_period, parse_level_up_line_lexed};
use crate::effect_sentences::merge_filters;
use crate::model::reference_state::ReferenceEnv;

fn first_for_each_object_filter(effects: &[EffectAst]) -> Option<ObjectFilter> {
    for effect in effects {
        if let EffectAst::ForEachObject { filter, .. } = effect {
            return Some(filter.clone());
        }
        let mut found = None;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = first_for_each_object_filter(nested);
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

fn mark_matching_for_each_object_leading_then(
    effects: &mut [EffectAst],
    expected: &ObjectFilter,
) -> bool {
    for effect in effects {
        if let EffectAst::ForEachObject { filter, .. } = effect
            && filter == expected
            && !filter.has_for_each_leading_then_surface()
        {
            filter.set_for_each_leading_then_surface(true);
            return true;
        }
        let mut marked = false;
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            if !marked {
                marked = mark_matching_for_each_object_leading_then(nested, expected);
            }
        });
        if marked {
            return true;
        }
    }
    false
}

/// A cross-sentence semantic rewrite can make prefix equality fail even when
/// a later sentence's distributive subject survives unchanged. In that flat
/// fallback, retain the explicit authored `Then for each ...` connective on
/// the matching typed filter rather than losing it with the sentence wrapper.
fn preserve_flat_leading_then_for_each_surface(
    sentences: &[Vec<OwnedLexToken>],
    mut effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    for sentence in sentences {
        let words = token_word_refs(sentence);
        if !words.get(..3).is_some_and(|prefix| {
            prefix[0].eq_ignore_ascii_case("then")
                && prefix[1].eq_ignore_ascii_case("for")
                && prefix[2].eq_ignore_ascii_case("each")
        }) {
            continue;
        }
        let sentence_effects = parse_effect_sentences_lexed(sentence).or_else(|_| {
            // Some isolated sentence parsers receive the connective only
            // from the multi-sentence dispatcher. The body after `Then`
            // carries the same typed distributive filter.
            parse_effect_sentences_lexed(&sentence[1..])
        });
        let Ok(sentence_effects) = sentence_effects else {
            continue;
        };
        let Some(filter) = first_for_each_object_filter(&sentence_effects) else {
            continue;
        };
        mark_matching_for_each_object_leading_then(&mut effects, &filter);
    }
    effects
}

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
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    if sentences.len() >= 2
        && effect_grammar::generic_sequence_shapes::parse_starting_each_player_optional_repeat_shape(
            &sentences[0],
            &sentences[1],
        )
        .is_some()
    {
        // The boundary-preserving fallback below strips the participant-order
        // prefix so ordinary per-sentence parsing can proceed. A repeat
        // sequence needs that prefix while its two authored sentences are
        // still adjacent: the second sentence is the first action's loop
        // terminator, not an independent each-player action.
        return parse_effect_sentences_lexed(tokens);
    }
    let mut parse_sentences = sentences.clone();
    let mut stripped_participant_ordering = false;
    if let Some(first) = parse_sentences.first_mut()
        && let Some((_, remainder)) = crate::grammar::primitives::strip_lexed_prefix_phrases(
            first,
            &[&["starting", "with", "you"]],
        )
    {
        *first = trim_lexed_commas(remainder).to_vec();
        stripped_participant_ordering = true;
    }
    let mut parsed_together = if stripped_participant_ordering {
        parse_effect_sentences_lexed(&join_sentences_with_period(&parse_sentences))?
    } else {
        parse_effect_sentences_lexed(tokens)?
    };
    // Cross-sentence self-replacement construction can rebuild a token-copy
    // action after the sentence-local parser has already discarded its
    // quoted exception. The complete authored token stream is still present
    // at this boundary, so reattach the typed inline rule before comparing
    // prefix parses. A changed joint AST intentionally falls back to the flat
    // program below, preserving the enriched replacement as one unit.
    crate::effect_sentences::attach_inline_token_granted_abilities_to_last_create(
        &mut parsed_together,
        tokens,
    );
    if sentences.len() < 2 {
        let Some(sentence) = sentences.first() else {
            return Ok(parsed_together);
        };
        let effects = crate::effect_sentences::preserve_coordinated_effect_chain_surface(
            sentence,
            parsed_together,
        );
        if stripped_participant_ordering {
            return Ok(vec![EffectAst::SourceSentence {
                effects,
                leading_then: false,
                starting_with_controller: true,
            }]);
        }
        return Ok(effects);
    }

    // Some authored follow-up sentences modify the preceding action instead
    // of adding a new top-level effect. Treat that exact typed attachment as
    // part of the preceding boundary group while proving source provenance:
    // parsing the preceding sentence alone must differ from the joint AST by
    // construction (the move has not acquired its entry state or grant yet).
    // Keeping the two sentences in one proof group still preserves an earlier
    // leading `Then` boundary and lets the optional procedure own the whole
    // follow-up at runtime.
    let mut boundary_parse_sentences = Vec::<Vec<OwnedLexToken>>::new();
    let mut boundary_surface_sentences = Vec::<Vec<OwnedLexToken>>::new();
    for (parse_sentence, surface_sentence) in parse_sentences
        .iter()
        .cloned()
        .zip(sentences.iter().cloned())
    {
        if effect_grammar::followup_shapes::parse_moved_object_entry_followup_shape(
            &surface_sentence,
        )
        .is_some()
            && let Some(previous) = boundary_parse_sentences.pop()
        {
            boundary_parse_sentences.push(join_sentences_with_period(&[previous, parse_sentence]));
            continue;
        }
        boundary_parse_sentences.push(parse_sentence);
        boundary_surface_sentences.push(surface_sentence);
    }

    let mut groups = Vec::with_capacity(boundary_parse_sentences.len());
    let mut previous_effect_count = 0usize;
    for prefix_len in 1..=boundary_parse_sentences.len() {
        let prefix_tokens = join_sentences_with_period(&boundary_parse_sentences[..prefix_len]);
        let Ok(parsed_prefix) = parse_effect_sentences_lexed(&prefix_tokens) else {
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        };
        let prefix_effect_count = parsed_prefix.len();
        if prefix_effect_count <= previous_effect_count
            || prefix_effect_count > parsed_together.len()
            || parsed_prefix.as_slice() != &parsed_together[..prefix_effect_count]
        {
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        }

        let sentence_effects = parsed_together[previous_effect_count..prefix_effect_count].to_vec();
        let sentence_effects = crate::effect_sentences::preserve_coordinated_effect_chain_surface(
            &boundary_surface_sentences[prefix_len - 1],
            sentence_effects,
        );
        let leading_then = token_word_refs(&boundary_surface_sentences[prefix_len - 1])
            .first()
            .is_some_and(|word| word.eq_ignore_ascii_case("then"));
        let sentence_words = token_word_refs(&boundary_surface_sentences[prefix_len - 1]);
        let starting_with_controller = sentence_words.get(..3).is_some_and(|words| {
            words[0].eq_ignore_ascii_case("starting")
                && words[1].eq_ignore_ascii_case("with")
                && words[2].eq_ignore_ascii_case("you")
        });
        groups.push(EffectAst::SourceSentence {
            effects: sentence_effects,
            leading_then,
            starting_with_controller,
        });
        previous_effect_count = prefix_effect_count;
    }

    if previous_effect_count == parsed_together.len() {
        Ok(groups)
    } else {
        Ok(preserve_flat_leading_then_for_each_surface(
            &sentences,
            parsed_together,
        ))
    }
}

#[cfg(test)]
mod source_boundary_surface_tests {
    use super::*;

    #[test]
    fn moved_object_followup_keeps_prior_leading_then_boundary() {
        let tokens = lex_line(
            "Draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
            0,
        )
        .expect("linked optional entry procedure should lex");
        let parsed = parse_effect_sentences_preserving_source_boundaries(&tokens)
            .expect("linked optional entry procedure should preserve source boundaries");
        let [
            EffectAst::SourceSentence {
                leading_then: false,
                ..
            },
            EffectAst::SourceSentence {
                effects,
                leading_then: true,
                ..
            },
        ] = parsed.as_slice()
        else {
            panic!("expected draw and linked deployment source groups: {parsed:#?}");
        };
        let [EffectAst::May { effects }] = effects.as_slice() else {
            panic!("entry follow-up must remain inside the optional procedure: {effects:#?}");
        };
        assert!(matches!(
            effects.as_slice(),
            [
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::MoveToZone {
                        battlefield_tapped: true,
                        battlefield_attacking: true,
                        ..
                    },
                    ..
                }),
                EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Tagged(tag, _),
                        duration: Until::EndOfTurn,
                        ..
                    },
                    ..
                }),
            ] if tag.as_str() == IT_TAG
        ));
    }

    #[test]
    fn flat_fallback_keeps_leading_then_on_the_matching_for_each_filter() {
        let tokens = lex_line(
            "Exile up to one target Assassin creature card from your graveyard with a memory counter on it. Then for each creature card you own in exile with a memory counter on it, create a tapped and attacking token that's a copy of it. Exile those tokens at end of combat.",
            0,
        )
        .expect("multi-sentence effect body should lex");
        let sentences = split_lexed_sentences(&tokens)
            .into_iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let parsed = parse_effect_sentences_lexed(&tokens)
            .expect("multi-sentence effect body should parse flat");
        let surfaced = preserve_flat_leading_then_for_each_surface(&sentences, parsed);
        let filter = first_for_each_object_filter(&surfaced)
            .expect("the exiled memory-card iterator should survive the flat parse");

        assert!(filter.has_for_each_leading_then_surface());
        assert_eq!(filter.zone, Some(Zone::Exile));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(
            filter.with_counter,
            Some(crate::filter::CounterConstraint::Typed(
                crate::object::CounterType::Named("memory")
            ))
        );
    }

    #[test]
    fn ordinary_for_each_sentence_does_not_gain_leading_then_surface() {
        let tokens = lex_line(
            "For each creature card you own in exile with a memory counter on it, create a token that's a copy of it.",
            0,
        )
        .expect("ordinary for-each effect should lex");
        let sentences = split_lexed_sentences(&tokens)
            .into_iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let parsed =
            parse_effect_sentences_lexed(&tokens).expect("ordinary for-each effect should parse");
        let surfaced = preserve_flat_leading_then_for_each_surface(&sentences, parsed);
        let filter = first_for_each_object_filter(&surfaced)
            .expect("ordinary for-each iterator should remain typed");

        assert!(!filter.has_for_each_leading_then_surface());
    }
}
