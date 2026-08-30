use super::*;

pub(super) fn opponent_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Opponent => Some(PlayerFilter::Opponent),
        ForEachParticipantScope::OpponentExceptDefending => Some(PlayerFilter::excluding(
            PlayerFilter::Opponent,
            PlayerFilter::Defending,
        )),
        ForEachParticipantScope::Player
        | ForEachParticipantScope::PlayerExceptYou
        | ForEachParticipantScope::PlayerExceptTarget
        | ForEachParticipantScope::PlayerExceptItsController
        | ForEachParticipantScope::PlayerOnYourTeam => None,
    }
}

pub(super) fn player_filter(scope: ForEachParticipantScope) -> Option<PlayerFilter> {
    match scope {
        ForEachParticipantScope::Player => Some(PlayerFilter::Any),
        ForEachParticipantScope::PlayerExceptYou => Some(PlayerFilter::NotYou),
        ForEachParticipantScope::PlayerExceptTarget => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::target_player(),
        )),
        ForEachParticipantScope::PlayerExceptItsController => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::ControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::It.key(),
            )),
        )),
        ForEachParticipantScope::PlayerOnYourTeam => Some(PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::Opponent,
        )),
        ForEachParticipantScope::Opponent | ForEachParticipantScope::OpponentExceptDefending => {
            None
        }
    }
}

/// In `each other player may copy that spell`, "other" is relative to the
/// player who controls the referenced spell, not necessarily the ability's
/// controller. Keep ordinary `each other player` clauses controller-relative,
/// but anchor this typed stack-copy shape to the triggering stack object.
pub(super) fn reanchor_other_player_copy_filter(
    filter: PlayerFilter,
    effects: &[EffectAst],
) -> PlayerFilter {
    if filter != PlayerFilter::NotYou || !effects.iter().any(effect_copies_triggering_stack_object)
    {
        return filter;
    }
    PlayerFilter::excluding(
        PlayerFilter::Any,
        PlayerFilter::AliasedControllerOf(ObjectRef::tagged("triggering")),
    )
}

pub(super) fn wrap_players(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Any {
        EffectAst::ForEachPlayer { effects }
    } else {
        EffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        }
    }
}

pub fn parse_for_each_target_players_clause(
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
    // `target player <action> ... for each <counted set>` contains the same
    // lexical markers as `N target players <qualifier> each <action>`. The
    // shape parser intentionally keeps the qualifier open-ended, so require
    // that its proposed target slice is actually a target phrase before
    // claiming the clause. Otherwise the ordinary action family (for example,
    // discard) must receive the complete `for each` count suffix.
    let Ok(target) = parse_target_phrase(shape.target_tokens) else {
        return Ok(None);
    };
    let filter = match target {
        TargetAst::Player(filter, _) => filter,
        TargetAst::WithCount(inner, _) => match *inner {
            TargetAst::Player(filter, _) => filter,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "expected player target in target-player each clause (clause: '{}')",
                    LexedClause::new(tokens).text()
                )));
            }
        },
        _ => {
            return Err(CardTextError::ParseError(format!(
                "expected player target in target-player each clause (clause: '{}')",
                LexedClause::new(tokens).text()
            )));
        }
    };
    // The participant after `each` is the actor of the trailing instruction.
    // Supplying that subject before parsing also lets possessive dynamic
    // values such as "half their library" bind to the iterated player rather
    // than falling back to the spell's controller.
    let effects = if for_each_shapes::contains_may(shape.effect_tokens) {
        parse_maybe_effects(shape.effect_tokens, true, true)?
    } else {
        let normalized = prepend_that_player_subject(shape.effect_tokens);
        parse_maybe_effects(&normalized, true, false)?
    };
    Ok(Some(EffectAst::ForEachTargetPlayers {
        count: shape.count,
        filter,
        effects,
    }))
}

pub fn parse_for_each_player_clause(
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
        super::super::parse_for_each_type_slot_choice_clause(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_players(&iteration_filter, effects)));
    }
    if let Some(effects) = parse_participant_creature_type_choice(outer.inner_tokens, slot_chooser)?
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
        let conditional =
            parse_relative_control_conditional(relative, outer.participant_is_actor, &clause_text)?;
        return Ok(Some(wrap_players(&iteration_filter, vec![conditional])));
    }

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

    if let Some(effect) =
        parse_combat_damage_history_participant(outer.inner_tokens, iteration_filter.clone())?
    {
        return Ok(Some(effect));
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

    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let mut effects = if outer.participant_is_actor && !participant_may {
        if let Some(effects) = parse_quantified_participant_actor_program(outer.inner_tokens)? {
            effects
        } else {
            let normalized = prepend_that_player_subject(outer.inner_tokens);
            parse_maybe_effects(&normalized, true, true)?
        }
    } else {
        let normalized = prepend_that_player_life_total_subject(outer.inner_tokens);
        parse_maybe_effects(&normalized, true, outer.participant_is_actor)?
    };
    if !outer.participant_is_actor {
        force_implicit_token_controller_you(&mut effects);
    }
    if participant_chooses {
        if outer.participant_is_actor
            && !outer.inner_tokens.iter().any(|token| token.is_word("you"))
        {
            bind_quantified_participant_actor(&mut effects);
        }
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
        stabilize_standalone_participant_choice_tag(&mut effects, outer.inner_tokens);
    }
    let iteration_filter = reanchor_other_player_copy_filter(iteration_filter, &effects);
    Ok(Some(wrap_players(&iteration_filter, effects)))
}
