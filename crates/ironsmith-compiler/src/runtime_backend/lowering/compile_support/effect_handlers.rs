use super::*;

fn compile_delayed_trigger_spec(
    trigger: &TriggerSpec,
) -> Result<ironsmith_core::DelayedTriggerSpec, CardTextError> {
    match trigger {
        TriggerSpec::BeginningOfUpkeep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player.clone()),
        ),
        TriggerSpec::BeginningOfDrawStep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player.clone()),
        ),
        TriggerSpec::BeginningOfEndStep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player.clone()),
        ),
        TriggerSpec::IsDealtDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Object(filter.clone())),
        ),
        TriggerSpec::PutIntoGraveyard(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(filter.clone()),
        ),
        TriggerSpec::PutIntoGraveyardFromZone { filter, from } => Ok(
            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                filter: filter.clone(),
                from: *from,
            },
        ),
        TriggerSpec::ThisDies => Ok(ironsmith_core::DelayedTriggerSpec::ThisDies),
        TriggerSpec::ThisAttacksAndIsntBlocked => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked)
        }
        TriggerSpec::Attacks(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::Attacks(filter.clone()))
        }
        TriggerSpec::AttacksAndIsntBlocked(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::AttacksAndIsntBlocked(filter.clone()),
        ),
        TriggerSpec::AttacksOneOrMore(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::AttacksOneOrMore(filter.clone()),
        ),
        TriggerSpec::Blocks(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::Blocks(filter.clone()))
        }
        TriggerSpec::Dies(filter) => Ok(ironsmith_core::DelayedTriggerSpec::Dies(filter.clone())),
        TriggerSpec::SpellCast {
            filter,
            caster,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => Ok(ironsmith_core::DelayedTriggerSpec::SpellCast {
            filter: filter.clone(),
            caster: caster.clone(),
            during_turn: during_turn.clone(),
            min_spells_this_turn: *min_spells_this_turn,
            exact_spells_this_turn: *exact_spells_this_turn,
            from_not_hand: *from_not_hand,
        }),
        other => Err(CardTextError::ParseError(format!(
            "unsupported delayed trigger spec: {other:?}"
        ))),
    }
}

pub(super) fn try_compile_timing_and_control_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::DelayedUntilNextEndStep { player, effects } => {
            let (delayed_effects, choices) = compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextUpkeep { player, effects } => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextDrawStep { player, effects } => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects } => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilEndOfCombat { effects } => {
            let (delayed_effects, choices) = compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::EndOfCombat,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedTriggerThisTurn { trigger, effects } => {
            let (delayed_effects, choices) = compile_trigger_effects(Some(trigger), effects)?;
            match trigger {
                TriggerSpec::IsDealtDamage(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Source),
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::IsDealtDamage(
                                    ChooseSpec::Object(resolved_filter),
                                ),
                                delayed_effects,
                                false,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyard(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisDies,
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                false,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyardFromZone { filter, from } => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                                filter: resolved_filter,
                                from: *from,
                            },
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
                TriggerSpec::ThisAttacksAndIsntBlocked => {
                    if let Some(target_tag) = ctx.last_object_tag.clone() {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            target_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked,
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked,
                                delayed_effects,
                                false,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::AttacksAndIsntBlocked(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked,
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::AttacksAndIsntBlocked(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                false,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                _ => {
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            compile_delayed_trigger_spec(trigger)?,
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
            }
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { filter, effects } => {
            let target_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule 'dies this turn' trigger without prior object context"
                        .to_string(),
                )
            })?;
            let previous_last = ctx.last_object_tag.clone();
            ctx.last_object_tag = Some("triggering".to_string());
            let compiled = compile_effects_preserving_last_effect(effects, ctx);
            ctx.last_object_tag = previous_last;
            let (delayed_effects, choices) = compiled?;
            let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                target_tag.clone().into(),
                ironsmith_core::DelayedTriggerSpec::ThisDies,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            );
            if let Some(filter) = filter {
                delayed = delayed
                    .with_target_filter(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_destroy_and_exile_effect(
    _effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    Ok(None)
}

fn predicate_object_filter_antecedent(predicate: &PredicateAst) -> Option<ObjectFilter> {
    match predicate {
        PredicateAst::PlayerControls { filter, .. }
        | PredicateAst::PlayerControlsAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerControlsAtLeastWithDifferentPowers { filter, .. }
        | PredicateAst::PlayerControlsNo { filter, .. }
        | PredicateAst::PlayerControlsMost { filter, .. } => Some(filter.clone()),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => predicate_object_filter_antecedent(left)
            .or_else(|| predicate_object_filter_antecedent(right)),
        _ => None,
    }
}

fn merge_filter_overlay(base: &mut ObjectFilter, overlay: ObjectFilter) {
    if let Some(zone) = overlay.zone {
        base.zone.get_or_insert(zone);
    }
    if base.controller.is_none() {
        base.controller = overlay.controller;
    }
    if base.owner.is_none() {
        base.owner = overlay.owner;
    }
    base.other |= overlay.other;
    for card_type in overlay.card_types {
        if !base.card_types.contains(&card_type) {
            base.card_types.push(card_type);
        }
    }
    for subtype in overlay.subtypes {
        if !base.subtypes.contains(&subtype) {
            base.subtypes.push(subtype);
        }
    }
    if let Some(colors) = overlay.colors {
        base.colors = Some(
            base.colors
                .map_or(colors, |existing| existing.intersection(colors)),
        );
    }
}

fn bind_condition_filter_antecedent(filter: &mut ObjectFilter, antecedent: &ObjectFilter) {
    let references_it = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
    });
    if !references_it {
        return;
    }

    let mut overlay = filter.clone();
    overlay.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == IT_TAG
            && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject))
    });
    let mut replacement = antecedent.clone();
    merge_filter_overlay(&mut replacement, overlay);
    *filter = replacement;
}

fn bind_condition_antecedent_in_target(target: &mut TargetAst, antecedent: &ObjectFilter) {
    fn bind_random_those_filter(filter: &mut ObjectFilter, antecedent: &ObjectFilter) {
        let mut replacement = antecedent.clone();
        merge_filter_overlay(&mut replacement, filter.clone());
        *filter = replacement;
    }

    match target {
        TargetAst::Object(filter, _, _) => bind_condition_filter_antecedent(filter, antecedent),
        TargetAst::WithCount(inner, count) => {
            if count.random
                && let TargetAst::Object(filter, _, _) = inner.as_mut()
                && filter.tagged_constraints.is_empty()
                && filter.with_counter.is_none()
            {
                bind_random_those_filter(filter, antecedent);
            } else {
                bind_condition_antecedent_in_target(inner, antecedent);
            }
        }
        _ => {}
    }
}

fn bind_condition_antecedent_in_effect(effect: &mut EffectAst, antecedent: &ObjectFilter) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. } => {
                bind_condition_antecedent_in_target(target, antecedent);
            }
            _ => {}
        },
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. } => {
            bind_condition_filter_antecedent(filter, antecedent);
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            bind_condition_antecedent_in_effects(if_true, antecedent);
            bind_condition_antecedent_in_effects(if_false, antecedent);
        }
        _ => {}
    }
}

fn bind_condition_antecedent_in_effects(effects: &mut [EffectAst], antecedent: &ObjectFilter) {
    for effect in effects {
        bind_condition_antecedent_in_effect(effect, antecedent);
    }
}

pub(super) fn try_compile_stack_and_condition_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::ResolvedIfResult {
            condition,
            predicate,
            effects,
        } => {
            let (inner_effects, inner_choices) =
                with_preserved_lowering_context(ctx, |_| {}, |ctx| compile_effects(effects, ctx))?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect = Effect::if_then(*condition, predicate, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ResolvedWhenResult {
            condition,
            predicate,
            effects,
        } => {
            let (inner_effects, inner_choices) =
                with_preserved_lowering_context(ctx, |_| {}, |ctx| compile_effects(effects, ctx))?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect =
                Effect::reflexive_trigger(*condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            let mut effective_if_true = if_true.clone();
            if let Some(antecedent) = predicate_object_filter_antecedent(predicate) {
                bind_condition_antecedent_in_effects(&mut effective_if_true, &antecedent);
            }
            let saved_last_tag = ctx.last_object_tag.clone();
            let saved_source_object_antecedent = ctx.source_object_antecedent;
            ctx.source_object_antecedent |= predicate.establishes_source_object_antecedent();
            let (true_effects, true_choices) = compile_effects(&effective_if_true, ctx)?;
            let true_last_tag = ctx.last_object_tag.clone();
            ctx.last_object_tag = saved_last_tag.clone();
            ctx.source_object_antecedent =
                saved_source_object_antecedent || predicate.establishes_source_object_antecedent();
            let (false_effects, false_choices) = compile_effects(if_false, ctx)?;
            ctx.source_object_antecedent = saved_source_object_antecedent;
            let predicate_references_it = matches!(
                predicate,
                PredicateAst::ItIsLandCard
                    | PredicateAst::ItIsSoulbondPaired
                    | PredicateAst::ItMatches(_)
            ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == IT_TAG)
                || matches!(predicate, PredicateAst::TaggedWasCast(tag) if tag.as_str() == IT_TAG)
                || matches!(
                    predicate,
                    PredicateAst::TargetMatches(filter) if filter_references_tag(filter, IT_TAG)
                )
                || matches!(
                    predicate,
                    PredicateAst::PlayerTaggedObjectMatches { tag, .. } if tag.as_str() == IT_TAG
                );

            let antecedent_choice = if saved_last_tag.is_none() && predicate_references_it {
                let mut antecedent_choice = None;
                for choice in true_choices.iter().chain(false_choices.iter()) {
                    if choice.is_target() && choose_spec_targets_object(choice) {
                        antecedent_choice = Some(choice.clone());
                        break;
                    }
                }
                antecedent_choice
            } else {
                None
            };

            let mut condition_reference_tag = saved_last_tag.clone();
            let mut prelude = Vec::new();
            if condition_reference_tag.is_none()
                && let Some(choice) = antecedent_choice.clone()
            {
                let tag = if let Some(existing) = tagged_alias_for_choice(&true_effects, &choice) {
                    existing
                } else {
                    ctx.next_tag("targeted")
                };
                prelude.push(
                    Effect::new(crate::effects::SequenceEffect::new(Vec::new())).tag(tag.clone()),
                );
                condition_reference_tag = Some(tag);
            }

            let original_last_tag = ctx.last_object_tag.clone();
            ctx.last_object_tag = condition_reference_tag.clone().or(saved_last_tag.clone());
            let condition =
                compile_condition_from_predicate_ast(predicate, ctx, &condition_reference_tag)?;
            ctx.last_object_tag = original_last_tag;

            let conditional = if false_effects.is_empty() {
                Effect::conditional_only(condition, true_effects)
            } else {
                Effect::conditional(condition, true_effects, false_effects)
            };
            prelude.push(conditional);

            if let Some(reference_tag) = condition_reference_tag {
                ctx.last_object_tag = Some(reference_tag);
            } else if if_false.is_empty() {
                ctx.last_object_tag = true_last_tag.clone().or(saved_last_tag.clone());
            } else {
                ctx.last_object_tag = saved_last_tag.clone();
            }

            let mut choices = true_choices;
            for choice in false_choices {
                push_choice(&mut choices, choice);
            }
            if let Some(choice) = antecedent_choice {
                push_choice(&mut choices, choice);
            }
            (prelude, choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_attachment_and_setup_effect(
    _effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    Ok(None)
}
