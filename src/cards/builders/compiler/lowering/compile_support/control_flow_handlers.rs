use super::*;

fn tag_last_discard_in_effects(effects: &mut [EffectAst], tag: &TagKey) -> bool {
    for effect in effects.iter_mut().rev() {
        if let EffectAst::Discard {
            tag: discard_tag, ..
        } = effect
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

pub(crate) fn compile_if_do_with_opponent_did(
    first: &EffectAst,
    second: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let EffectAst::ForEachOpponentDid {
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
        let mut merged_opponent_effects = opponent_effects.clone();
        merged_opponent_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
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
        let conditional = Effect::if_then(id, EffectPredicate::Happened, inner_effects);
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
    merged_opponent_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
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
        let mut merged_player_effects = player_effects.clone();
        merged_player_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
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

    let Some(EffectAst::ForEachPlayer {
        effects: player_effects,
    }) = first_effects.first()
    else {
        return Ok(None);
    };

    let mut merged_player_effects = player_effects.clone();
    merged_player_effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
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
        IfResultPredicate::DidNot => EffectPredicate::DidNotHappen,
        IfResultPredicate::DiesThisWay => EffectPredicate::HappenedNotReplaced,
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
        let (mut effect_list, effect_choices) = compile_effect(effect, ctx)?;
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

pub(crate) fn compile_effects_in_iterated_player_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
    tagged_object: Option<String>,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut iterated_frame = saved_frame.clone();
    iterated_frame.iterated_player = true;
    iterated_frame.last_effect_id = None;
    iterated_frame.last_player_filter = Some(PlayerFilter::IteratedPlayer);
    if tagged_object.is_some() {
        iterated_frame.last_object_tag = Some(IT_TAG.to_string());
    }

    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, iterated_frame)?;
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

pub(crate) fn force_implicit_vote_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::CreateTokenWithMods { player, .. }
            | EffectAst::CreateTokenCopy { player, .. }
            | EffectAst::CreateTokenCopyFromSource { player, .. } => {
                if matches!(player, PlayerAst::Implicit) {
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

fn compiled_vote_option_uses_iterated_player(effects: &[Effect], choices: &[ChooseSpec]) -> bool {
    str_contains(format!("{effects:?}{choices:?}").as_str(), "IteratedPlayer")
}

pub(crate) fn compile_vote_sequence(
    effects: &[AnnotatedEffect],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>, usize)>, CardTextError> {
    let Some(first) = effects.first() else {
        return Ok(None);
    };
    let vote_start = match &first.effect {
        EffectAst::VoteStart { options } => Some((Some(options.clone()), None)),
        EffectAst::VoteStartObjects { filter, count } => {
            Some((None, Some((filter.clone(), *count))))
        }
        _ => None,
    };
    let Some((named_options, object_vote)) = vote_start else {
        return Ok(None);
    };

    let mut extra_mandatory: u32 = 0;
    let mut extra_optional: u32 = 0;
    let consumed = effects
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(idx, annotated)| match &annotated.effect {
            EffectAst::VoteOption { .. } | EffectAst::VoteExtra { .. } => Some(idx + 1),
            EffectAst::Conditional { predicate, .. } if is_vote_related_predicate(predicate) => {
                Some(idx + 1)
            }
            _ => None,
        })
        .last()
        .unwrap_or(1);

    for annotated in effects.iter().take(consumed).skip(1) {
        if let EffectAst::VoteExtra { count, optional } = &annotated.effect {
            if *optional {
                extra_optional = extra_optional.saturating_add(*count);
            } else {
                extra_mandatory = extra_mandatory.saturating_add(*count);
            }
        }
    }

    if let Some((filter, count)) = object_vote {
        let resolved = resolve_it_tag(&filter, &current_reference_env(ctx))?;
        let effect = if extra_optional > 0 {
            Effect::vote_objects_with_optional_extra(
                resolved,
                count,
                extra_mandatory,
                extra_optional,
            )
        } else {
            Effect::vote_objects(resolved, count, extra_mandatory)
        };
        let mut compiled = vec![effect];
        let mut choices = Vec::new();
        for annotated in effects.iter().take(consumed).skip(1) {
            apply_local_reference_env(ctx, &annotated.in_env);
            ctx.auto_tag_object_targets =
                ctx.force_auto_tag_object_targets || annotated.auto_tag_object_targets;
            match &annotated.effect {
                EffectAst::VoteExtra { .. } => {}
                _ => {
                    let (followups, followup_choices) = compile_effect(&annotated.effect, ctx)?;
                    compiled.extend(followups);
                    for choice in followup_choices {
                        push_choice(&mut choices, choice);
                    }
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
            EffectAst::VoteExtra { .. } => {}
            EffectAst::VoteOption { option, effects } => {
                let mut option_effects_ast = effects.clone();
                force_implicit_vote_token_controller_you(&mut option_effects_ast);
                let (repeat_effects, repeat_choices) = compile_effects(&option_effects_ast, ctx)?;
                if compiled_vote_option_uses_iterated_player(&repeat_effects, &repeat_choices) {
                    let (per_vote_effects, per_vote_choices) =
                        compile_effects_in_iterated_player_context(&option_effects_ast, ctx, None)?;
                    if let Some(vote_option_idx) =
                        find_index(vote_options.as_slice(), |vote_option| {
                            vote_option.name.eq_ignore_ascii_case(option)
                        })
                    {
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

    let effect = if extra_optional > 0 {
        Effect::vote_with_optional_extra(vote_options, extra_mandatory, extra_optional)
    } else {
        Effect::vote(vote_options, extra_mandatory)
    };
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
