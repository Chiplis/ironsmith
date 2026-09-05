//! The readings of one effect chain at its entry, before comma-then
//! splitting and the uncoordinated chain: the typed whole-chain shapes
//! (optional library placement, delayed schedules, joint subjects, counter
//! coordinations, copy-token exceptions, distributed targets, venture, "where
//! X" bindings, damage fanouts, ...). Formerly a first-match ladder in
//! `chain_carry`; every reading runs, two different readings of one input are an
//! ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct ChainEntry<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl ChainEntry<'_> {
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
                RuleId::new("chain-entry-registry-reading"),
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
    admits: fn(&ChainEntry<'_>) -> bool,
    read: fn(&ChainEntry<'_>) -> ParseOutcome<Vec<EffectAst>>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("chain-entry-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("optional-library-placement"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_optional_library_placement(input)),
    },
    Reading {
        id: RuleId::new("sentence-delayed-next-step-unless-pays"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sentence_delayed_next_step_unless_pays(input)),
    },
    Reading {
        id: RuleId::new("shuffle-object-into-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shuffle_object_into_library(input)),
    },
    Reading {
        id: RuleId::new("source-and-tagged-object-each-actions"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_and_tagged_object_each_actions(input)),
    },
    Reading {
        id: RuleId::new("repeated-counter-placement-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_repeated_counter_placement_coordination(input)),
    },
    Reading {
        id: RuleId::new("atomic-put-counter-for-each"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_atomic_put_counter_for_each(input)),
    },
    Reading {
        id: RuleId::new("atomic-token-copy-exception"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_atomic_token_copy_exception(input)),
    },
    Reading {
        id: RuleId::new("target-player-resource-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_target_player_resource_coordination(input)),
    },
    Reading {
        id: RuleId::new("tagged-conditional-entry-counters"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_tagged_conditional_entry_counters(input)),
    },
    Reading {
        id: RuleId::new("sentence-distribute-counters"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_sentence_distribute_counters(input)),
    },
    Reading {
        id: RuleId::new("trailing-venture-conjunction"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_trailing_venture_conjunction(input)),
    },
    Reading {
        id: RuleId::new("each-prior-affected-object-controller-mana-value-life"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| {
            input.outcome(read_each_prior_affected_object_controller_mana_value_life(
                input,
            ))
        },
    },
    Reading {
        id: RuleId::new("terminal-where-x-binding"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_terminal_where_x_binding(input)),
    },
    Reading {
        id: RuleId::new("until-duration-triggered"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_until_duration_triggered(input)),
    },
    Reading {
        id: RuleId::new("unpreventable-damage-rider"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_unpreventable_damage_rider(input)),
    },
    Reading {
        id: RuleId::new("copy-spell-with-exception"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_copy_spell_with_exception(input)),
    },
    Reading {
        id: RuleId::new("shuffle-graveyard-into-library"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_shuffle_graveyard_into_library(input)),
    },
    Reading {
        id: RuleId::new("paid-label-condition"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("terminal-where-x-binding")
        },
        read: |input| input.outcome(read_paid_label_condition(input)),
    },
    Reading {
        id: RuleId::new("conditional-become-pair"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_conditional_become_pair(input)),
    },
    Reading {
        id: RuleId::new("for-each-object-effect-chain"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_for_each_object_effect_chain(input)),
    },
    Reading {
        id: RuleId::new("independent-explicit-may-coordination"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_independent_explicit_may_coordination(input)),
    },
    Reading {
        id: RuleId::new("compound-damage-fanout"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_compound_damage_fanout(input)),
    },
    Reading {
        id: RuleId::new("remove-counters-then-shared-damage-fanout"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_remove_counters_then_shared_damage_fanout(input)),
    },
    Reading {
        id: RuleId::new("leading-action-then-shared-damage-fanout"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("remove-counters-then-shared-damage-fanout")
        },
        read: |input| input.outcome(read_leading_action_then_shared_damage_fanout(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ChainEntry<'_>) -> ParseOutcome<RuleMatch<Vec<EffectAst>>> {
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

fn read_optional_library_placement(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(stripped) = grammar::strip_lexed_prefix_phrase(tokens, &["you", "may"]) {
        let player = PlayerAst::You;
        let stripped = crate::util::trim_edge_punctuation_tokens(stripped);
        let placement_tokens =
            grammar::strip_lexed_suffix_phrase(stripped, &["instead"]).unwrap_or(stripped);
        if let Some(effect) = parse_simple_that_creature_owner_library_placement(placement_tokens) {
            return Ok(Some(vec![EffectAst::MayByPlayer {
                player,
                effects: vec![effect],
            }]));
        }
    }
    Ok(None)
}
fn read_sentence_delayed_next_step_unless_pays(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Action-first delayed-step sentences expose an ordinary resource verb
    // before their schedule. Claim the complete typed schedule/payment shape
    // before broad resource dispatch tries to consume the timing suffix as
    // part of the life-loss operand.
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_sentence_delayed_next_step_unless_pays(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_shuffle_object_into_library(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if parse_leading_player_may_lexed(tokens).is_none()
        && let Some(effects) =
            super::super::subject_verb_primitives::parse_sentence_shuffle_object_into_library(
                SubjectVerbPrimitiveClause::new(tokens),
            )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_source_and_tagged_object_each_actions(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A joint object subject owns every conjunction in its shared action
    // tail (`this creature and that creature each get ... and gain ...`).
    // Claim the grammar-proven subject before generic chain splitting can
    // mistake the subject's first `and` for an effect boundary.
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_source_and_tagged_object_each_actions_sentence(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_repeated_counter_placement_coordination(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_repeated_counter_placement_coordination(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_atomic_put_counter_for_each(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A dynamic counter amount can itself be a coordinated object domain:
    // `for each suspended card ... and each other permanent ...`. Once the
    // count grammar consumes that entire suffix, its conjunction is data,
    // not an effect boundary. Materialize the one typed counter action before
    // generic sentence coordination can expose either count arm as a target.
    if is_atomic_put_counter_for_each_sentence(tokens) {
        return Ok(Some(vec![
            super::super::zone_counter_helpers::parse_put_counters(tokens)?,
        ]));
    }
    Ok(None)
}
fn read_atomic_token_copy_exception(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A copy-token exception is part of one creation action even when its
    // characteristic bundle contains `and` (`except it's 1/1 and it's a
    // Nightmare ...`). Let the typed creation grammar prove and materialize
    // the complete shape before effect coordination can expose a modifier as
    // a second create clause.
    if let Some(effect) = parse_atomic_token_copy_exception(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_target_player_resource_coordination(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A target-player subject can govern several coordinated resource
    // actions (`loses life, gets a poison counter, then mills`). The typed
    // coordination recognizer proves the multi-member shape; route it to the
    // chain materializer before whole-sentence primitive registries can treat
    // the leading `target` as an object-selection verb and try to parse the
    // remaining player action as an object filter.
    if has_target_player_resource_coordination(tokens) {
        return parse_effect_chain_inner_lexed(tokens).map(Some);
    }
    Ok(None)
}
fn read_tagged_conditional_entry_counters(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A conditional entry-counter list is one atomic subject/verb sentence:
    // every `and an additional ... if it's ...` arm is a sibling action on
    // the same returned set. Claim it before generic conjunction splitting,
    // which otherwise treats the second counter descriptor as a continuation
    // of the first condition and nests the two predicates.
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_tagged_conditional_entry_counters_sentence(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_sentence_distribute_counters(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A distributed-target range may contain authored commas of its own
    // (`among one, two, or three target creatures`).  Claim the complete
    // typed sentence before generic comma-chain splitting can mistake those
    // list separators for executable boundaries and truncate the target
    // phrase at `one`.
    if let Some(effects) =
        super::super::subject_verb_primitives::parse_sentence_distribute_counters(
            SubjectVerbPrimitiveClause::new(tokens),
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_trailing_venture_conjunction(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let venture_view = TokenWordView::new(tokens);
    if let Some(venture_word) =
        venture_view.parse_phrase_start(&["and", "venture", "into", "the", "dungeon"])
        && let Some(venture_start) = venture_view.map_word_to_token_start(venture_word)
        && let Some(venture_end) = venture_view.token_index_after_words(venture_word + 5)
        && tokens[venture_end..]
            .iter()
            .all(|token| token.as_word().is_none())
    {
        let mut effects = parse_effect_chain_lexed(&tokens[..venture_start])?;
        effects.push(EffectAst::subject_verb_venture_into_dungeon(
            PlayerAst::You,
            false,
        ));
        let coordination = crate::grammar::effects::coordination::coordination_from_effects(
            crate::model::CoordinationKindAst::Conjunction,
            crate::model::CoordinationOperatorAst::And,
            crate::model::EffectOrderingAst::Unordered,
            effects,
        )
        .expect("venture conjunction contains at least two effects");
        return Ok(Some(vec![EffectAst::Coordination(coordination)]));
    }
    Ok(None)
}
fn read_each_prior_affected_object_controller_mana_value_life(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_each_prior_affected_object_controller_mana_value_life(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_terminal_where_x_binding(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some((leading_tokens, where_value)) = parse_terminal_where_x_binding(tokens) {
        let mut effects = parse_effect_chain_lexed(leading_tokens)?;
        replace_unbound_x_in_effects_anywhere(
            &mut effects,
            &where_value,
            &token_word_refs(tokens).join(" "),
        )?;
        ensure_explicit_target_player_subject_declarations(&mut effects, leading_tokens);
        dedupe_shared_target_player_draw_lose_x(&mut effects, tokens);
        preserve_independent_target_player_coordination(&mut effects, leading_tokens);
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_until_duration_triggered(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // `Until ..., whenever ...` is one delayed-trigger clause. Keep that
    // typed outer scope intact before general conjunction/duration chain
    // recognition can expose words in the trigger event as direct effect
    // heads (for example, `deals combat damage` or `becomes tapped`).
    if let Some(effect) =
        super::super::clause_primitives::parse_until_duration_triggered_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
fn read_unpreventable_damage_rider(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // The final prevention rider belongs to the damage action. Preserve
    // object-or-player recipient unions before generic coordination can
    // split their `or` arm into a standalone restriction clause.
    if super::super::verb_handlers::damage_clause_has_terminal_unpreventable_rider(tokens) {
        let damage_tokens =
            super::super::lex_chain_helpers::strip_leading_instead_prefix_lexed(tokens)
                .unwrap_or(tokens);
        return Ok(Some(vec![parse_effect_clause_lexed(damage_tokens)?]));
    }
    Ok(None)
}
fn read_copy_spell_with_exception(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let copy_shape =
            super::super::super::grammar::effects::clause_pattern_shapes::parse_copy_clause_shape_tokens(
                tokens,
            );
    if copy_shape.is_some_and(|shape| shape.copy_word == 0 && shape.tail.exception_split.is_some())
        && let Some(copy) = super::super::clause_pattern_helpers::parse_copy_spell_clause(tokens)?
    {
        return Ok(Some(vec![copy]));
    }
    Ok(None)
}
fn read_shuffle_graveyard_into_library(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A hand/graveyard-into-library shuffle owns its complete clause. The
    // chain splitter would otherwise sever the coordinated zone list ("their
    // hand and graveyard into their library") and hand the shuffle verb a
    // bare "their hand" fragment.
    if let Some(effects) =
        super::super::search_library::parse_shuffle_graveyard_into_library_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_paid_label_condition(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if leading_condition_is_paid_label(tokens) {
        let Some(mut effects) =
            parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
        else {
            return Err(CardTextError::ParseError(
                "paid-label condition did not parse as a conditional sentence".to_string(),
            ))
            .map(Some);
        };
        preserve_leading_result_coordination_lexed(tokens, &mut effects);
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_conditional_become_pair(
    input: &ChainEntry<'_>,
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
fn read_for_each_object_effect_chain(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Conditional consequence parsing can call the chain entrypoint directly
    // before the ordinary uncoordinated dispatcher gets a chance to inspect
    // the complete `for each ..., effect` shape. Keep that grammar atomic at
    // this boundary so the iterator's subject is not sent to the generic
    // subject/verb parser as an orphaned clause.
    if let Some(effects) = parse_for_each_object_effect_chain_shape(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_independent_explicit_may_coordination(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_independent_explicit_may_coordination(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_compound_damage_fanout(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Conditional consequence parsing enters through the chain boundary.
    // Preserve a repeated damage head as one typed fanout before the
    // generic conjunction splitter leaves the second amount without its
    // shared source/verb.
    if let Some(effects) =
        super::super::fanout_family::parse_compound_damage_fanout_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
fn read_remove_counters_then_shared_damage_fanout(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::fanout_family::parse_remove_counters_then_shared_damage_fanout(tokens)?
    {
        return Ok(Some(vec![EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        }]));
    }
    Ok(None)
}
fn read_leading_action_then_shared_damage_fanout(
    input: &ChainEntry<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_leading_action_then_shared_damage_fanout(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
