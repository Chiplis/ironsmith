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
        && predicate_tag.as_str() == IT_TAG
    {
        *predicate_tag = tag.clone();
    }
    bound
}

pub(crate) fn compile_if_do_with_opponent_doesnt(
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
            let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
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
            let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
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
        let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
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

pub(crate) fn compile_if_do_with_player_doesnt(
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
            let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
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
        let explicit_tag = TagKey::from(ctx.next_tag("discarded").as_str());
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
    } else {
        IfResultPredicate::Did
    }
}

pub(crate) fn compile_if_do_with_opponent_did(
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

pub(crate) fn compile_if_do_with_player_did(
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

pub(crate) fn compile_result_followup(
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
        let damage_idx = first_effects
            .iter()
            .rposition(effect_contains_deal_damage)
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
    } else {
        let last = first_effects
            .pop()
            .expect("nonempty antecedent checked above");
        first_effects.push(Effect::with_id(id.0, last));
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

pub(crate) fn with_preserved_lowering_context<T, Configure, Run>(
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

pub(crate) fn compile_effects_preserving_last_effect(
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

pub(crate) fn effect_predicate_from_if_result(predicate: IfResultPredicate) -> EffectPredicate {
    match predicate {
        IfResultPredicate::Did => EffectPredicate::Happened,
        IfResultPredicate::WonClash => {
            EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0))
        }
        IfResultPredicate::AcceptedChoice => EffectPredicate::Chosen,
        IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
        IfResultPredicate::SearchedLibrary => EffectPredicate::SearchedLibrary,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
        IfResultPredicate::ExcessDamageDealt => EffectPredicate::ExcessDamageDealt,
        IfResultPredicate::DealtDamageToPlayer => EffectPredicate::DealtDamageToPlayer,
        IfResultPredicate::AffectedObjectMatchesCardType { card_type, negated } => {
            EffectPredicate::AffectedObjectMatchesCardType { card_type, negated }
        }
        IfResultPredicate::PriorEffectResult(surface) => {
            EffectPredicate::PriorEffectResult(surface)
        }
        IfResultPredicate::WasDeclined => EffectPredicate::WasDeclined,
        IfResultPredicate::Value(cmp) => EffectPredicate::Value(cmp),
    }
}

pub(crate) fn compile_repeat_process_body(
    effects: &[EffectAst],
    continue_effect_index: usize,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, EffectId), CardTextError> {
    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut condition: Option<EffectId> = None;

    for (idx, effect) in effects.iter().enumerate() {
        let (mut effect_list, effect_choices) = if idx == continue_effect_index {
            if let Some(compiled) = compile_starting_with_controller_pay_life_process(effect, ctx)?
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
            assign_effect_result_id(
                &mut effect_list,
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

fn compile_starting_with_controller_pay_life_process(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachPlayer { effects } = effect else {
        return Ok(None);
    };
    let [EffectAst::SubjectVerb(subject_verb)] = effects.as_slice() else {
        return Ok(None);
    };
    if subject_verb.subject.role != SubjectVerbRoleAst::AffectedPlayer
        || subject_verb.subject.player != PlayerAst::That
        || !matches!(subject_verb.action, SubjectVerbActionAst::PayAnyLife { .. })
    {
        return Ok(None);
    }

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

pub(crate) fn compile_effects_in_iterated_player_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
    tagged_object: Option<String>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    iterated_frame.last_effect_id = None;
    if tagged_object.is_some() {
        // A tagged-object loop establishes `__it__`, but it does not replace
        // an outer player antecedent with an artificial iterated player.
        iterated_frame.last_object_tag = Some(IT_TAG.to_string());
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

pub(crate) fn compile_effects_in_iterated_object_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    // Iterating objects establishes `__it__`, not an iterated player. Preserve
    // an outer player iteration when one exists; otherwise contextual
    // `that player` filters continue to resolve to the saved antecedent.
    iterated_frame.last_effect_id = None;
    iterated_frame.last_object_tag = Some(IT_TAG.to_string());
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

pub(crate) fn force_implicit_vote_token_controller_you(effects: &mut [EffectAst]) {
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
        EffectAst::MayByPlayer { player, effects }
            if matches!(player, PlayerAst::You | PlayerAst::Implicit) =>
        {
            match effects.as_slice() {
                [EffectAst::VoteExtra { count, .. }] => Some((*count, true)),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn compile_vote_sequence(
    effects: &[AnnotatedEffect],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(first) = effects.first() else {
        return Ok(None);
    };
    if let EffectAst::SecretChoiceStart {
        options,
        participants,
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
            .last()
            .unwrap_or(1);

        let mut compiled = vec![Effect::new(crate::effects::SecretChoiceEffect::new(
            options.clone(),
            participants.clone(),
        ))];
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
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
        .last()
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
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            if vote_extra_amount(&annotated.effect).is_none() {
                let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
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
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            if vote_extra_amount(&annotated.effect).is_none() {
                let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
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
                    let (per_vote_effects, per_vote_choices) =
                        compile_effects_in_iterated_player_context(&option_effects_ast, ctx, None)?;
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
                    post_vote_effects.push(Effect::repeat_effects(
                        Value::VoteCount(option.clone()),
                        repeat_effects,
                    ));
                    for choice in repeat_choices {
                        push_choice(&mut choices, choice);
                    }
                }
            }
            _ => {
                let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
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
    compiled.extend(post_vote_effects);

    Ok(Some((compiled, choices, consumed)))
}

pub(crate) fn choose_spec_for_targeted_player_filter(filter: &PlayerFilter) -> Option<ChooseSpec> {
    if let PlayerFilter::Target(inner) = filter {
        return Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())));
    }
    None
}

pub(crate) fn collect_targeted_player_specs_from_player_filter(
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
        _ => {}
    }
}

pub(crate) fn collect_targeted_player_specs_from_filter(
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

pub(crate) fn target_context_prelude_for_filter(
    filter: &ObjectFilter,
) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let mut choices = Vec::new();
    collect_targeted_player_specs_from_filter(filter, &mut choices);
    let effects = choices
        .iter()
        .cloned()
        .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
        .collect();
    (effects, choices)
}
