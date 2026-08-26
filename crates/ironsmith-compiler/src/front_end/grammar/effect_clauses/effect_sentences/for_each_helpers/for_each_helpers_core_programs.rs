use super::*;

pub(super) fn wrap_opponents(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Opponent {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        }
    }
}

pub fn parse_for_each_opponent_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Voter-relative opponent sets are already represented by an event-
    // populated player tag. Recognize that typed set before the ordinary
    // quantified-opponent path wraps it in a second loop, which would apply
    // the tagged-player action once for every opponent.
    if let Some(mut effects) =
        super::super::dispatch_inner::parse_vote_affinity_subject_verb(tokens)?
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
        super::super::parse_for_each_type_slot_choice_clause(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_opponents(&iteration_filter, effects)));
    }
    if let Some(effects) =
        parse_participant_creature_type_choice_program(outer.inner_tokens, slot_chooser)?
    {
        return Ok(Some(wrap_opponents(&iteration_filter, effects)));
    }
    if iteration_filter == PlayerFilter::Opponent
        && let Some(effect) = parse_for_each_doesnt_control_lose_game(tokens, true)?
    {
        return Ok(Some(effect));
    }

    if let Some(relative) = for_each_shapes::parse_relative_control_clause_shape(outer.inner_tokens)
    {
        let conditional =
            parse_relative_control_conditional(relative, outer.participant_is_actor, &clause_text)?;
        return Ok(Some(wrap_opponents(&iteration_filter, vec![conditional])));
    }

    if let Some(effect) =
        parse_combat_damage_history_participant(outer.inner_tokens, iteration_filter.clone())?
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
                return Ok(Some(EffectAst::ForEachOpponentDoesNot {
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
                    bind_implicit_player_context(effect, player);
                }
                return Ok(Some(EffectAst::ForEachOpponentDid {
                    effects,
                    predicate: None,
                    result_predicate: IfResultPredicate::AcceptedChoice,
                }));
            }
        }
    }

    if outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("return") || token.is_word("returns"))
    {
        let return_tokens = crate::util::trim_edge_punctuation_tokens(&outer.inner_tokens[1..]);
        let mut effect = super::super::zone_handlers::parse_return(return_tokens)?;
        bind_implicit_player_context(&mut effect, PlayerAst::That);
        return Ok(Some(wrap_players(&iteration_filter, vec![effect])));
    }

    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let quantified_unless_payment = if outer.participant_is_actor
        && super::super::has_unless_payment_choice(outer.inner_tokens)?
    {
        let normalized = prepend_that_player_subject(outer.inner_tokens);
        super::super::parse_sentence_unless_pays(super::super::SubjectVerbPrimitiveClause::new(
            &normalized,
        ))?
    } else {
        None
    };
    let mut effects = if let Some(effects) = quantified_unless_payment {
        effects
    } else if outer.participant_is_actor && !participant_may {
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
        // The quantified participant is the iteration key, not the actor, in
        // imperative clauses such as "For each opponent, create a token."
        // Resolve the otherwise implicit token controller to the effect
        // controller before lowering enters iterated-player context.
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
    Ok(Some(wrap_opponents(&iteration_filter, effects)))
}
