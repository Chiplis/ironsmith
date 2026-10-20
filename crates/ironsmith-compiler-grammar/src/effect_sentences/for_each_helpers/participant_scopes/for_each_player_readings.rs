//! The readings of one "for each player ..." clause: the typed participant
//! programs (type-slot and creature-type choices, "doesn't control ... loses
//! the game", relative control, "the source attacked", combat-damage history,
//! the "who ..." clauses) read before the generic participant effect chain.
//! Formerly a first-match ladder in `participant_scopes`; every reading runs;
//! two different readings of one input are an ambiguity error.
use crate::zone::Zone;

use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::ForEachEffectAst;
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
        if relative.controls_fewest {
            if iteration_filter != PlayerFilter::Any || !outer.participant_is_actor {
                return Err(CardTextError::ParseError(
                    "unsupported fewest-controller participant scope".into(),
                ));
            }
            let mut filter = parse_object_filter(relative.filter_tokens, false)?;
            if filter.controller.is_some()
                || filter.zone.is_some_and(|zone| zone != Zone::Battlefield)
            {
                return Err(CardTextError::ParseError("fewest-controller scope must name battlefield objects without another controller".into()));
            }
            filter.zone = Some(Zone::Battlefield);
            let normalized = prepend_that_player_subject(relative.effect_tokens);
            let effects = parse_maybe_effects(&normalized, true, false)?;
            // Selecting the full tied set happens before any token creation
            // changes counts; a per-participant condition would be too late.
            return Ok(Some(EffectAst::ForEach(
                ForEachEffectAst::ForEachPlayersFiltered {
                    filter: PlayerFilter::ControlsFewestTied {
                        filter: Box::new(filter),
                    },
                    effects,
                    sequential: false,
                },
            )));
        }
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
        return Ok(Some(EffectAst::ForEach(
            ForEachEffectAst::ForEachPlayersFiltered {
                sequential: false,
                filter: PlayerFilter::AttackedBySourceThisTurn,
                effects,
            },
        )));
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
                // The relative clause restricts which players act; it is not
                // an optional branch. A trailing-if gate keeps the acting
                // players' results (the lands they sacrificed) as the
                // antecedent of a following "each player who sacrificed a
                // Plains this way" (Desolation).
                return Ok(Some(wrap_players(
                    &iteration_filter,
                    vec![EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
                        predicate: PredicateAst::Player(
                            PlayerPredicateAst::PlayerTappedLandForManaThisTurn {
                                player: PlayerAst::That,
                            },
                        ),
                        effects: branch_effects,
                    })],
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
                return Ok(Some(EffectAst::ForEach(
                    ForEachEffectAst::ForEachPlayerDoesNot {
                        effects: parse_effect_chain_inner(
                            scoped_effect_tokens.as_deref().unwrap_or(effect_tokens),
                        )?,
                        predicate: tagged_predicate(tagged_filter_tokens),
                    },
                )));
            }
            WhoClauseShape::DidThisWay {
                result_tokens,
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect after 'each player who ... this way' (clause: '{}')",
                        clause_text
                    )));
                }
                // "Each player who discarded a card with the greatest mana
                // value among cards discarded this way loses life equal to
                // that mana value." (Scythe Specter): the scope is every card
                // the preceding discard tagged, and "that mana value" is the
                // same greatest mana value.
                if let Some(filter_tokens) = tagged_filter_tokens {
                    let filter_words = crate::lexer::parser_token_word_refs(filter_tokens);
                    let effect_words = crate::lexer::parser_token_word_refs(effect_tokens);
                    const EXTREMUM: &[&str] = &[
                        "with", "the", "greatest", "mana", "value", "among", "cards", "discarded",
                    ];
                    if filter_words.len() > EXTREMUM.len()
                        && filter_words.ends_with(EXTREMUM)
                        && matches!(filter_words[0], "a" | "an")
                        && filter_words[1..filter_words.len() - EXTREMUM.len()] == ["card"]
                        && let Some(that_idx) = effect_tokens.windows(3).position(|window| {
                            window[0].is_word("that")
                                && window[1].is_word("mana")
                                && window[2].is_word("value")
                        })
                        && effect_words.windows(3).filter(|window| *window == ["that", "mana", "value"]).count() == 1
                    {
                        let discarded = crate::tag::CompilerReferenceTag::It;
                        let greatest = crate::effect::Value::GreatestManaValue(
                            crate::filter::ObjectFilter::tagged(discarded.key()),
                        );
                        let mut card = crate::filter::ObjectFilter::default();
                        card.set_explicit_card_noun(true);
                        card.mana_value =
                            Some(crate::filter::Comparison::EqualExpr(Box::new(greatest.clone())));
                        let mut rewritten = effect_tokens[..that_idx].to_vec();
                        rewritten.push(crate::lexer::OwnedLexToken::synthetic_word("x"));
                        rewritten.extend_from_slice(&effect_tokens[that_idx + 3..]);
                        let mut effects = parse_effect_chain_inner(&rewritten)?;
                        crate::effect_sentences::dispatch_entry::replace_unbound_x_in_effects_anywhere(
                            &mut effects,
                            &greatest,
                            clause_text,
                        )?;
                        return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid {
                            effects,
                            predicate: Some(PredicateAst::Player(
                                PlayerPredicateAst::PlayerTaggedObjectMatches {
                                    player: PlayerAst::That,
                                    tag: discarded.bind(),
                                    filter: card,
                                    mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
                                },
                            )),
                            result_predicate: IfResultPredicate::Did,
                        })));
                    }
                }
                // "Then each player who owns a spell you cast this way loses
                // life equal to its mana value." (Kefka, Dancing Mad): the
                // relative clause ranges over the spells just cast, once per
                // spell, and its owner is the acting player; "its" names that
                // spell.
                let result_words = crate::lexer::parser_token_word_refs(result_tokens);
                if matches!(
                    result_words.as_slice(),
                    ["who", "owns", "a" | "an", "spell", "you", "cast", "this", "way"]
                ) {
                    let mut rewritten =
                        crate::lexer::synthetic_word_tokens(["its", "owner"]);
                    rewritten.extend_from_slice(effect_tokens);
                    let effects = parse_effect_chain_inner(&rewritten)?;
                    return Ok(Some(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                        tag: crate::tag::CompilerReferenceTag::It.bind(),
                        effects,
                    })));
                }
                let mut result_clause = result_tokens.to_vec();
                if let Some(subject) = result_clause.first_mut() {
                    subject.replace_word("player");
                }
                let result_predicate =
                    match crate::grammar::modal_results::parse_if_result_predicate_lexed_tokens(
                        &result_clause,
                    ) {
                        Some(IfResultPredicate::SearchedLibrary) => {
                            IfResultPredicate::SearchedLibrary
                        }
                        _ => IfResultPredicate::Did,
                    };
                // "Each player who shuffled ... this way reveals cards from
                // the top of their library until ..." (Collision of Realms):
                // a subjectless consult otherwise defaults its library owner
                // to you; the acting player is the iterated one.
                let effect_words = crate::lexer::parser_token_word_refs(effect_tokens);
                let scoped_effect_tokens = (effect_words
                    .first()
                    .is_some_and(|word| matches!(*word, "reveals" | "exiles" | "mills"))
                    && effect_words.contains(&"until"))
                .then(|| prepend_that_player_subject(effect_tokens));
                return Ok(Some(EffectAst::ForEach(
                    ForEachEffectAst::ForEachPlayerDid {
                        effects: parse_effect_chain_inner(
                            scoped_effect_tokens.as_deref().unwrap_or(effect_tokens),
                        )?,
                        predicate: tagged_past_action_predicate(
                            tagged_filter_tokens,
                            result_tokens,
                        ),
                        result_predicate,
                    },
                )));
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
                return Ok(Some(EffectAst::ForEach(
                    ForEachEffectAst::ForEachPlayerDid {
                        effects,
                        predicate: None,
                        result_predicate: IfResultPredicate::AcceptedChoice,
                    },
                )));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod fewest_participant_tests {
    use super::*;
    #[test]
    fn fewest_relative_clause_lowers_to_a_captured_player_set_not_a_repeated_condition() {
        let tokens=crate::lexer::lex_line("Each player who controls the fewest creatures creates a 4/3 blue Salamander Warrior creature token.",0).unwrap();
        let (parsed, loss) = ironsmith_compiler::parse_loss::capture(|| {
            crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        });
        let parsed = parsed.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let [
            EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::ControlsFewestTied { filter },
                effects,
                sequential: false,
            }),
        ] = parsed.as_slice()
        else {
            panic!("{parsed:#?}")
        };
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(
            filter
                .card_types
                .contains(&crate::types::CardType::Creature)
        );
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, EffectAst::Conditionals(_))),
            "all eligible players are selected before any creates a token"
        );
        let unsupported = crate::lexer::lex_line(
            "Each opponent who controls the fewest creatures draws a card.",
            0,
        )
        .unwrap();
        assert!(
            crate::effect_sentences::parse_effect_sentence_lexed(&unsupported).is_err(),
            "do not silently substitute a different comparison scope"
        );
    }
}
