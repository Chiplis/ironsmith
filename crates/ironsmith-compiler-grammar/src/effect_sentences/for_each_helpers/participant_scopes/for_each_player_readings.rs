//! The readings of one "for each player ..." clause: the typed participant
//! programs (type-slot and creature-type choices, "doesn't control ... loses
//! the game", relative control, "the source attacked", combat-damage history,
//! the "who ..." clauses) read before the generic participant effect chain.
//! Formerly a first-match ladder in `participant_scopes`; every reading runs;
//! two different readings of one input are an ambiguity error.

use super::*;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId, RuleMatch};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

/// The input the readings read.
pub(super) struct ParticipantClause<'a> {
    pub(super) tokens: &'a [OwnedLexToken],
    pub(super) outer: &'a for_each_shapes::ForEachParticipantClauseShape<'a>,
    pub(super) iteration_filter: PlayerFilter,
    pub(super) clause_text: &'a str,
    pub(super) slot_chooser: PlayerAst,
}

impl ParticipantClause<'_> {
    /// A reading's outcome: its error is a committed diagnostic on the input.
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("for-each-player-registry-reading"),
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
    admits: fn(&ParticipantClause<'_>) -> bool,
    read: fn(&ParticipantClause<'_>) -> ParseOutcome<EffectAst>,
}

pub(super) const REGISTRY: RuleId = RuleId::new("for-each-player-registry");

/// The readings, in the order they were ranked.
const READINGS: &[Reading] = &[
    Reading {
        id: RuleId::new("for-each-type-slot-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_for_each_type_slot_choice(input)),
    },
    Reading {
        id: RuleId::new("participant-creature-type-choice"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_participant_creature_type_choice(input)),
    },
    Reading {
        id: RuleId::new("doesnt-control-lose-game"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_doesnt_control_lose_game(input)),
    },
    Reading {
        id: RuleId::new("relative-control-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_relative_control_clause(input)),
    },
    Reading {
        id: RuleId::new("source-attacked-player"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_source_attacked_player(input)),
    },
    Reading {
        id: RuleId::new("combat-damage-history-participant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_combat_damage_history_participant(input)),
    },
    Reading {
        id: RuleId::new("who-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_who_clause(input)),
    },
];

/// The input's reading, if a rule has one. Every admitted reading runs.
pub(super) fn read(input: &ParticipantClause<'_>) -> ParseOutcome<RuleMatch<EffectAst>> {
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
    let mut distinct: Vec<RegistryCandidate<EffectAst>> = Vec::new();
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

fn read_for_each_type_slot_choice(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    let slot_chooser = input.slot_chooser;
    if let Some(effects) = super::super::super::parse_for_each_type_slot_choice_clause(
        outer.inner_tokens,
        slot_chooser,
    )? {
        return Ok(Some(wrap_players(&iteration_filter, effects)));
    }
    Ok(None)
}
fn read_participant_creature_type_choice(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    let slot_chooser = input.slot_chooser;
    if let Some(effects) = parse_participant_creature_type_choice(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_players(&iteration_filter, effects)));
    }
    Ok(None)
}
fn read_doesnt_control_lose_game(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let iteration_filter = input.iteration_filter.clone();
    if iteration_filter == PlayerFilter::Any
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, false)?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_relative_control_clause(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    let clause_text = input.clause_text;
    if let Some(relative) = for_each_shapes::parse_relative_control_clause_shape(outer.inner_tokens)
    {
        let conditional =
            parse_relative_control_conditional(relative, outer.participant_is_actor, &clause_text)?;
        return Ok(Some(wrap_players(&iteration_filter, vec![conditional])));
    }
    Ok(None)
}
fn read_source_attacked_player(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    if iteration_filter == PlayerFilter::Any
        && let Some(source_attacked) =
            for_each_shapes::parse_source_attacked_player_clause_shape(outer.inner_tokens)
    {
        let normalized = prepend_that_player_subject(source_attacked.effect_tokens);
        let effects = parse_maybe_effects(&normalized, false, true)?;
        return Ok(Some(EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::AttackedBySourceThisTurn,
            effects,
        }));
    }
    Ok(None)
}
fn read_combat_damage_history_participant(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    if let Some(effect) =
        parse_combat_damage_history_participant(outer.inner_tokens, iteration_filter.clone())?
    {
        return Ok(Some(effect));
    }
    Ok(None)
}
fn read_who_clause(input: &ParticipantClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    let clause_text = input.clause_text;
    if let Some(who) = for_each_shapes::parse_who_clause_shape(outer.inner_tokens) {
        match who {
            WhoClauseShape::TappedLandForMana { effect_tokens } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                        clause_text
                    )));
                }
                let branch_effects = parse_maybe_effects(effect_tokens, true, false)?;
                return Ok(Some(wrap_players(
                    &iteration_filter,
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                            player: PlayerAst::That,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                )));
            }
            WhoClauseShape::Negated {
                effect_tokens,
                tagged_filter_tokens,
                implicit_player_is_iterated,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect in for each player who doesn't clause (clause: '{}')",
                        clause_text
                    )));
                }
                let scoped_effect_tokens =
                    implicit_player_is_iterated.then(|| prepend_that_player_subject(effect_tokens));
                return Ok(Some(EffectAst::ForEachPlayerDoesNot {
                    effects: parse_effect_chain_inner(
                        scoped_effect_tokens.as_deref().unwrap_or(effect_tokens),
                    )?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                }));
            }
            WhoClauseShape::DidThisWay {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who ... this way' (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachPlayerDid {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                    result_predicate: IfResultPredicate::Did,
                }));
            }
            WhoClauseShape::DidAction {
                effect_tokens,
                implicit_player_is_you,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who does' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut effects = parse_effect_chain_inner(effect_tokens)?;
                let player = if implicit_player_is_you {
                    PlayerAst::You
                } else {
                    PlayerAst::That
                };
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player);
                }
                return Ok(Some(EffectAst::ForEachPlayerDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                }));
            }
        }
    }
    Ok(None)
}
