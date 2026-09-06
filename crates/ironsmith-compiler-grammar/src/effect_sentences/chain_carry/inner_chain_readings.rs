//! The readings of one inner effect chain before coordination: the typed
//! whole-chain shapes (the sacrifice-it delayed step, token-copy exceptions,
//! looked-card partitions, graveyard shuffles, gain-then-get compounds, the
//! control-flow plan, conditional animations, consult traversals, library
//! searches, ...). Formerly a first-match ladder in `chain_carry`; every
//! reading runs; two different readings of one input are an ambiguity error. The
//! coordination materializer is the fallback.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct InnerChain<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) recognize_control_flow: bool,
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl InnerChain<'_> {
    /// Whether the reading `id` of this registry reads this input; a reading
    /// ranked below it admits the input only when it does not.
    fn read_by(&self, id: &'static str) -> bool {
        if let Some(read) = self.read_by_cache.borrow().get(id) {
            return *read;
        }
        let read = READINGS
            .iter()
            .find(|reading| reading.id.as_str() == id)
            .is_some_and(|reading| {
                (reading.admits)(self) && matches!((reading.read)(self), ParseOutcome::Match(_))
            });
        self.read_by_cache.borrow_mut().insert(id, read);
        read
    }
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(
        &self,
        read: Result<Option<Vec<EffectAst>>, CardTextError>,
    ) -> ParseOutcome<Vec<EffectAst>> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("inner-chain-registry-reading"),
                span,
                error,
            )),
        }
    }
}

/// One reading: a stable id, the head that admits it, a further admission
/// test, and the reader.
struct Reading {
    id: RuleId,
    head: HeadDiscriminator,
    admits: fn(&InnerChain<'_>) -> bool,
    read: fn(&InnerChain<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("inner-chain-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("return-coordinated-objects"),
        head: HeadDiscriminator::Any,
        admits: |input| input.tokens.first().is_some_and(|token| token.is_word("return"))
            && !super::super::lex_chain_helpers::has_authored_comma_then_surface_lexed(input.tokens),
        read: |input| input.outcome(super::super::subject_verb_primitives::parse_sentence_return_multiple_targets(
            super::super::SubjectVerbPrimitiveClause::new(input.tokens))),
    },
    Reading {
        id: RuleId::new("sacrifice-it-next-end-step"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sacrifice_it_next_end_step(input)),
    },
    Reading {
        id: RuleId::new("atomic-token-copy-exception"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_atomic_token_copy_exception(input)),
    },
    Reading {
        id: RuleId::new("inline-looked-card-partition-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_inline_looked_card_partition_chain(input)),
    },
    Reading {
        id: RuleId::new("shuffle-graveyard-into-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shuffle_graveyard_into_library(input)),
    },
    Reading {
        id: RuleId::new("gain-then-get"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_gain_then_get(input)),
    },
    Reading {
        id: RuleId::new("control-flow-plan"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("gain-then-get")
        },
        read: |input| input.outcome(read_control_flow_plan(input)),
    },
    Reading {
        id: RuleId::new("venture-conjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_venture_conjunction(input)),
    },
    Reading {
        id: RuleId::new("keyword-mechanic-without-terminal-punctuation"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_keyword_mechanic_without_terminal_punctuation(input)),
    },
    Reading {
        id: RuleId::new("conditional-become-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_become_pair(input)),
    },
    Reading {
        id: RuleId::new("has-base-power-toughness"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("control-flow-plan")
        },
        read: |input| input.outcome(read_has_base_power_toughness(input)),
    },
    Reading {
        id: RuleId::new("sentence-lose-draw-clash-repeat-process"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sentence_lose_draw_clash_repeat_process(input)),
    },
    Reading {
        id: RuleId::new("sentence-unless-pays"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sentence_unless_pays(input)),
    },
    Reading {
        id: RuleId::new("generic-consult-reveal-until-battlefield-bottom"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_generic_consult_reveal_until_battlefield_bottom(input)),
    },
    Reading {
        id: RuleId::new("consult-traversal-with-inline-followup"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("control-flow-plan")
                && !input.read_by("generic-consult-reveal-until-battlefield-bottom")
        },
        read: |input| input.outcome(read_consult_traversal_with_inline_followup(input)),
    },
    Reading {
        id: RuleId::new("consult-traversal"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("consult-traversal-with-inline-followup")
                && !input.read_by("control-flow-plan")
                && !input.read_by("generic-consult-reveal-until-battlefield-bottom")
        },
        read: |input| input.outcome(read_consult_traversal(input)),
    },
    Reading {
        id: RuleId::new("search-library-sentence"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("control-flow-plan")
        },
        read: |input| input.outcome(read_search_library_sentence(input)),
    },
    Reading {
        id: RuleId::new("source-exiled-bottom-random"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_exiled_bottom_random(input)),
    },
    Reading {
        id: RuleId::new("player-chooses-source-excluded-permanent-then-exiles"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_player_chooses_source_excluded_permanent_then_exiles(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("tap-those-then-unattach-equipment"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tap_those_then_unattach_equipment(input)),
    },
    Reading {
        id: RuleId::new("return-it-then-loses-all-abilities"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_return_it_then_loses_all_abilities(input)),
    },
    Reading {
        id: RuleId::new("explicit-player-subject-clauses"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_explicit_player_subject_clauses(input)),
    },
    Reading {
        id: RuleId::new("quantified-participant-subject-effect"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("control-flow-plan")
                && !input.read_by("explicit-player-subject-clauses")
                && !input.read_by("search-library-sentence")
                && !input.read_by("shuffle-graveyard-into-library")
        },
        read: |input| input.outcome(read_quantified_participant_subject_effect(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &InnerChain<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
    let head = crate::lexer::parser_token_word_refs(input.tokens)
        .first()
        .copied()
        .unwrap_or("");
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();
    for reading in READINGS {
        if !reading.head.accepts(head) || !(reading.admits)(input) {
            continue;
        }
        match (reading.read)(input).within(reading.id) {
            ParseOutcome::Match(matched) => candidates.push(RegistryCandidate::new(
                RegistryRuleMetadata::distinct(reading.id, reading.head),
                matched.value,
                matched.span,
            )),
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => diagnostics.push(diagnostic),
        }
    }
    // Equal readings from two rules are one reading.
    let mut distinct: Vec<RegistryCandidate<Vec<EffectAst>>> = Vec::new();
    for candidate in candidates {
        if !distinct.iter().any(|kept| kept.value == candidate.value) {
            distinct.push(candidate);
        }
    }
    if distinct.len() > 1 {
        crate::parse_trace::event(format!(
            "{REGISTRY}: {} readings: {}",
            distinct.len(),
            distinct
                .iter()
                .map(|candidate| candidate.metadata.id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let outcome = resolve_registry_candidates(REGISTRY, distinct, diagnostics);
    if let ParseOutcome::Match(matched) = &outcome {
        crate::parse_trace::event(format!("{REGISTRY}: {} read the input", matched.value.rule));
    }
    outcome
}

fn read_sacrifice_it_next_end_step(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let recognize_control_flow = input.recognize_control_flow;
    if (!recognize_control_flow || split_trailing_if_clause_lexed(tokens).is_none())
        && let Some(effects) =
            super::super::subject_verb_primitives::parse_sentence_sacrifice_it_next_end_step(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_atomic_token_copy_exception(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Nested consequence parsing enters this inner materializer directly.
    // Preserve the same typed copy-token exception ownership as the public
    // chain entrypoint before coordination exposes an `and` inside the
    // characteristic bundle as another create clause.
    if let Some(effect) = parse_atomic_token_copy_exception(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_inline_looked_card_partition_chain(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_inline_looked_card_partition_chain(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_shuffle_graveyard_into_library(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A hand/graveyard-into-library shuffle owns its complete clause even
    // when a participant loop already stripped the subject; coordination
    // would otherwise sever the zone list from the shuffle verb. An optional
    // shuffle keeps its `may` scope through the may-aware routes.
    if parse_leading_player_may_lexed(tokens).is_none()
        && !chain_grammar::starts_with_may_tokens(tokens)
        && super::super::super::grammar::effects::parse_shuffle_graveyard_shape_lexed(tokens)
            .is_some_and(|shape| shape.has_hand_clause)
        && let Some(effects) =
            super::super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_gain_then_get(input: &InnerChain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let recognize_control_flow = input.recognize_control_flow;
    // A gain/get compound carries one shared target and can also carry a
    // leading duration. Its typed parser must see the intact sentence before
    // generic duration/control-flow wrapping splits the coordinated actions;
    // otherwise a per-card modifier is reduced to a generic dynamic pump.
    if recognize_control_flow
        && super::super::super::grammar::effects::gain_ability_shapes::parse_gain_then_get_shape(
            tokens,
        )
        .is_some()
        && let Some(effects) = super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_control_flow_plan(input: &InnerChain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let recognize_control_flow = input.recognize_control_flow;
    if recognize_control_flow {
        match super::super::super::grammar::effects::control_flow::recognize_control_flow(tokens) {
            crate::recognition::ParseOutcome::Match(matched) => {
                let plan = matched.value;
                let mut effects = if plan.parse_original_with_legacy {
                    parse_effect_chain_inner_lexed_unstacked(tokens, false)?
                } else {
                    parse_effect_chain_inner_lexed(plan.body_tokens)?
                };
                let body_words = crate::lexer::token_word_refs(plan.body_tokens);
                if crate::word_primitives::parse_any_sequence_prefix(
                    &body_words,
                    &[&["discard"], &["then", "discard"]],
                ) {
                    for effect in &mut effects {
                        bind_implicit_player_context(effect, PlayerAst::You);
                    }
                }
                if let Some(control) = plan.into_ast(effects.clone()) {
                    return Ok(Some(vec![EffectAst::ControlFlow(Box::new(control))]));
                }
                return Ok(Some(effects));
            }
            crate::recognition::ParseOutcome::NoMatch => {}
            crate::recognition::ParseOutcome::Error(diagnostic) => {
                return Err(diagnostic.into_card_text_error()).map(Some);
            }
        }
    }
    Ok(None)
}
fn read_venture_conjunction(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // `Venture into the dungeon` is a complete subjectless mechanic action.
    // When it leads a coordinated clause, the general subject/verb splitter
    // can treat it as context for the later explicit-player arm and retain
    // only that arm. Prove the exact mechanic phrase and conjunction before
    // lowering both executable actions.
    if tokens.len() > 5
        && tokens[0].is_word("venture")
        && tokens[1].is_word("into")
        && tokens[2].is_word("the")
        && tokens[3].is_word("dungeon")
        && tokens[4].is_word("and")
    {
        let mut effects = vec![EffectAst::subject_verb_venture_into_dungeon(
            PlayerAst::You,
            false,
        )];
        effects.extend(parse_effect_chain_inner_lexed(&tokens[5..])?);
        let coordination = crate::grammar::effects::coordination::coordination_from_effects(
            crate::model::CoordinationKindAst::Conjunction,
            crate::model::CoordinationOperatorAst::And,
            crate::model::EffectOrderingAst::Unordered,
            effects,
        )
        .expect("leading venture conjunction contains at least two effects");
        return Ok(Some(vec![EffectAst::Coordination(coordination)]));
    }
    Ok(None)
}
fn read_keyword_mechanic_without_terminal_punctuation(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A keyword mechanic at the end of a coordinated chain must not consume
    // the earlier action as part of its target phrase (for example, "you
    // lose 1 life and this creature endures 1"). Let the semantic chain
    // splitter isolate those arms before probing the bare-keyword parser.
    if split_effect_chain_on_and_lexed(tokens).len() <= 1
        && !has_explicit_comma_then_boundary_lexed(tokens)
        && let Some(effect) = parse_keyword_mechanic_without_terminal_punctuation(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_conditional_become_pair(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let leading_duration_shape = chain_grammar::parse_carry_duration_prefix_tokens(tokens);
    let pair_tokens = leading_duration_shape
        .as_ref()
        .map_or(tokens, |shape| shape.rest);
    if let Some(mut effect) =
        super::super::clause_dispatch::parse_conditional_become_pair(pair_tokens)?
    {
        if let Some(shape) = leading_duration_shape {
            super::super::dispatch_entry::apply_leading_duration_to_become_effect(
                &mut effect,
                &shape.duration,
            );
        }
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_has_base_power_toughness(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Keep the duration attached while recognizing a base-P/T clause. The
    // ordinary chain path carries a leading duration separately, but doing so
    // before verb dispatch leaves `creatures ... have base power ...` without
    // the temporal evidence that distinguishes a temporary effect from a
    // static characteristic-setting sentence. A surrounding where-X sentence
    // may also have already removed its binding tail; its typed binding pass
    // will replace the X values after this clause has been lowered.
    if let Some(effect) =
        super::super::for_each_helpers::parse_has_base_power_toughness_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_sentence_lose_draw_clash_repeat_process(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This mechanic is a single authored process even though the generic
    // conjunction splitter sees `lose ... and draw ...` and would otherwise
    // send the draw/clash tail through the ordinary draw parser. Claim the
    // complete shape before splitting coordinated verbs.
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_sentence_lose_draw_clash_repeat_process(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_unless_pays(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if chain_grammar::starts_with_unless_tokens(tokens)
        && let Some(effects) = parse_sentence_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_generic_consult_reveal_until_battlefield_bottom(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::dispatch_inner::parse_generic_consult_reveal_until_battlefield_bottom_subject_verb(
                tokens,
            )?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
fn read_consult_traversal_with_inline_followup(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::consult_family::parse_consult_traversal_with_inline_followup(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_consult_traversal(input: &InnerChain<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(parts) = super::super::consult_family::parse_consult_traversal_sentence(tokens)? {
        return Ok(Some(parts.effects));
    }
    Ok(None)
}
fn read_search_library_sentence(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_search_library_sentence_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_source_exiled_bottom_random(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let source_exiled_bottom_random = {
        let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
        crate::word_primitives::sequence_occurs(&words, &["exiled", "with", "this"])
            && crate::word_primitives::sequence_occurs(&words, &["on", "the", "bottom"])
            && crate::word_primitives::sequence_occurs(&words, &["in", "a", "random"])
    };
    if source_exiled_bottom_random {
        let action_tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
            &tokens[1..]
        } else {
            tokens
        };
        if let Some(surface) =
            super::super::verb_handlers::parse_exiled_with_source_move_surface(action_tokens)
        {
            let verb_index = crate::slice_primitives::select_position(action_tokens, |token| {
                token.is_word("put") || token.is_word("puts")
            })
            .unwrap_or(0);
            let effect = super::super::verb_handlers::parse_put_into_hand(
                &action_tokens[verb_index..],
                None,
            )?
            .with_exiled_with_source_surface(Some(surface));
            return Ok(Some(vec![effect]));
        }
    }
    Ok(None)
}
fn read_player_chooses_source_excluded_permanent_then_exiles(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_player_chooses_source_excluded_permanent_then_exiles(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_tap_those_then_unattach_equipment(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_tap_those_then_unattach_equipment_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_return_it_then_loses_all_abilities(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_return_it_then_loses_all_abilities_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_explicit_player_subject_clauses(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(clauses) =
        super::super::player_subject_sequences::split_explicit_player_subject_clauses(tokens)
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_chain_inner_lexed(clause)?);
        }
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_quantified_participant_subject_effect(
    input: &InnerChain<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_quantified_participant_subject_effect(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
