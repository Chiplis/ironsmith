use super::*;

fn tag_last_discard_in_effects(effects: &mut [EffectAst], tag: &TagKey) -> bool {
    for effect in effects.iter_mut().rev() {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::Discard {
                tag: discard_tag, ..
            } = &mut subject_verb.action
        {
            *discard_tag = Some(tag.clone());
            return true;
        }
    }
    false
}

fn bind_explicit_tag_to_player_tagged_predicate(
    predicate: &PredicateAst,
    tag: &TagKey,
) -> PredicateAst {
    let mut bound = predicate.clone();
    if let PredicateAst::PlayerTaggedObjectMatches {
        tag: predicate_tag, ..
    } = &mut bound
        && predicate_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    {
        *predicate_tag = tag.clone();
    }
    bound
}

pub fn compile_if_do_with_opponent_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDoesNot {
        effects: second_effects,
        predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let explicit_tag = ctx.next_tag("discarded");
            let mut tagged_opponent_effects = opponent_effects.clone();
            if !tag_last_discard_in_effects(&mut tagged_opponent_effects, &explicit_tag) {
                return Err(CardTextError::ParseError(
                    "missing discard antecedent for tagged opponent follow-up".to_string(),
                ));
            }
            let first_ast = EffectAst::ForEachOpponent {
                effects: tagged_opponent_effects,
            };
            let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: bind_explicit_tag_to_player_tagged_predicate(
                        predicate,
                        &explicit_tag,
                    ),
                    if_true: Vec::new(),
                    if_false: second_effects.clone(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }
    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let explicit_tag = ctx.next_tag("discarded");
            let mut tagged_player_effects = player_effects.clone();
            if !tag_last_discard_in_effects(&mut tagged_player_effects, &explicit_tag) {
                return Err(CardTextError::ParseError(
                    "missing discard antecedent for tagged player follow-up".to_string(),
                ));
            }
            let first_ast = EffectAst::ForEachPlayer {
                effects: tagged_player_effects,
            };
            let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: bind_explicit_tag_to_player_tagged_predicate(
                        predicate,
                        &explicit_tag,
                    ),
                    if_true: Vec::new(),
                    if_false: second_effects.clone(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let first_ast = EffectAst::ForEachPlayer {
            effects: player_effects.clone(),
        };
        let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
        let id = if let Some(last) = first_effects.pop() {
            let id = ctx.next_effect_id();
            first_effects.push(Effect::with_id(id.0, last));
            id
        } else {
            return Err(CardTextError::ParseError(
                "missing per-player antecedent effect for if-you-don't follow-up".to_string(),
            ));
        };

        let (inner_effects, inner_choices) =
            compile_effects_in_iterated_player_context(second_effects, ctx, None)?;
        for choice in inner_choices {
            push_choice(&mut choices, choice);
        }
        let conditional = Effect::if_then(id, EffectPredicate::DidNotHappen, inner_effects);
        first_effects.push(Effect::for_each_opponent(vec![conditional]));
        return Ok(Some((first_effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let explicit_tag = ctx.next_tag("discarded");
        let mut tagged_first_effects = first_effects.clone();
        let Some(EffectAst::ForEachOpponent {
            effects: tagged_opponent_effects,
        }) = tagged_first_effects.first_mut()
        else {
            return Ok(None);
        };
        if !tag_last_discard_in_effects(tagged_opponent_effects, &explicit_tag) {
            return Err(CardTextError::ParseError(
                "missing discard antecedent for tagged opponent follow-up".to_string(),
            ));
        }
        let tagged_first = if let Some(condition) = condition {
            EffectAst::ResolvedIfResult {
                condition,
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        } else {
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        };
        let (mut first_compiled, mut choices) = compile_effect(&tagged_first, ctx)?;
        let followup = EffectAst::ForEachOpponent {
            effects: vec![EffectAst::Conditional {
                predicate: bind_explicit_tag_to_player_tagged_predicate(predicate, &explicit_tag),
                if_true: Vec::new(),
                if_false: second_effects.clone(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachOpponent {
        effects: merged_opponent_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

pub fn compile_if_do_with_player_doesnt(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayerDoesNot {
        effects: second_effects,
        predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let explicit_tag = ctx.next_tag("discarded");
            let mut tagged_player_effects = player_effects.clone();
            if !tag_last_discard_in_effects(&mut tagged_player_effects, &explicit_tag) {
                return Err(CardTextError::ParseError(
                    "missing discard antecedent for tagged player follow-up".to_string(),
                ));
            }
            let first_ast = EffectAst::ForEachPlayer {
                effects: tagged_player_effects,
            };
            let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
            let followup = EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: bind_explicit_tag_to_player_tagged_predicate(
                        predicate,
                        &explicit_tag,
                    ),
                    if_true: Vec::new(),
                    if_false: second_effects.clone(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let explicit_tag = ctx.next_tag("discarded");
        let mut tagged_first_effects = first_effects.clone();
        let Some(EffectAst::ForEachPlayer {
            effects: tagged_player_effects,
        }) = tagged_first_effects.first_mut()
        else {
            return Ok(None);
        };
        if !tag_last_discard_in_effects(tagged_player_effects, &explicit_tag) {
            return Err(CardTextError::ParseError(
                "missing discard antecedent for tagged player follow-up".to_string(),
            ));
        }
        let tagged_first = if let Some(condition) = condition {
            EffectAst::ResolvedIfResult {
                condition,
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        } else {
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: tagged_first_effects,
            }
        };
        let (mut first_compiled, mut choices) = compile_effect(&tagged_first, ctx)?;
        let followup = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::Conditional {
                predicate: bind_explicit_tag_to_player_tagged_predicate(predicate, &explicit_tag),
                if_true: Vec::new(),
                if_false: second_effects.clone(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    merged_player_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachPlayer {
        effects: merged_player_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

fn correlated_choice_result_predicate(
    requested: IfResultPredicate,
    antecedent_effects: &[EffectAst],
) -> IfResultPredicate {
    if requested == IfResultPredicate::AcceptedChoice
        && antecedent_effects.last().is_some_and(|effect| {
            matches!(
                effect,
                EffectAst::May { .. } | EffectAst::MayByPlayer { .. }
            )
        })
    {
        IfResultPredicate::AcceptedChoice
    } else if requested == IfResultPredicate::DidNot {
        IfResultPredicate::DidNot
    } else {
        IfResultPredicate::Did
    }
}

pub fn compile_if_do_with_opponent_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDid {
        effects: second_effects,
        predicate,
        result_predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachOpponent {
        effects: opponent_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: predicate.clone(),
                    if_true: second_effects.clone(),
                    if_false: Vec::new(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let result_predicate =
            correlated_choice_result_predicate(result_predicate.clone(), opponent_effects);
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: result_predicate,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachOpponent {
            effects: merged_opponent_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }
    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        if let Some(predicate) = predicate {
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let followup = EffectAst::ForEachOpponent {
                effects: vec![EffectAst::Conditional {
                    predicate: predicate.clone(),
                    if_true: second_effects.clone(),
                    if_false: Vec::new(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let first_ast = EffectAst::ForEachPlayer {
            effects: player_effects.clone(),
        };
        let (mut first_effects, mut choices) = compile_effect(&first_ast, ctx)?;
        let id = if let Some(last) = first_effects.pop() {
            let id = ctx.next_effect_id();
            first_effects.push(Effect::with_id(id.0, last));
            id
        } else {
            return Err(CardTextError::ParseError(
                "missing per-player antecedent effect for if-you-do follow-up".to_string(),
            ));
        };

        let (inner_effects, inner_choices) =
            compile_effects_in_iterated_player_context(second_effects, ctx, None)?;
        for choice in inner_choices {
            push_choice(&mut choices, choice);
        }
        let predicate =
            correlated_choice_result_predicate(result_predicate.clone(), player_effects);
        let conditional = Effect::if_then(
            id,
            effect_predicate_from_if_result(predicate),
            inner_effects,
        );
        first_effects.push(Effect::for_each_opponent(vec![conditional]));
        return Ok(Some((first_effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    if let Some(predicate) = predicate {
        let (mut first_compiled, mut choices) = compile_effect(first, ctx)?;
        let followup = EffectAst::ForEachOpponent {
            effects: vec![EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: second_effects.clone(),
                if_false: Vec::new(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    let Some(EffectAst::ForEachOpponent {
        effects: opponent_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_opponent_effects = opponent_effects.clone();
    let result_predicate =
        correlated_choice_result_predicate(result_predicate.clone(), opponent_effects);
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: result_predicate,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachOpponent {
        effects: merged_opponent_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

pub fn compile_if_do_with_player_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayerDid {
        effects: second_effects,
        predicate,
        result_predicate,
    } = second
    else {
        return Ok(None);
    };

    if let EffectAst::ForEachPlayer {
        effects: player_effects,
    } = first
    {
        let is_face_only_coin_flip = matches!(
            player_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::FlipCoinFaceOnly,
                ..
            })]
        );
        if is_face_only_coin_flip && predicate.is_none() {
            // Resolve every player's physical flip first and retain the
            // ForPlayers outcome's per-player heads/tails counts. The outer
            // IfEffect then applies the separately authored follow-up only to
            // matching players instead of interleaving one player's
            // consequence before the next player's flip.
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let Some(first_effect) = first_effects.pop() else {
                return Err(CardTextError::ParseError(
                    "missing each-player coin flip antecedent".to_string(),
                ));
            };
            let id = ctx.next_effect_id();
            first_effects.push(Effect::with_id(id.0, first_effect));

            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(second_effects, ctx, None)?;
            for choice in inner_choices {
                push_choice(&mut choices, choice);
            }
            first_effects.push(Effect::new(
                crate::effects::IfEffect::if_then(
                    id,
                    effect_predicate_from_if_result(result_predicate.clone()),
                    inner_effects,
                )
                .with_per_player_result(true),
            ));
            return Ok(Some((first_effects, choices)));
        }

        if let Some(predicate) = predicate {
            let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
            let followup = EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: predicate.clone(),
                    if_true: second_effects.clone(),
                    if_false: Vec::new(),
                }],
            };
            let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
            first_effects.extend(second_compiled);
            for choice in second_choices {
                push_choice(&mut choices, choice);
            }
            return Ok(Some((first_effects, choices)));
        }
        let result_predicate =
            correlated_choice_result_predicate(result_predicate.clone(), player_effects);
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: result_predicate,
            effects: second_effects.clone(),
        });

        let merged = EffectAst::ForEachPlayer {
            effects: merged_player_effects,
        };
        let (effects, choices) = compile_effect(&merged, ctx)?;
        return Ok(Some((effects, choices)));
    }

    if let Some(predicate) = predicate {
        let (mut first_compiled, mut choices) = compile_effect(first, ctx)?;
        let followup = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: second_effects.clone(),
                if_false: Vec::new(),
            }],
        };
        let (second_compiled, second_choices) = compile_effect(&followup, ctx)?;
        first_compiled.extend(second_compiled);
        for choice in second_choices {
            push_choice(&mut choices, choice);
        }
        return Ok(Some((first_compiled, choices)));
    }

    if !matches!(
        first,
        EffectAst::IfResult { .. } | EffectAst::ResolvedIfResult { .. }
    ) {
        let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
        let id = if let Some(last) = first_effects.pop() {
            let id = ctx.next_effect_id();
            first_effects.push(Effect::with_id(id.0, last));
            id
        } else {
            return Err(CardTextError::ParseError(
                "missing antecedent effect for player who did follow-up".to_string(),
            ));
        };

        let (inner_effects, inner_choices) =
            compile_effects_in_iterated_player_context(second_effects, ctx, None)?;
        for choice in inner_choices {
            push_choice(&mut choices, choice);
        }
        first_effects.push(Effect::if_then(
            id,
            effect_predicate_from_if_result(correlated_choice_result_predicate(
                result_predicate.clone(),
                std::slice::from_ref(first),
            )),
            inner_effects,
        ));
        return Ok(Some((first_effects, choices)));
    }

    let (condition, first_effects) = match first {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        } => (None, effects),
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects,
        } => (Some(*condition), effects),
        _ => return Ok(None),
    };

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    let result_predicate =
        correlated_choice_result_predicate(result_predicate.clone(), player_effects);
    merged_player_effects.push(EffectAst::IfResult {
        predicate: result_predicate,
        effects: second_effects.clone(),
    });

    let merged_effects = vec![EffectAst::ForEachPlayer {
        effects: merged_player_effects,
    }];
    let merged = if let Some(condition) = condition {
        EffectAst::ResolvedIfResult {
            condition,
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    } else {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: merged_effects,
        }
    };

    let (effects, choices) = compile_effect(&merged, ctx)?;
    Ok(Some((effects, choices)))
}

fn effect_contains_deal_damage(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .is_some()
    {
        return true;
    }

    let mut contains_damage = false;
    effect.visit_child_effects(&mut |child| {
        if !contains_damage && effect_contains_deal_damage(child) {
            contains_damage = true;
        }
    });
    contains_damage
}

fn runtime_effect_is_terminal_result_producer(
    effect: &Effect,
    producer: TerminalResultProducer,
) -> bool {
    match producer {
        TerminalResultProducer::Clash => effect
            .downcast_ref::<crate::effects::ClashEffect>()
            .is_some(),
        TerminalResultProducer::FlipCoin => effect
            .downcast_ref::<crate::effects::FlipCoinEffect>()
            .is_some(),
    }
}

fn wrap_terminal_runtime_result_producer(
    effect: &Effect,
    producer: TerminalResultProducer,
    id: EffectId,
) -> Option<Effect> {
    if runtime_effect_is_terminal_result_producer(effect, producer) {
        return Some(Effect::with_id(id.0, effect.clone()));
    }

    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let mut sequence = sequence.clone();
    let terminal = sequence.effects.last()?.clone();
    *sequence.effects.last_mut()? = wrap_terminal_runtime_result_producer(&terminal, producer, id)?;
    Some(Effect::new(sequence))
}

fn runtime_result_producer_count(effect: &Effect, producer: TerminalResultProducer) -> usize {
    if runtime_effect_is_terminal_result_producer(effect, producer) {
        return 1;
    }
    effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .map_or(0, |sequence| {
            sequence
                .effects
                .iter()
                .map(|effect| runtime_result_producer_count(effect, producer))
                .sum()
        })
}

fn wrap_unique_runtime_result_producer(
    effect: &Effect,
    producer: TerminalResultProducer,
    id: EffectId,
) -> Option<Effect> {
    if runtime_effect_is_terminal_result_producer(effect, producer) {
        return Some(Effect::with_id(id.0, effect.clone()));
    }
    let mut sequence = effect
        .downcast_ref::<crate::effects::SequenceEffect>()?
        .clone();
    if runtime_result_producer_count(effect, producer) != 1 {
        return None;
    }
    let index = crate::slice_primitives::select_position(&sequence.effects, |effect| {
        runtime_result_producer_count(effect, producer) == 1
    })?;
    sequence.effects[index] =
        wrap_unique_runtime_result_producer(&sequence.effects[index], producer, id)?;
    Some(Effect::new(sequence))
}

pub(super) fn try_assign_effect_result_id_for_unique_producer(
    effects: &mut [Effect],
    producer: TerminalResultProducer,
    id: EffectId,
) -> bool {
    let total = effects
        .iter()
        .map(|effect| runtime_result_producer_count(effect, producer))
        .sum::<usize>();
    if total != 1 {
        return false;
    }
    let index = crate::slice_primitives::select_position(effects, |effect| {
        runtime_result_producer_count(effect, producer) == 1
    })
    .expect("one result producer was counted");
    let Some(wrapped) = wrap_unique_runtime_result_producer(&effects[index], producer, id) else {
        return false;
    };
    effects[index] = wrapped;
    true
}

pub(super) fn assign_effect_result_id_for_ast(
    effects: &mut Vec<Effect>,
    ast: &EffectAst,
    id: EffectId,
    error_message: &str,
) -> Result<(), CardTextError> {
    let Some(producer) = terminal_result_producer(ast) else {
        // An authored coordination can put another instruction after the
        // value-producing action while a later branch still refers to that
        // action's typed result (for example, "clash, then return ... If you
        // win ..."). An assigned result ID must observe the unique producer,
        // not the presentation sequence that contains it.
        if try_assign_effect_result_id_for_unique_producer(
            effects,
            TerminalResultProducer::Clash,
            id,
        ) || try_assign_effect_result_id_for_unique_producer(
            effects,
            TerminalResultProducer::FlipCoin,
            id,
        ) {
            return Ok(());
        }
        return assign_effect_result_id(effects, id, error_message);
    };
    let Some(terminal) = effects.last().cloned() else {
        return Err(CardTextError::InvariantViolation(error_message.to_string()));
    };
    let wrapped =
        wrap_terminal_runtime_result_producer(&terminal, producer, id).ok_or_else(|| {
            CardTextError::InvariantViolation(format!(
                "{error_message}: terminal {producer:?} producer was not present in lowered effects"
            ))
        })?;
    *effects
        .last_mut()
        .expect("nonempty lowered effects checked above") = wrapped;
    Ok(())
}

pub fn compile_result_followup(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let (predicate, followup_effects, reflexive) = match second {
        EffectAst::IfResult { predicate, effects } => (predicate.clone(), effects, false),
        EffectAst::WhenResult { predicate, effects } => (predicate.clone(), effects, true),
        _ => return Ok(None),
    };
    if matches!(
        first,
        EffectAst::IfResult { .. }
            | EffectAst::WhenResult { .. }
            | EffectAst::ResolvedIfResult { .. }
            | EffectAst::ResolvedWhenResult { .. }
    ) {
        return Ok(None);
    }

    let (mut first_effects, mut choices) = compile_effect(first, ctx)?;
    if first_effects.is_empty() {
        return Err(CardTextError::ParseError(
            "missing antecedent effect for result follow-up".to_string(),
        ));
    }
    let id = ctx.next_effect_id();
    if predicate == IfResultPredicate::DealtDamageToPlayer {
        let damage_idx = crate::slice_primitives::select_last_position(&first_effects, |effect| {
            effect_contains_deal_damage(effect)
        })
        .ok_or_else(|| {
            CardTextError::ParseError(
                "damage-to-player result is missing a damage antecedent".to_string(),
            )
        })?;
        let mut antecedent_effects = first_effects.split_off(damage_idx);
        let antecedent = if antecedent_effects.len() == 1 {
            antecedent_effects.remove(0)
        } else {
            Effect::new(crate::effects::SequenceEffect::new(antecedent_effects))
        };
        first_effects.push(Effect::with_id(id.0, antecedent));
    } else if predicate == IfResultPredicate::WonClash {
        if !try_assign_effect_result_id_for_unique_producer(
            &mut first_effects,
            TerminalResultProducer::Clash,
            id,
        ) {
            return Err(CardTextError::InvariantViolation(
                "clash-win follow-up is missing its unique clash antecedent".to_string(),
            ));
        }
    } else {
        assign_effect_result_id_for_ast(
            &mut first_effects,
            first,
            id,
            "result follow-up is missing its antecedent effect",
        )?;
    }

    let (inner_effects, inner_choices) = with_preserved_lowering_context(
        ctx,
        |ctx| {
            ctx.last_effect_id = Some(id);
        },
        |ctx| compile_effects(followup_effects, ctx),
    )?;
    let predicate = effect_predicate_from_if_result(predicate);
    if reflexive {
        first_effects.push(Effect::reflexive_trigger(
            id,
            predicate,
            inner_effects,
            inner_choices,
        ));
    } else {
        first_effects.push(Effect::if_then(id, predicate, inner_effects));
        for choice in inner_choices {
            push_choice(&mut choices, choice);
        }
    }

    Ok(Some((first_effects, choices)))
}

#[derive(Debug, Clone)]
struct EffectLoweringContextState {
    frame: LoweringFrame,
}

impl EffectLoweringContextState {
    fn capture(ctx: &EffectLoweringContext) -> Self {
        Self {
            frame: ctx.lowering_frame(),
        }
    }

    fn restore(self, ctx: &mut EffectLoweringContext) {
        ctx.apply_lowering_frame(self.frame);
    }
}

pub fn with_preserved_lowering_context<T, Configure, Run>(
    ctx: &mut EffectLoweringContext,
    configure: Configure,
    run: Run,
) -> Result<T, CardTextError>
where
    Configure: FnOnce(&mut EffectLoweringContext),
    Run: FnOnce(&mut EffectLoweringContext) -> Result<T, CardTextError>,
{
    let saved = EffectLoweringContextState::capture(ctx);
    configure(ctx);
    let result = run(ctx);
    saved.restore(ctx);
    result
}

pub fn compile_effects_preserving_last_effect(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, mut frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, saved_frame.clone())?;
    frame_out.last_effect_id = saved_frame.last_effect_id;
    ctx.apply_id_gen_context(id_gen);
    ctx.apply_lowering_frame(frame_out);
    Ok((compiled, choices))
}

pub fn effect_predicate_from_if_result(predicate: IfResultPredicate) -> EffectPredicate {
    match predicate {
        IfResultPredicate::Did => EffectPredicate::Happened,
        IfResultPredicate::WonClash => {
            EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
        }
        IfResultPredicate::AcceptedChoice => EffectPredicate::Chosen,
        IfResultPredicate::DidNot
        | IfResultPredicate::ExplicitDidNot
        | IfResultPredicate::Otherwise => EffectPredicate::DidNotHappen,
        IfResultPredicate::SearchedLibrary => EffectPredicate::SearchedLibrary,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
        IfResultPredicate::ExcessDamageDealt => EffectPredicate::ExcessDamageDealt,
        IfResultPredicate::DealtDamageToPlayer => EffectPredicate::DealtDamageToPlayer,
        IfResultPredicate::AffectedObjectMatchesCardType { card_type, negated } => {
            EffectPredicate::AffectedObjectMatchesCardType { card_type, negated }
        }
        IfResultPredicate::PriorEffectResult(surface)
            if surface.action == ironsmith_core::PriorEffectAction::Searched
                && surface.actor == ironsmith_core::PriorEffectResultActor::You
                && surface.quantifier
                    == ironsmith_core::PriorEffectResultQuantifier::ActionOnly
                && surface.filter == ObjectFilter::default()
                && surface.required_count.is_none()
                && surface.shared_characteristic.is_none() =>
        {
            // The dedicated predicate evaluates the chosen object's origin
            // zone, which is the runtime distinction needed by
            // "If you searched your library this way" after a library and/or
            // graveyard search.
            EffectPredicate::SearchedLibrary
        }
        IfResultPredicate::PriorEffectResult(surface) => {
            EffectPredicate::PriorEffectResult(surface)
        }
        IfResultPredicate::WasDeclined => EffectPredicate::WasDeclined,
        IfResultPredicate::Value(cmp) => EffectPredicate::Value(cmp),
    }
}

pub fn compile_repeat_process_body(
    effects: &[EffectAst],
    continue_effect_index: usize,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, EffectId), CardTextError> {
    fn defines_effect_result_id(effect: &Effect, id: EffectId) -> bool {
        if effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .is_some_and(|with_id| with_id.id == id)
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            if !found && defines_effect_result_id(child, id) {
                found = true;
            }
        });
        found
    }

    // When the continuation gate is the final AST effect, lower the body as
    // one annotated sequence before attaching the repeat-result ID. This
    // preserves ordinary result links inside the process (for example, a coin
    // flip followed by its win and loss branches) while still letting the
    // final branch outcome decide whether another iteration begins.
    if continue_effect_index == effects.len().saturating_sub(1)
        && effects
            .get(continue_effect_index)
            .and_then(starting_with_controller_each_player_effects)
            .is_none()
    {
        let (mut compiled, choices) = compile_effects(effects, ctx)?;
        if compiled.is_empty() {
            return Err(CardTextError::ParseError(
                "repeat process condition compiled to no effects".to_string(),
            ));
        }
        // Sequence annotation may assign result IDs before lowering consumes
        // the context's generator (notably for multiple branches referring to
        // one coin flip). Reserve past every ID already materialized in this
        // body so the continuation wrapper cannot overwrite an antecedent.
        let condition = loop {
            let candidate = ctx.next_effect_id();
            let already_used = compiled
                .iter()
                .any(|effect| defines_effect_result_id(effect, candidate));
            if !already_used {
                break candidate;
            }
        };
        assign_effect_result_id_for_ast(
            &mut compiled,
            &effects[continue_effect_index],
            condition,
            "repeat process condition is missing a final effect",
        )?;
        ctx.last_effect_id = Some(condition);
        return Ok((compiled, choices, condition));
    }

    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut condition: Option<EffectId> = None;

    for (idx, effect) in effects.iter().enumerate() {
        let (mut effect_list, effect_choices) = if idx == continue_effect_index {
            if let Some(compiled) =
                compile_starting_with_controller_each_player_process(effect, ctx)?
            {
                compiled
            } else {
                compile_effect(effect, ctx)?
            }
        } else {
            compile_effect(effect, ctx)?
        };
        if idx == continue_effect_index {
            if effect_list.is_empty() {
                return Err(CardTextError::ParseError(
                    "repeat process condition compiled to no effects".to_string(),
                ));
            }
            let id = ctx.next_effect_id();
            assign_effect_result_id_for_ast(
                &mut effect_list,
                effect,
                id,
                "repeat process condition is missing a final effect",
            )?;
            ctx.last_effect_id = Some(id);
            condition = Some(id);
        }
        compiled.extend(effect_list);
        for choice in effect_choices {
            push_choice(&mut choices, choice);
        }
    }

    let condition = condition.ok_or_else(|| {
        CardTextError::ParseError("repeat process is missing a condition effect".to_string())
    })?;
    Ok((compiled, choices, condition))
}

fn starting_with_controller_each_player_effects(effect: &EffectAst) -> Option<&[EffectAst]> {
    let EffectAst::SourceSentence {
        effects,
        starting_with_controller: true,
        ..
    } = effect
    else {
        return None;
    };
    let [EffectAst::ForEachPlayer { effects }] = effects.as_slice() else {
        return None;
    };
    Some(effects)
}

fn compile_starting_with_controller_each_player_process(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let Some(effects) = starting_with_controller_each_player_effects(effect) else {
        return Ok(None);
    };

    let (inner_effects, inner_choices) =
        compile_effects_in_iterated_player_context(effects, ctx, None)?;
    Ok(Some((
        vec![Effect::for_players_starting_with_controller(
            PlayerFilter::Any,
            inner_effects,
        )],
        inner_choices,
    )))
}

pub fn compile_effects_in_iterated_player_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
    tagged_object: Option<TagKey>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    if !effects
        .iter()
        .any(effect_references_typed_removed_counter_metric)
    {
        iterated_frame.last_effect_id = None;
    }
    if tagged_object.is_some() {
        // A tagged-object loop establishes `__it__`, but it does not replace
        // an outer player antecedent with an artificial iterated player.
        iterated_frame.last_object_tag = Some(crate::tag::CompilerReferenceTag::It.bind());
        iterated_frame.last_it_choice_is_set = false;
        iterated_frame.iterated_object = true;
    } else {
        iterated_frame.iterated_player = true;
        iterated_frame.last_player_filter = Some(PlayerFilter::IteratedPlayer);
    }

    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, iterated_frame)?;
    let choices = choices
        .into_iter()
        .filter(|choice| !choose_spec_mentions_iterated_player(choice))
        .collect();
    ctx.apply_id_gen_context(id_gen);
    let produced_last_tag = if tagged_object.is_none() {
        frame_out.last_object_tag.clone()
    } else {
        None
    };
    ctx.apply_lowering_frame(saved_frame);
    if let Some(tag) = produced_last_tag {
        ctx.last_object_tag = Some(tag);
    }
    Ok((compiled, choices))
}

pub fn compile_effects_in_iterated_object_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    // Iterating objects establishes `__it__`, not an iterated player. Preserve
    // an outer player iteration when one exists; otherwise contextual
    // `that player` filters continue to resolve to the saved antecedent.
    if !effects
        .iter()
        .any(effect_references_typed_removed_counter_metric)
    {
        iterated_frame.last_effect_id = None;
    }
    iterated_frame.last_object_tag = Some(crate::tag::CompilerReferenceTag::It.bind());
    iterated_frame.last_it_choice_is_set = false;
    iterated_frame.iterated_object = true;

    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, _frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, iterated_frame)?;
    let choices = choices
        .into_iter()
        .filter(|choice| !choose_spec_mentions_iterated_player(choice))
        .collect();
    ctx.apply_id_gen_context(id_gen);
    ctx.apply_lowering_frame(saved_frame);
    Ok((compiled, choices))
}

pub fn force_implicit_vote_token_controller_you(effects: &mut [EffectAst]) {
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
                force_implicit_vote_token_controller_you(nested);
            }),
        }
    }
}

fn is_vote_related_predicate(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::VoteOptionGetsMoreVotes { .. }
            | PredicateAst::VoteOptionGetsMoreVotesOrTied { .. }
            | PredicateAst::NoVoteObjectsMatched { .. }
    )
}

fn is_secret_choice_related_predicate(predicate: &PredicateAst) -> bool {
    matches!(predicate, PredicateAst::SecretChoicesMatch)
}

fn compiled_vote_option_effect_uses_iterated_player(
    effect: &Effect,
    iterated_player_bound: bool,
) -> bool {
    let nested_uses_iterated_player = |effects: &[Effect], bound| {
        effects
            .iter()
            .any(|effect| compiled_vote_option_effect_uses_iterated_player(effect, bound))
    };

    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return nested_uses_iterated_player(&sequence.effects, iterated_player_bound);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<Effect>>() {
        return (!iterated_player_bound
            && may
                .decider
                .as_ref()
                .is_some_and(PlayerFilter::mentions_iterated_player))
            || nested_uses_iterated_player(&may.effects, iterated_player_bound);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>() {
        return (!iterated_player_bound && unless_pays.player.mentions_iterated_player())
            || nested_uses_iterated_player(&unless_pays.effects, iterated_player_bound);
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect<Effect>>()
    {
        return (!iterated_player_bound && unless_action.player.mentions_iterated_player())
            || nested_uses_iterated_player(&unless_action.effects, iterated_player_bound)
            || nested_uses_iterated_player(&unless_action.alternative, iterated_player_bound);
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
        return (!iterated_player_bound && value_mentions_iterated_player(&repeat.count))
            || nested_uses_iterated_player(&repeat.effects, iterated_player_bound);
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatProcessEffect>() {
        return nested_uses_iterated_player(&repeat.effects, iterated_player_bound);
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect<Effect>>() {
        return (!iterated_player_bound && for_players.filter.mentions_iterated_player())
            || nested_uses_iterated_player(&for_players.effects, true);
    }
    if let Some(for_each_object) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return (!iterated_player_bound
            && object_filter_mentions_iterated_player(&for_each_object.filter))
            || nested_uses_iterated_player(&for_each_object.effects, iterated_player_bound);
    }
    if let Some(for_each_tagged) =
        effect.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>()
    {
        return nested_uses_iterated_player(&for_each_tagged.effects, iterated_player_bound);
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect<Effect>>()
    {
        return nested_uses_iterated_player(&for_each_controller.effects, true);
    }
    if let Some(for_each_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect<Effect>>()
    {
        return nested_uses_iterated_player(&for_each_player.effects, true);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return (!iterated_player_bound
            && condition_mentions_iterated_player(&conditional.condition))
            || nested_uses_iterated_player(&conditional.if_true, iterated_player_bound)
            || nested_uses_iterated_player(&conditional.if_false, iterated_player_bound);
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return nested_uses_iterated_player(&if_effect.then, true)
            || nested_uses_iterated_player(&if_effect.else_, true);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return compiled_vote_option_effect_uses_iterated_player(
            &tagged.effect,
            iterated_player_bound,
        );
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return compiled_vote_option_effect_uses_iterated_player(
            &with_id.effect,
            iterated_player_bound,
        );
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        return choose_mode
            .modes
            .iter()
            .any(|mode| nested_uses_iterated_player(&mode.effects, iterated_player_bound));
    }

    !iterated_player_bound && effect_mentions_iterated_player(effect)
}

fn compiled_vote_option_uses_iterated_player(effects: &[Effect], choices: &[ChooseSpec]) -> bool {
    effects
        .iter()
        .any(|effect| compiled_vote_option_effect_uses_iterated_player(effect, false))
        || choices.iter().any(choose_spec_mentions_iterated_player)
}

fn vote_option_ast_uses_iterated_player_in_scope(
    effects: &[EffectAst],
    iterated_player_bound: bool,
) -> bool {
    let mut found = false;
    for effect in effects {
        if !iterated_player_bound
            && let EffectAst::ChooseObjects { filter, player, .. }
            | EffectAst::ChooseObjectsWithAggregateConstraint { filter, player, .. }
            | EffectAst::ChooseObjectsBottomOfLibrary { filter, player, .. }
            | EffectAst::ChooseObjectsTopOfLibrary { filter, player, .. }
            | EffectAst::ChooseTaggedObjectsInZone { filter, player, .. }
            | EffectAst::ChooseObjectsAcrossZones { filter, player, .. } = effect
            && (matches!(*player, PlayerAst::That)
                || object_filter_mentions_iterated_player(filter))
        {
            return true;
        }
        let nested_player_bound = iterated_player_bound
            || matches!(
                effect,
                EffectAst::ForEachOpponent { .. }
                    | EffectAst::ForEachPlayersFiltered { .. }
                    | EffectAst::ForEachPlayer { .. }
                    | EffectAst::ForEachTargetPlayers { .. }
                    | EffectAst::ForEachOpponentDoesNot { .. }
                    | EffectAst::ForEachPlayerDoesNot { .. }
                    | EffectAst::ForEachOpponentDid { .. }
                    | EffectAst::ForEachPlayerDid { .. }
                    | EffectAst::ForEachTaggedPlayer { .. }
                    | EffectAst::ForEachTagged { .. }
                    | EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { .. }
                    | EffectAst::AnyPlayerMay { .. }
            );
        for_each_nested_effects(effect, true, |nested| {
            if !found && vote_option_ast_uses_iterated_player_in_scope(nested, nested_player_bound)
            {
                found = true;
            }
        });
        if found {
            return true;
        }
    }
    false
}

fn vote_option_ast_uses_iterated_player(effects: &[EffectAst]) -> bool {
    vote_option_ast_uses_iterated_player_in_scope(effects, false)
}

fn vote_extra_amount(effect: &EffectAst) -> Option<(u32, bool)> {
    match effect {
        EffectAst::VoteExtra { count, optional } => Some((*count, *optional)),
        EffectAst::May { effects } => match effects.as_slice() {
            [EffectAst::VoteExtra { count, .. }] => Some((*count, true)),
            _ => None,
        },
        EffectAst::MayByPlayer {
            player: PlayerAst::You | PlayerAst::Implicit,
            effects,
        } => match effects.as_slice() {
            [EffectAst::VoteExtra { count, .. }] => Some((*count, true)),
            _ => None,
        },
        _ => None,
    }
}

/// `compile_annotated_effects_with_context` normally installs an annotated
/// result ID after lowering one effect. Vote and secret-choice sequences lower
/// their members manually, so they must perform the same final-effect wrapping
/// before a later result predicate tries to read that outcome.
fn preserve_annotated_effect_result_id(
    annotated: &AnnotatedEffect,
    compiled: &mut Vec<Effect>,
) -> Result<(), CardTextError> {
    let Some(id) = annotated.assigned_effect_id else {
        return Ok(());
    };
    if compiled.is_empty() {
        return Ok(());
    }
    assign_effect_result_id_for_ast(
        compiled,
        &annotated.effect,
        id,
        "missing final effect while assigning event id (vote sequence)",
    )
}

pub fn compile_vote_sequence(
    effects: &[AnnotatedEffect],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(first) = effects.first() else {
        return Ok(None);
    };
    if let EffectAst::SecretChoiceStart {
        options,
        participants,
        object_choice,
    } = &first.effect
    {
        let consumed = effects
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(idx, annotated)| match &annotated.effect {
                EffectAst::Conditional { predicate, .. }
                    if is_secret_choice_related_predicate(predicate) =>
                {
                    Some(idx + 1)
                }
                _ => None,
            })
            .next_back()
            .unwrap_or(1);

        let secret_choice = if let Some(object_choice) = object_choice {
            crate::effects::SecretChoiceEffect::new_objects(
                participants.clone(),
                object_choice.clone(),
            )
        } else {
            crate::effects::SecretChoiceEffect::new(options.clone(), participants.clone())
        };
        let mut compiled = vec![Effect::new(secret_choice)];
        preserve_annotated_effect_result_id(first, &mut compiled)?;
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            let (mut followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
            preserve_annotated_effect_result_id(annotated, &mut followups)?;
            compiled.extend(followups);
            for choice in followup_choices {
                push_choice(&mut choices, choice);
            }
            apply_local_reference_env(ctx, &annotated.out_env);
        }
        return Ok(Some((compiled, choices, consumed)));
    }

    let vote_start = match &first.effect {
        EffectAst::VoteStart {
            options,
            secret,
            starting_with_controller,
        } => Some((
            Some(options.clone()),
            None,
            None,
            *secret,
            *starting_with_controller,
        )),
        EffectAst::VoteStartObjects {
            filter,
            count,
            secret,
            starting_with_controller,
        } => Some((
            None,
            Some((filter.clone(), *count)),
            None,
            *secret,
            *starting_with_controller,
        )),
        EffectAst::VoteStartPlayers {
            filter,
            exclude_voter,
            secret,
            starting_with_controller,
        } => Some((
            None,
            None,
            Some((filter.clone(), *exclude_voter)),
            *secret,
            *starting_with_controller,
        )),
        _ => None,
    };
    let Some((named_options, object_vote, player_vote, secret, starting_with_controller)) =
        vote_start
    else {
        return Ok(None);
    };

    let mut extra_mandatory: u32 = 0;
    let mut extra_optional: u32 = 0;
    let consumed = effects
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(idx, annotated)| match &annotated.effect {
            EffectAst::VoteOption { .. } => Some(idx + 1),
            EffectAst::Conditional { predicate, .. } if is_vote_related_predicate(predicate) => {
                Some(idx + 1)
            }
            effect if vote_extra_amount(effect).is_some() => Some(idx + 1),
            _ => None,
        })
        .next_back()
        .unwrap_or(1);

    for annotated in effects.iter().take(consumed).skip(1) {
        if let Some((count, optional)) = vote_extra_amount(&annotated.effect) {
            if optional {
                extra_optional = extra_optional.saturating_add(count);
            } else {
                extra_mandatory = extra_mandatory.saturating_add(count);
            }
        }
    }

    if let Some((filter, count)) = object_vote {
        let resolved = resolve_it_tag(&filter, &current_reference_env(ctx))?;
        let vote = if extra_optional > 0 {
            crate::effects::VoteEffect::objects(resolved, count, extra_mandatory, extra_optional)
        } else {
            crate::effects::VoteEffect::vote_objects(resolved, count, extra_mandatory)
        }
        .with_secret(secret)
        .starting_with_controller(starting_with_controller);
        let effect = Effect::new(vote);
        let mut compiled = vec![effect];
        preserve_annotated_effect_result_id(first, &mut compiled)?;
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            if vote_extra_amount(&annotated.effect).is_none() {
                let (mut followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
                preserve_annotated_effect_result_id(annotated, &mut followups)?;
                compiled.extend(followups);
                for choice in followup_choices {
                    push_choice(&mut choices, choice);
                }
            }
            apply_local_reference_env(ctx, &annotated.out_env);
        }
        return Ok(Some((compiled, choices, consumed)));
    }

    if let Some((filter, exclude_voter)) = player_vote {
        let vote = crate::effects::VoteEffect::vote_players_with_optional_extra(
            filter,
            exclude_voter,
            extra_mandatory,
            extra_optional,
        )
        .with_secret(secret)
        .starting_with_controller(starting_with_controller);
        let effect = Effect::new(vote);
        let mut compiled = vec![effect];
        preserve_annotated_effect_result_id(first, &mut compiled)?;
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            if vote_extra_amount(&annotated.effect).is_none() {
                let (mut followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
                preserve_annotated_effect_result_id(annotated, &mut followups)?;
                compiled.extend(followups);
                for choice in followup_choices {
                    push_choice(&mut choices, choice);
                }
            }
            apply_local_reference_env(ctx, &annotated.out_env);
        }
        return Ok(Some((compiled, choices, consumed)));
    }

    let mut vote_options = named_options
        .as_ref()
        .expect("named vote start should exist")
        .iter()
        .map(|option| VoteOption::new(option.clone(), Vec::new()))
        .collect::<Vec<_>>();
    let mut choices = Vec::new();
    let mut post_vote_effects = Vec::new();
    for annotated in effects.iter().take(consumed).skip(1) {
        apply_local_reference_env(ctx, &annotated.in_env);
        ctx.auto_tag_object_targets =
            ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
        match &annotated.effect {
            effect if vote_extra_amount(effect).is_some() => {}
            EffectAst::VoteOption { option, effects } => {
                let mut option_effects_ast = effects.clone();
                force_implicit_vote_token_controller_you(&mut option_effects_ast);
                let ast_uses_iterated_player =
                    vote_option_ast_uses_iterated_player(&option_effects_ast);
                let (repeat_effects, repeat_choices) = compile_effects(&option_effects_ast, ctx)?;
                if ast_uses_iterated_player
                    || compiled_vote_option_uses_iterated_player(&repeat_effects, &repeat_choices)
                {
                    let (mut per_vote_effects, per_vote_choices) =
                        compile_effects_in_iterated_player_context(&option_effects_ast, ctx, None)?;
                    preserve_annotated_effect_result_id(annotated, &mut per_vote_effects)?;
                    let mut matching_vote_option = None;
                    for (index, vote_option) in vote_options.iter().enumerate() {
                        if vote_option.name.eq_ignore_ascii_case(option) {
                            matching_vote_option = Some(index);
                            break;
                        }
                    }
                    if let Some(vote_option_idx) = matching_vote_option {
                        vote_options[vote_option_idx]
                            .effects_per_vote
                            .extend(per_vote_effects);
                    }
                    for choice in per_vote_choices {
                        push_choice(&mut choices, choice);
                    }
                } else {
                    let mut repeated = vec![Effect::repeat_effects(
                        Value::VoteCount(option.clone()),
                        repeat_effects,
                    )];
                    preserve_annotated_effect_result_id(annotated, &mut repeated)?;
                    post_vote_effects.extend(repeated);
                    for choice in repeat_choices {
                        push_choice(&mut choices, choice);
                    }
                }
            }
            _ => {
                let (mut followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
                preserve_annotated_effect_result_id(annotated, &mut followups)?;
                post_vote_effects.extend(followups);
                for choice in followup_choices {
                    push_choice(&mut choices, choice);
                }
            }
        }
        apply_local_reference_env(ctx, &annotated.out_env);
    }

    let vote = if extra_optional > 0 {
        crate::effects::VoteEffect::named(vote_options, extra_mandatory, extra_optional)
    } else {
        crate::effects::VoteEffect::new(vote_options, extra_mandatory)
    }
    .with_secret(secret)
    .starting_with_controller(starting_with_controller);
    let effect = Effect::new(vote);
    let mut compiled = vec![effect];
    preserve_annotated_effect_result_id(first, &mut compiled)?;
    compiled.extend(post_vote_effects);

    Ok(Some((compiled, choices, consumed)))
}

pub fn choose_spec_for_targeted_player_filter(filter: &PlayerFilter) -> Option<ChooseSpec> {
    if let PlayerFilter::Target(inner) = filter {
        return Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())));
    }
    None
}

pub fn collect_targeted_player_specs_from_player_filter(
    filter: &PlayerFilter,
    specs: &mut Vec<ChooseSpec>,
) {
    match filter {
        PlayerFilter::Target(inner) => {
            push_choice(
                specs,
                ChooseSpec::target(ChooseSpec::Player((**inner).clone())),
            );
            collect_targeted_player_specs_from_player_filter(inner, specs);
        }
        PlayerFilter::Excluding { base, excluded } => {
            collect_targeted_player_specs_from_player_filter(base, specs);
            collect_targeted_player_specs_from_player_filter(excluded, specs);
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => {
            collect_targeted_player_specs_from_player_filter(base, specs);
        }
        PlayerFilter::LostLifeThisTurn { base } => {
            collect_targeted_player_specs_from_player_filter(base, specs);
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, sources, .. } => {
            collect_targeted_player_specs_from_player_filter(base, specs);
            collect_targeted_player_specs_from_filter(sources, specs);
        }
        _ => {}
    }
}

pub fn collect_targeted_player_specs_from_filter(
    filter: &ObjectFilter,
    specs: &mut Vec<ChooseSpec>,
) {
    if let Some(controller) = &filter.controller
        && let Some(spec) = choose_spec_for_targeted_player_filter(controller)
    {
        push_choice(specs, spec);
    }

    if let Some(owner) = &filter.owner
        && let Some(spec) = choose_spec_for_targeted_player_filter(owner)
    {
        push_choice(specs, spec);
    }

    if let Some(targets_player) = &filter.targets_player
        && let Some(spec) = choose_spec_for_targeted_player_filter(targets_player)
    {
        push_choice(specs, spec);
    }

    if let Some(targets_object) = &filter.targets_object {
        collect_targeted_player_specs_from_filter(targets_object, specs);
    }
}

pub fn target_context_prelude_for_filter(filter: &ObjectFilter) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let mut choices = Vec::new();
    collect_targeted_player_specs_from_filter(filter, &mut choices);
    let effects = choices
        .iter()
        .cloned()
        .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
        .collect();
    (effects, choices)
}

#[cfg(test)]
mod typed_search_predicate_tests {
    use super::*;

    #[test]
    fn removed_counter_metric_survives_runtime_player_and_object_fanout_lowering() {
        let counter_type = crate::object::CounterType::PlusOnePlusOne;
        let removed_count = || {
            Value::PendingPriorEffectMetric(
                ironsmith_core::PriorEffectMetricQuery::new(
                    ironsmith_core::EffectMetricSource::Outcome,
                    ironsmith_core::EffectMetric::Count,
                )
                .with_action(ironsmith_core::PriorEffectAction::Removed)
                .with_counter_type(Some(counter_type)),
            )
        };
        let removal = EffectAst::subject_verb_remove_up_to_any_counters(
            Value::CountersOnSource(counter_type),
            TargetAst::Source(None),
            Some(counter_type),
            false,
        );
        let mut controlled_creature = ObjectFilter::creature().in_zone(Zone::Battlefield);
        controlled_creature.controller = Some(PlayerFilter::IteratedPlayer);
        let fanout = EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::subject_verb_damage(
                    removed_count(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                ),
                EffectAst::subject_verb_damage_each(removed_count(), controlled_creature),
            ],
        };

        let compiled = crate::compile_support::compile_statement_effects(&[removal, fanout])
            .expect("removed count should lower across both fanout frames");
        let debug = format!("{compiled:#?}");

        assert_eq!(
            debug.matches("amount: PriorEffectMetric").count(),
            2,
            "{debug}"
        );
        assert_eq!(debug.matches("action: Some(\n").count(), 2, "{debug}");
        assert_eq!(debug.matches("Removed,\n").count(), 2, "{debug}");
        assert!(
            !debug.contains("amount: EffectValue"),
            "fanout damage must not bind to an inner recipient: {debug}"
        );
    }

    #[test]
    fn searched_action_only_surface_uses_zone_sensitive_search_predicate() {
        let surface = ironsmith_core::PriorEffectResultSurface::new(
            ironsmith_core::PriorEffectAction::Searched,
            ObjectFilter::default(),
            ironsmith_core::PriorEffectResultActor::You,
            ironsmith_core::PriorEffectResultQuantifier::ActionOnly,
        );

        assert_eq!(
            effect_predicate_from_if_result(IfResultPredicate::PriorEffectResult(surface)),
            EffectPredicate::SearchedLibrary
        );
    }

    #[test]
    fn result_followup_ids_the_nested_terminal_coin_flip() {
        let effects = vec![
            EffectAst::CommaThen {
                effects: vec![
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    ),
                    EffectAst::subject_verb_flip_coin(PlayerAst::You),
                ],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
            },
        ];

        let compiled = crate::compile_support::compile_statement_effects(&effects)
            .expect("coin-flip result follow-up should lower");
        let sequence = compiled[0]
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("authored comma-then should remain a sequence");
        assert!(
            sequence.effects[0]
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some(),
            "the preceding action must remain outside the result wrapper"
        );
        let flip_with_id = sequence.effects[1]
            .as_with_id()
            .expect("the terminal coin flip should carry the result ID");
        assert!(
            flip_with_id
                .effect
                .downcast_ref::<crate::effects::FlipCoinEffect>()
                .is_some(),
            "WithId must wrap the coin flip itself"
        );
        let followup = compiled[1]
            .as_if_effect()
            .expect("the coin-flip outcome should gate the follow-up");
        assert_eq!(followup.condition, flip_with_id.id);
    }

    #[test]
    fn each_player_face_result_uses_outer_player_counts_before_followup() {
        let effects = vec![
            EffectAst::ForEachPlayer {
                effects: vec![EffectAst::subject_verb_flip_coin_face_only(PlayerAst::That)],
            },
            EffectAst::ForEachPlayerDid {
                effects: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::That,
                    SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
                predicate: None,
                result_predicate: IfResultPredicate::DidNot,
            },
        ];

        let compiled = crate::compile_support::compile_statement_effects(&effects)
            .expect("each-player coin-face correlation should lower");
        let [flip_result, followup] = compiled.as_slice() else {
            panic!("expected one shared flip result and one correlated follow-up: {compiled:#?}");
        };
        let flip_result = flip_result
            .as_with_id()
            .expect("complete each-player flip must carry the result ID");
        let flip_players = flip_result
            .effect
            .downcast_ref::<crate::effects::ForPlayersEffect<crate::effect::Effect>>()
            .expect("flip result should contain the player loop");
        let [flip] = flip_players.effects.as_slice() else {
            panic!("expected one flip per player: {flip_players:#?}");
        };
        assert!(
            flip.downcast_ref::<crate::effects::FlipCoinEffect>()
                .is_some_and(|flip| flip.kind == ironsmith_core::CoinFlipKind::FaceOnly)
        );

        let followup = followup
            .as_if_effect()
            .expect("player-count result must gate the follow-up");
        assert_eq!(followup.condition, flip_result.id);
        assert_eq!(followup.predicate, EffectPredicate::DidNotHappen);
        assert!(followup.per_player_result);
        assert!(format!("{:#?}", followup.then).contains("IteratedPlayer"));
    }

    #[test]
    fn repeat_process_ids_the_nested_terminal_clash() {
        let process = EffectAst::RepeatProcess {
            effects: vec![EffectAst::Coordinated {
                effects: vec![
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::LoseLife {
                            amount: Value::Fixed(2),
                        },
                    ),
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(2),
                        },
                    ),
                    EffectAst::subject_verb_clash(ClashOpponentAst::Opponent),
                ],
                leading_duration: false,
                result_conjunction: false,
            }],
            continue_effect_index: 0,
            continue_predicate: IfResultPredicate::WonClash,
        };

        let compiled = crate::compile_support::compile_statement_effects(&[process])
            .expect("wrapped clash repeat process should lower");
        let repeat = compiled[0]
            .downcast_ref::<crate::effects::RepeatProcessEffect>()
            .expect("expected a runtime repeat process");
        assert_eq!(
            repeat.predicate,
            EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
        );
        let sequence = repeat.effects[0]
            .downcast_ref::<crate::effects::SequenceEffect>()
            .expect("the coordinated process body should remain a sequence");
        assert!(
            sequence.effects[0]
                .downcast_ref::<crate::effects::LoseLifeEffect>()
                .is_some()
                && sequence.effects[1]
                    .downcast_ref::<crate::effects::DrawCardsEffect>()
                    .is_some(),
            "the non-result actions must remain outside the result wrapper"
        );
        let clash_with_id = sequence.effects[2]
            .as_with_id()
            .expect("the terminal clash should carry the repeat condition ID");
        assert_eq!(clash_with_id.id, repeat.condition);
        assert!(
            clash_with_id
                .effect
                .downcast_ref::<crate::effects::ClashEffect>()
                .is_some(),
            "WithId must wrap the clash itself"
        );
    }
}
