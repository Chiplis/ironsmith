//! The readings of one "for each opponent ..." clause: the typed participant
//! programs (type-slot and creature-type choices, "doesn't control ... loses
//! the game", relative control, "the source attacked", combat-damage history,
//! the "who ..." clauses) read before the generic participant effect chain.
//! Formerly a first-match ladder in `opponent_iteration`; every reading runs;
//! two different readings of one input are an ambiguity error.

use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LifeResourceActionAst;
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
    /// Which readings of this registry read this input, once asked.
    pub(super) read_by_cache: std::cell::RefCell<std::collections::HashMap<&'static str, bool>>,
}

impl ParticipantClause<'_> {
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
    fn outcome(&self, read: Result<Option<EffectAst>, CardTextError>) -> ParseOutcome<EffectAst> {
        let span = crate::util::span_from_tokens(self.tokens);
        match read {
            Ok(Some(value)) => ParseOutcome::matched(value, span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("for-each-opponent-registry-reading"),
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

pub(super) const REGISTRY: RuleId = RuleId::new("for-each-opponent-registry");

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
        id: RuleId::new("combat-damage-history-participant"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_combat_damage_history_participant(input)),
    },
    Reading {
        id: RuleId::new("opponent-special-shape"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_opponent_special_shape(input)),
    },
    Reading {
        id: RuleId::new("who-clause"),
        head: HeadDiscriminator::Any,
        admits: |input| {
            // Readings ranked above this one that read the input read it.
            !input.read_by("doesnt-control-lose-game")
        },
        read: |input| input.outcome(read_who_clause(input)),
    },
    Reading {
        id: RuleId::new("actor-return-clause"),
        head: HeadDiscriminator::Any,
        admits: |_| true,
        read: |input| input.outcome(read_actor_return_clause(input)),
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
        return Ok(Some(wrap_opponents(&iteration_filter, effects)));
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
        return Ok(Some(wrap_opponents(&iteration_filter, effects)));
    }
    Ok(None)
}
fn read_doesnt_control_lose_game(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = input.tokens;
    let iteration_filter = input.iteration_filter.clone();
    if iteration_filter == PlayerFilter::Opponent
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, true)?
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
        return Ok(Some(wrap_opponents(&iteration_filter, vec![conditional])));
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
fn read_opponent_special_shape(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    let clause_text = input.clause_text;
    if let Some(special) = for_each_shapes::parse_opponent_special_shape(outer.inner_tokens)? {
        match special {
            OpponentSpecialShape::IgnoreScryOrSurveil => return Ok(None),
            OpponentSpecialShape::ChooseReturnUnlessDraw { target_tokens } => {
                let target = parse_target_phrase(target_tokens)?;
                let return_target =
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![
                        EffectAst::subject_verb_target_only(target),
                        EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                            effects: vec![EffectAst::subject_verb_return_to_hand(
                                return_target,
                                false,
                            )],
                            alternative: vec![EffectAst::subject_verb(
                                SubjectVerbRoleAst::AffectedPlayer,
                                PlayerAst::You,
                                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                                    count: Value::Fixed(1),
                                }),
                            )],
                            player: PlayerAst::ItsController,
                        }),
                    ],
                )));
            }
            OpponentSpecialShape::LessLifeThanYou { effect_tokens } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who has less life than you' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut branch_effects = parse_maybe_effects(effect_tokens, false, false)?;
                force_implicit_token_controller_you(&mut branch_effects);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                        predicate: PredicateAst::Player(PlayerPredicateAst::PlayerHasLessLifeThanYou {
                            player: PlayerAst::That,
                        }),
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    })],
                )));
            }
            OpponentSpecialShape::PoisonCounters {
                count,
                effect_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who has ... poison counters' (clause: '{}')",
                        clause_text
                    )));
                }
                let mut branch_effects = parse_effect_chain(effect_tokens)?;
                force_implicit_token_controller_you(&mut branch_effects);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                        predicate: PredicateAst::Player(PlayerPredicateAst::PlayerHasPoisonCountersOrMore {
                            player: PlayerAst::That,
                            count,
                        }),
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    })],
                )));
            }
        }
    }
    Ok(None)
}
fn read_who_clause(input: &ParticipantClause<'_>) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
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
                return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                    effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                        predicate: PredicateAst::Player(PlayerPredicateAst::PlayerTappedLandForManaThisTurn {
                            player: PlayerAst::That,
                        }),
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    })],
                })));
            }
            WhoClauseShape::Negated {
                effect_tokens,
                tagged_filter_tokens,
                implicit_player_is_iterated,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect in for each opponent who doesn't clause (clause: '{}')",
                        clause_text
                    )));
                }
                let scoped_effect_tokens =
                    implicit_player_is_iterated.then(|| prepend_that_player_subject(effect_tokens));
                return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDoesNot {
                    effects: parse_effect_chain_inner(
                        scoped_effect_tokens.as_deref().unwrap_or(effect_tokens),
                    )?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                })));
            }
            WhoClauseShape::DidThisWay {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who ... this way' (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDid {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                    result_predicate: IfResultPredicate::Did,
                })));
            }
            WhoClauseShape::DidAction {
                effect_tokens,
                implicit_player_is_you,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each opponent who does' (clause: '{}')",
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
                return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                })));
            }
        }
    }
    Ok(None)
}
fn read_actor_return_clause(
    input: &ParticipantClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    let outer = input.outer;
    let iteration_filter = input.iteration_filter.clone();
    if outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("return") || token.is_word("returns"))
    {
        let return_tokens = crate::util::trim_edge_punctuation_tokens(&outer.inner_tokens[1..]);
        let mut effect = super::super::super::zone_handlers::parse_return(return_tokens)?;
        bind_implicit_player_context(&mut effect, PlayerAst::That);
        return Ok(Some(wrap_players(&iteration_filter, vec![effect])));
    }
    Ok(None)
}
