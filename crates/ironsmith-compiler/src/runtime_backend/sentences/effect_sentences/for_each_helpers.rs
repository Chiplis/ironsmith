use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, IfResultPredicate, OwnedLexToken, PlayerAst,
    PredicateAst, SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey,
    TargetAst,
};
use crate::diagnostics::TextSpan;
use crate::effect::{Until, Value};
use crate::target::{ObjectFilter, ObjectRef, PlayerFilter};
use ironsmith_core::ValueSurfaceHint;

use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::effects::for_each_shapes::{
    self, ForEachParticipantScope, ManaClauseShape, ModifierTailAction, OpponentSpecialShape,
    WhoClauseShape,
};
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::LexedClause;
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_for_each_count_value_words, parse_target_phrase, replace_unbound_x_with_value,
    value_contains_unbound_x,
};
use super::chain_carry::bind_implicit_player_context;
use super::chain_carry::{parse_effect_chain, parse_effect_chain_inner, remove_first_word};
use super::conditionals::parse_for_each_doesnt_control_lose_game;

fn prepend_that_player_life_total_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    if !for_each_shapes::starts_life_total_becomes(tokens) {
        return tokens.to_vec();
    }
    prepend_that_player_subject(tokens)
}

fn prepend_that_player_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = Vec::with_capacity(tokens.len() + 2);
    rewritten.push(OwnedLexToken::word(
        "that".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.push(OwnedLexToken::word(
        "players".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.extend_from_slice(tokens);
    rewritten
}

pub(crate) fn parse_for_each_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_object_subject_shape(subject_tokens) else {
        return Ok(None);
    };
    Ok(Some(parse_object_filter(shape.filter_tokens, false)?))
}

pub(crate) fn parse_for_each_targeted_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_subject_shape(subject_tokens) else {
        return Ok(None);
    };
    let target = match parse_target_phrase(shape.target_tokens) {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    let TargetAst::WithCount(inner, count) = target else {
        return Ok(None);
    };
    let TargetAst::Object(filter, _, _) = *inner else {
        return Ok(None);
    };
    Ok(Some((filter, count)))
}

pub(crate) fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    for_each_shapes::is_target_player_damage_subject_words(words)
}

pub(crate) fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    for_each_shapes::parse_mana_clause_shape_words(words) == Some(ManaClauseShape::Replacement)
}

pub(crate) fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    for_each_shapes::parse_mana_clause_shape_words(words)
        == Some(ManaClauseShape::AdditionalTrigger)
}

pub(crate) fn parse_has_base_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_base_power_clause_shape(tokens)? else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    Ok(Some(EffectAst::subject_verb_set_base_power(
        shape.power,
        target,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_has_base_power_toughness_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_base_power_toughness_clause_shape(tokens)? else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    Ok(Some(EffectAst::subject_verb_set_base_power_toughness(
        shape.power,
        shape.toughness,
        target,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_get_for_each_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if for_each_shapes::parse_for_each_target_subject_shape(tokens).is_none() {
        return Ok(None);
    }
    if let Some(value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(tokens)
    {
        return Ok(Some(value));
    }
    let words = LexedClause::new(tokens).word_refs();
    let Some((value, _)) = parse_for_each_count_value_words(&words) else {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    };
    Ok(Some(value))
}

pub(crate) fn parse_get_modifier_values_with_tail(
    modifier_tokens: &[OwnedLexToken],
    power: Value,
    toughness: Value,
) -> Result<(Value, Value, Until, Option<crate::ConditionExpr>), CardTextError> {
    let clause = LexedClause::new(modifier_tokens).text();
    let mut out_power = power;
    let mut out_toughness = toughness;
    let shape = for_each_shapes::parse_modifier_tail_shape(modifier_tokens);

    if let ModifierTailAction::DynamicForEach(count_tokens) = shape.action {
        let Some(count) = parse_get_for_each_count_value(count_tokens)? else {
            return Err(CardTextError::ParseError(
                "missing filter after 'for each' in gets clause".to_string(),
            ));
        };
        let scale_modifier = |modifier: Value| -> Result<Value, CardTextError> {
            match modifier {
                Value::Fixed(0) => Ok(Value::Fixed(0)),
                Value::Fixed(1) => Ok(count.clone()),
                Value::Fixed(multiplier) => Ok(Value::Scaled(Box::new(count.clone()), multiplier)),
                other if value_contains_unbound_x(&other) => {
                    replace_unbound_x_with_value(other, &count, &clause)
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each clause (clause: '{}')",
                    clause
                ))),
            }
        };
        out_power = scale_modifier(out_power)?;
        out_toughness = scale_modifier(out_toughness)?;
        return Ok((out_power, out_toughness, shape.duration, shape.condition));
    }

    let ModifierTailAction::WhereX(binding_tokens) = shape.action else {
        return match shape.action {
            ModifierTailAction::Complete => {
                Ok((out_power, out_toughness, shape.duration, shape.condition))
            }
            ModifierTailAction::Unsupported => Err(CardTextError::ParseError(format!(
                "unsupported trailing gets clause (clause: '{}')",
                clause
            ))),
            ModifierTailAction::DynamicForEach(_) | ModifierTailAction::WhereX(_) => unreachable!(),
        };
    };
    if !value_contains_unbound_x(&out_power) && !value_contains_unbound_x(&out_toughness) {
        return Err(CardTextError::ParseError(format!(
            "where-X gets clause missing X modifier (clause: '{}')",
            clause
        )));
    }
    let x_value = parse_value_binding_clause(binding_tokens)
        .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-X gets clause (clause: '{}')",
                clause
            ))
        })?;
    out_power = replace_unbound_x_with_value(out_power, &x_value, &clause)?;
    out_toughness = replace_unbound_x_with_value(out_toughness, &x_value, &clause)?;
    Ok((out_power, out_toughness, shape.duration, shape.condition))
}

pub(crate) fn force_implicit_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenWithMods { player, .. }
                    | SubjectVerbActionAst::CreateTokenCopy { player, .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. },
                ..
            }) => {
                if matches!(*player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                force_implicit_token_controller_you(nested);
            }),
        }
    }
}

fn bind_implicit_choose_chooser(effects: &mut [EffectAst], chooser: PlayerAst) {
    for effect in effects {
        match effect {
            EffectAst::ChooseObjects { player, .. }
            | EffectAst::ChooseObjectsWithAggregateConstraint { player, .. }
            | EffectAst::ChooseObjectsBottomOfLibrary { player, .. }
            | EffectAst::ChooseObjectsTopOfLibrary { player, .. }
            | EffectAst::ChooseObjectsAcrossZones { player, .. }
            | EffectAst::ChooseTaggedObjectsInZone { player, .. }
                if matches!(*player, PlayerAst::Implicit) =>
            {
                *player = chooser;
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                bind_implicit_choose_chooser(nested, chooser);
            }),
        }
    }
}

fn tagged_predicate(filter_tokens: Option<&[OwnedLexToken]>) -> Option<PredicateAst> {
    let filter = parse_object_filter(filter_tokens?, false).ok()?;
    Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    })
}

fn parse_maybe_effects(
    tokens: &[OwnedLexToken],
    parse_inner: bool,
    scope_may_to_that_player: bool,
) -> Result<Vec<EffectAst>, CardTextError> {
    if !for_each_shapes::contains_may(tokens) {
        return if parse_inner {
            parse_effect_chain_inner(tokens)
        } else {
            parse_effect_chain(tokens)
        };
    }
    let stripped = remove_first_word(tokens);
    let scoped;
    let may_tokens = if scope_may_to_that_player {
        scoped = prepend_that_player_subject(&stripped);
        scoped.as_slice()
    } else {
        stripped.as_slice()
    };
    let effects = parse_effect_chain_inner(may_tokens)?;
    Ok(vec![EffectAst::May { effects }])
}

fn opponent_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Opponent => Some(PlayerFilter::Opponent),
        ForEachParticipantScope::OpponentExceptDefending => Some(PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::Defending,
        )),
        ForEachParticipantScope::Player
        | ForEachParticipantScope::PlayerExceptYou
        | ForEachParticipantScope::PlayerExceptItsController
        | ForEachParticipantScope::PlayerOnYourTeam => None,
    }
}

fn player_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Player => Some(PlayerFilter::Any),
        ForEachParticipantScope::PlayerExceptYou => Some(PlayerFilter::NotYou),
        ForEachParticipantScope::PlayerExceptItsController => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::ControllerOf(ObjectRef::tagged(TagKey::from(IT_TAG))),
        )),
        ForEachParticipantScope::PlayerOnYourTeam => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::Opponent,
        )),
        ForEachParticipantScope::Opponent | ForEachParticipantScope::OpponentExceptDefending => None,
    }
}

fn wrap_opponents(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Opponent {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        }
    }
}

fn wrap_players(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Any {
        EffectAst::ForEachPlayer { effects }
    } else {
        EffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        }
    }
}

pub(crate) fn parse_for_each_opponent_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Voter-relative opponent sets are already represented by an event-
    // populated player tag. Recognize that typed set before the ordinary
    // quantified-opponent path wraps it in a second loop, which would apply
    // the tagged-player action once for every opponent.
    if let Some(mut effects) =
        super::dispatch_inner::parse_vote_affinity_subject_verb(tokens)?
    {
        if effects.len() == 1 {
            return Ok(effects.pop());
        }
        return Err(CardTextError::ParseError(
            "voter-relative opponent clause produced multiple outer effects".to_string(),
        ));
    }

    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = opponent_filter(outer.scope) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let slot_chooser = if outer.participant_is_actor {
        PlayerAst::That
    } else {
        PlayerAst::You
    };
    if let Some(effects) =
        super::parse_for_each_type_slot_choice_clause(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_opponents(&iteration_filter, effects)));
    }
    if iteration_filter == PlayerFilter::Opponent
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, true)?
    {
        return Ok(Some(effect));
    }

    if let Some(special) = for_each_shapes::parse_opponent_special_shape(outer.inner_tokens)? {
        match special {
            OpponentSpecialShape::IgnoreScryOrSurveil => return Ok(None),
            OpponentSpecialShape::ChooseReturnUnlessDraw { target_tokens } => {
                let target = parse_target_phrase(target_tokens)?;
                let return_target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                return Ok(Some(wrap_opponents(
                    &iteration_filter,
                    vec![
                        EffectAst::subject_verb_target_only(target),
                        EffectAst::UnlessAction {
                            effects: vec![EffectAst::subject_verb_return_to_hand(
                                return_target,
                                false,
                            )],
                            alternative: vec![EffectAst::subject_verb(
                                SubjectVerbRoleAst::AffectedPlayer,
                                PlayerAst::You,
                                SubjectVerbActionAst::Draw {
                                    count: Value::Fixed(1),
                                },
                            )],
                            player: PlayerAst::ItsController,
                        },
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
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerHasLessLifeThanYou {
                            player: PlayerAst::That,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
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
                    vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerHasPoisonCountersOrMore {
                            player: PlayerAst::That,
                            count,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                )));
            }
        }
    }

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
                return Ok(Some(EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::Conditional {
                        predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                            player: PlayerAst::That,
                        },
                        if_true: branch_effects,
                        if_false: Vec::new(),
                    }],
                }));
            }
            WhoClauseShape::Negated {
                effect_tokens,
                tagged_filter_tokens,
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect in for each opponent who doesn't clause (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachOpponentDoesNot {
                    effects: parse_effect_chain_inner(effect_tokens)?,
                    predicate: tagged_predicate(tagged_filter_tokens),
                }));
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
                return Ok(Some(EffectAst::ForEachOpponentDid {
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
                    bind_implicit_player_context(effect, player.clone());
                }
                return Ok(Some(EffectAst::ForEachOpponentDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                }));
            }
        }
    }

    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let normalized = if outer.participant_is_actor && !participant_may && !participant_chooses {
        prepend_that_player_subject(outer.inner_tokens)
    } else {
        prepend_that_player_life_total_subject(outer.inner_tokens)
    };
    let mut effects = parse_maybe_effects(&normalized, false, outer.participant_is_actor)?;
    if participant_chooses {
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
    }
    Ok(Some(wrap_opponents(&iteration_filter, effects)))
}

pub(crate) fn parse_for_each_target_players_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = for_each_shapes::parse_for_each_target_players_shape(tokens) else {
        return Ok(None);
    };
    if shape.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after target-player each clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }
    let effects = parse_maybe_effects(shape.effect_tokens, true, false)?;
    Ok(Some(EffectAst::ForEachTargetPlayers {
        count: shape.count,
        effects,
    }))
}

pub(crate) fn parse_who_did_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    Ok(tagged_predicate(
        for_each_shapes::parse_who_tagged_filter_shape(inner_tokens),
    ))
}

pub(crate) fn parse_for_each_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = player_filter(outer.scope) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let slot_chooser = if outer.participant_is_actor {
        PlayerAst::That
    } else {
        PlayerAst::You
    };
    if let Some(effects) =
        super::parse_for_each_type_slot_choice_clause(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_players(&iteration_filter, effects)));
    }
    if iteration_filter == PlayerFilter::Any
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, false)?
    {
        return Ok(Some(effect));
    }

    if let Some(relative) = for_each_shapes::parse_relative_control_clause_shape(outer.inner_tokens)
    {
        let filter = parse_object_filter(relative.filter_tokens, false)?;
        let branch_effects = parse_maybe_effects(relative.effect_tokens, true, false)?;
        let predicate = if relative.controls_most {
            PredicateAst::PlayerControlsMost {
                player: PlayerAst::That,
                filter,
            }
        } else {
            PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            }
        };
        return Ok(Some(wrap_players(
            &iteration_filter,
            vec![EffectAst::Conditional {
                predicate,
                if_true: branch_effects,
                if_false: Vec::new(),
            }],
        )));
    }

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
            } => {
                if effect_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing effect in for each player who doesn't clause (clause: '{}')",
                        clause_text
                    )));
                }
                return Ok(Some(EffectAst::ForEachPlayerDoesNot {
                    effects: parse_effect_chain_inner(effect_tokens)?,
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
                    bind_implicit_player_context(effect, player.clone());
                }
                return Ok(Some(EffectAst::ForEachPlayerDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                }));
            }
        }
    }

    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let normalized = if outer.participant_is_actor && !participant_may && !participant_chooses {
        prepend_that_player_subject(outer.inner_tokens)
    } else {
        prepend_that_player_life_total_subject(outer.inner_tokens)
    };
    let mut effects = parse_maybe_effects(&normalized, false, outer.participant_is_actor)?;
    if participant_chooses {
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
    }
    Ok(Some(wrap_players(&iteration_filter, effects)))
}

#[cfg(test)]
mod participant_choice_ownership_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn parsed_debug(text: &str) -> String {
        let tokens = lex_line(text, 0).expect("participant choice should lex");
        let effect =
            if text.starts_with("Each opponent") || text.starts_with("For each opponent") {
                parse_for_each_opponent_clause(&tokens)
            } else {
                parse_for_each_player_clause(&tokens)
            }
            .expect("participant choice should parse")
            .expect("participant choice shape should match");
        format!("{effect:#?}")
    }

    #[test]
    fn participant_subject_owns_choice_but_for_each_imperative_does_not() {
        let each_opponent =
            parsed_debug("Each opponent chooses a creature they control and sacrifices it.");
        assert!(each_opponent.contains("player: That"), "{each_opponent}");
        assert!(!each_opponent.contains("player: You"), "{each_opponent}");

        let each_player = parsed_debug("Each player chooses a creature they control.");
        assert!(each_player.contains("player: That"), "{each_player}");

        let controller = parsed_debug("For each opponent, choose a creature they control.");
        assert!(controller.contains("player: You"), "{controller}");
    }
}
