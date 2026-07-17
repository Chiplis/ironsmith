use super::*;
use crate::runtime_backend::condition_antecedent::{
    ConditionAntecedentBinding, bind_condition_antecedent_in_effects,
    bind_condition_counter_antecedent_in_effects,
    bind_random_count_condition_antecedent_in_effects, predicate_object_filter_antecedent,
    predicate_source_counter_antecedent,
};

pub(crate) fn compile_delayed_trigger_spec(
    trigger: &TriggerSpec,
) -> Result<ironsmith_core::DelayedTriggerSpec, CardTextError> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => compile_delayed_trigger_spec(trigger),
        TriggerSpec::BeginningOfUpkeep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player.clone()),
        ),
        TriggerSpec::BeginningOfDrawStep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player.clone()),
        ),
        TriggerSpec::BeginningOfEndStep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player.clone()),
        ),
        TriggerSpec::BeginningOfTheEndStep => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
        ),
        TriggerSpec::BeginningOfCombat(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfCombat(player.clone()),
        ),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Ok(ironsmith_core::DelayedTriggerSpec::BeginningOfPrecombatMainPhase(player.clone()))
        }
        TriggerSpec::BeginningOfPostcombatMain(player) => {
            Ok(ironsmith_core::DelayedTriggerSpec::BeginningOfPostcombatMainPhase(player.clone()))
        }
        TriggerSpec::ThisEntersBattlefield => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefield)
        }
        TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface,
            subject_number,
        } => Ok(
            ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefieldWithSurface {
                surface: surface.clone(),
                subject_number: *subject_number,
            },
        ),
        TriggerSpec::IsDealtDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Object(filter.clone())),
        ),
        TriggerSpec::IsDealtCombatDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Object(filter.clone())),
        ),
        TriggerSpec::PutIntoGraveyard(filter) | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(
                filter.clone(),
            ))
        }
        TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from,
            one_or_more,
        } => Ok(
            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                filter: filter.clone(),
                from: *from,
                one_or_more: *one_or_more,
            },
        ),
        TriggerSpec::ThisDies => Ok(ironsmith_core::DelayedTriggerSpec::ThisDies),
        TriggerSpec::ThisLeavesBattlefield => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield)
        }
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
        TriggerSpec::BlocksOneOrMore(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::BlocksOneOrMore(filter.clone()),
        ),
        TriggerSpec::BecomesBlocked(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::BecomesBlocked(filter.clone()),
        ),
        TriggerSpec::LeavesBattlefield(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::LeavesBattlefield(filter.clone()),
        ),
        TriggerSpec::Dies(filter) | TriggerSpec::DiesOneOrMore(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::Dies(filter.clone()))
        }
        TriggerSpec::PermanentBecomesTapped(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(filter.clone()),
        ),
        TriggerSpec::DealsCombatDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(filter.clone()),
        ),
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            Ok(ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                source: source.clone(),
                target: target.clone(),
            })
        }
        TriggerSpec::DealsCombatDamageToPlayer { source, player }
        | TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player } => Ok(
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                source: source.clone(),
                player: player.clone(),
            },
        ),
        TriggerSpec::SpellCast {
            filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => Ok(ironsmith_core::DelayedTriggerSpec::SpellCast {
            filter: filter.clone(),
            caster: caster.clone(),
            timing: *timing,
            during_turn: during_turn.clone(),
            min_spells_this_turn: *min_spells_this_turn,
            exact_spells_this_turn: *exact_spells_this_turn,
            from_not_hand: *from_not_hand,
            first_spell_of_game: false,
        }),
        TriggerSpec::PlayerPlaysLand { player, filter } => {
            Ok(ironsmith_core::DelayedTriggerSpec::PlayerPlaysLand {
                player: player.clone(),
                filter: filter.clone(),
            })
        }
        TriggerSpec::AbilityActivated {
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            activation_cost_has_tap,
        } => Ok(ironsmith_core::DelayedTriggerSpec::AbilityActivated {
            activator: activator.clone(),
            filter: filter.clone(),
            non_mana_only: *non_mana_only,
            loyalty_only: *loyalty_only,
            activation_cost_has_tap: *activation_cost_has_tap,
        }),
        TriggerSpec::Either(left, right) => Ok(ironsmith_core::DelayedTriggerSpec::Either(
            Box::new(compile_delayed_trigger_spec(left)?),
            Box::new(compile_delayed_trigger_spec(right)?),
        )),
        other => Err(CardTextError::ParseError(format!(
            "unsupported delayed trigger spec: {other:?}"
        ))),
    }
}

fn compile_delayed_effects_preserving_outer_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut delayed_frame = saved_frame.clone();
    delayed_frame.last_effect_id = None;
    let mut id_gen = ctx.id_gen_context();
    let (compiled, choices, mut frame_out) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, delayed_frame)?;
    frame_out.last_effect_id = saved_frame.last_effect_id;
    ctx.apply_id_gen_context(id_gen);
    ctx.apply_lowering_frame(frame_out);
    Ok((compiled, choices))
}

fn rewrite_filter_tag_relation(
    filter: &mut ObjectFilter,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) {
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag.as_str() == tag && constraint.relation == from {
            constraint.relation = to;
        }
    }
}

fn rewrite_choose_spec_tag_relation(
    spec: &mut ChooseSpec,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            rewrite_choose_spec_tag_relation(spec, tag, from, to);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            rewrite_filter_tag_relation(filter, tag, from, to);
        }
        _ => {}
    }
}

fn rewrite_effect_tag_relation(
    effect: Effect,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) -> Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            rewrite_effect_tag_relation((*tagged.effect).clone(), tag, from, to),
        ));
    }

    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return Effect::new(
            crate::effects::ConditionalEffect::new(
                conditional.condition.clone(),
                rewrite_effects_tag_relation(conditional.if_true.clone(), tag, from, to),
                rewrite_effects_tag_relation(conditional.if_false.clone(), tag, from, to),
            )
            .with_surface(conditional.surface),
        );
    }

    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        let mut rewritten = apply.clone();
        if let crate::continuous::EffectTarget::Filter(filter) = &mut rewritten.target {
            rewrite_filter_tag_relation(filter, tag, from, to);
        }
        if let Some(spec) = &mut rewritten.target_spec {
            rewrite_choose_spec_tag_relation(spec, tag, from, to);
        }
        return Effect::new(rewritten);
    }

    effect
}

fn rewrite_effects_tag_relation(
    effects: Vec<Effect>,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) -> Vec<Effect> {
    effects
        .into_iter()
        .map(|effect| rewrite_effect_tag_relation(effect, tag, from, to))
        .collect()
}

fn trigger_without_intro(trigger: &TriggerSpec) -> &TriggerSpec {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_without_intro(trigger),
        trigger => trigger,
    }
}

fn apply_delayed_trigger_duration(
    delayed: crate::effects::ScheduleDelayedTriggerEffect,
    duration: &Until,
) -> Result<crate::effects::ScheduleDelayedTriggerEffect, CardTextError> {
    match duration {
        Until::EndOfTurn => Ok(delayed.until_end_of_turn()),
        Until::EndOfCombat => Ok(delayed.until_end_of_combat()),
        Until::YourNextTurn => Ok(delayed.until_controller_next_turn()),
        other => Err(CardTextError::ParseError(format!(
            "unsupported delayed-trigger duration: {other:?}"
        ))),
    }
}

fn compile_duration_scoped_delayed_trigger(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
    one_shot: bool,
    duration: &Until,
    either_of_watched_objects: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (delayed_effects, _delayed_choices) = compile_trigger_effects(Some(trigger), effects)?;
    let refs = current_reference_env(ctx);
    let mut watched_tag = None;
    let mut watched_filter = None;
    let mut watch_ability_source = false;
    let mut watch_all_object_targets = false;

    let delayed_trigger = match trigger_without_intro(trigger) {
        TriggerSpec::PermanentBecomesTapped(filter) => {
            let resolved = resolve_it_tag(filter, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(ObjectFilter::source())
            } else {
                ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(resolved)
            }
        }
        TriggerSpec::DealsCombatDamage(filter) => {
            let resolved = resolve_it_tag(filter, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(ObjectFilter::source())
            } else if either_of_watched_objects {
                watch_all_object_targets = true;
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(ObjectFilter::source())
            } else {
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(resolved)
            }
        }
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            let resolved_source = resolve_it_tag(source, &refs)?;
            let resolved_target = resolve_it_tag(target, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved_source) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved_source);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                    source: ObjectFilter::source(),
                    target: resolved_target,
                }
            } else {
                watch_ability_source = resolved_source.source || resolved_target.source;
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                    source: resolved_source,
                    target: resolved_target,
                }
            }
        }
        _ => compile_delayed_trigger_spec(trigger)?,
    };

    let mut delayed = if let Some(tag) = watched_tag {
        crate::effects::ScheduleDelayedTriggerEffect::from_tag(
            tag,
            delayed_trigger,
            delayed_effects,
            one_shot,
            Vec::new(),
            PlayerFilter::You,
        )
    } else {
        crate::effects::ScheduleDelayedTriggerEffect::new(
            delayed_trigger,
            delayed_effects,
            one_shot,
            Vec::new(),
            PlayerFilter::You,
        )
    };
    if let Some(filter) = watched_filter {
        delayed = delayed.with_target_filter(filter);
    }
    if watch_ability_source {
        delayed = delayed.watch_ability_source();
    }
    if watch_all_object_targets {
        delayed = delayed.watch_all_object_targets();
    }
    if either_of_watched_objects {
        delayed = delayed.with_either_of_watched_objects_surface();
    }
    delayed = apply_delayed_trigger_duration(delayed, duration)?;

    Ok((vec![Effect::new(delayed)], Vec::new()))
}

pub(super) fn try_compile_timing_and_control_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::DelayedUntilNextEndStep { player, effects } => {
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
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
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
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
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
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
        EffectAst::DelayedUntilNextMainPhase { player, effects } => {
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfMainPhase(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilEndStepOfExtraTurn { player, effects } => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
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
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::EndOfCombat,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedTriggerForDuration {
            trigger,
            effects,
            one_shot,
            duration,
            either_of_watched_objects,
        } => {
            return compile_duration_scoped_delayed_trigger(
                trigger,
                effects,
                *one_shot,
                duration,
                *either_of_watched_objects,
                ctx,
            )
            .map(Some);
        }
        EffectAst::DelayedTriggerThisTurn {
            trigger,
            effects,
            one_shot,
            until_end_of_combat,
            attach_to_previous_ability: _,
        } => {
            let (delayed_effects, _delayed_choices) =
                compile_trigger_effects(Some(trigger), effects)?;
            let choices = Vec::new();
            match trigger {
                TriggerSpec::IsDealtDamage(filter) | TriggerSpec::IsDealtCombatDamage(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Source),
                            delayed_effects,
                            *one_shot,
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
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
                    let resolved_filter = resolve_it_tag(source, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                                source: crate::target::ObjectFilter::source(),
                                player: player.clone(),
                            },
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let mut delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        if *until_end_of_combat {
                            delayed = delayed.until_end_of_combat();
                        }
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
                            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                                source: resolved_filter,
                                player: player.clone(),
                            },
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn();
                        if *until_end_of_combat {
                            delayed = delayed.until_end_of_combat();
                        }
                        (vec![Effect::new(delayed)], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyard(filter)
                | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let watched_tag = watch_tag_from_filter(&resolved_filter).or_else(|| {
                        filter_references_tag(filter, IT_TAG)
                            .then(|| ctx.last_object_tag.clone())
                            .flatten()
                            .map(TagKey::from)
                    });
                    if let Some(watched_tag) = watched_tag {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(watched_tag.clone()),
                                ..Default::default()
                            },
                        )?;
                        let delayed_effects = lowered.effects.to_vec();
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisDies,
                            delayed_effects,
                            *one_shot,
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
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::Dies(filter) | TriggerSpec::DiesOneOrMore(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let watched_tag = watch_tag_from_filter(&resolved_filter).or_else(|| {
                        filter_references_tag(filter, IT_TAG)
                            .then(|| ctx.last_object_tag.clone())
                            .flatten()
                            .map(TagKey::from)
                    });
                    if let Some(watched_tag) = watched_tag {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(watched_tag.clone()),
                                ..Default::default()
                            },
                        )?;
                        let delayed_effects = lowered.effects.to_vec();
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisDies,
                            delayed_effects,
                            *one_shot,
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
                                ironsmith_core::DelayedTriggerSpec::Dies(resolved_filter),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::LeavesBattlefield(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let watched_tag = watch_tag_from_filter(&resolved_filter).or_else(|| {
                        filter_references_tag(filter, IT_TAG).then(|| TagKey::from("targeted_0"))
                    });
                    if let Some(watched_tag) = watched_tag {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(watched_tag.clone()),
                                ..Default::default()
                            },
                        )?;
                        let delayed_effects = lowered.effects.to_vec();
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield,
                            delayed_effects,
                            *one_shot,
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
                                ironsmith_core::DelayedTriggerSpec::LeavesBattlefield(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyardFromZone {
                    filter,
                    from,
                    one_or_more,
                } => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                                filter: resolved_filter,
                                from: *from,
                                one_or_more: *one_or_more,
                            },
                            delayed_effects,
                            *one_shot,
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
                            *one_shot,
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
                                *one_shot,
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
                            *one_shot,
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
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::DealsCombatDamageToPlayerOneOrMore { source, player } => {
                    let resolved_source = resolve_it_tag(source, &current_reference_env(ctx))?;
                    let trigger = ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                        source: resolved_source.clone(),
                        player: player.clone(),
                    };
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_source) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone().into(),
                            trigger,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_source)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                trigger,
                                delayed_effects,
                                *one_shot,
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
                            *one_shot,
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
            )
            .until_end_of_turn();
            if let Some(filter) = filter {
                delayed = delayed
                    .with_target_filter(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        EffectAst::DelayedWhenLastObjectLeavesBattlefield { filter, effects } => {
            let target_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule leaves-the-battlefield trigger without prior object context"
                        .to_string(),
                )
            })?;
            let previous_last = ctx.last_object_tag.clone();
            ctx.last_object_tag = Some("triggering".to_string());
            let compiled = compile_effects_preserving_last_effect(effects, ctx);
            ctx.last_object_tag = previous_last;
            let (delayed_effects, choices) = compiled?;

            let mut watched_filter = filter.clone();
            watched_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: target_tag.clone().into(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                target_tag.into(),
                ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            )
            .with_target_filter(watched_filter);
            (vec![Effect::new(delayed)], choices)
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
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(*condition);
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(predicate.clone());
            let effect = Effect::if_then(*condition, predicate, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ResolvedWhenResult {
            condition,
            predicate,
            effects,
        } => {
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(*condition);
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(predicate.clone());
            let effect =
                Effect::reflexive_trigger(*condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::TrailingIf { predicate, effects } => {
            let conditional = EffectAst::Conditional {
                predicate: predicate.clone(),
                if_true: effects.clone(),
                if_false: Vec::new(),
            };
            let Some((mut compiled, choices)) =
                try_compile_stack_and_condition_effect(&conditional, ctx)?
            else {
                return Err(CardTextError::ParseError(
                    "failed to lower trailing-if condition".to_string(),
                ));
            };
            let Some(lowered_conditional_effect) = compiled.pop() else {
                return Err(CardTextError::ParseError(
                    "trailing-if condition lowered without an effect".to_string(),
                ));
            };
            let Some(lowered_conditional) =
                lowered_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
            else {
                return Err(CardTextError::ParseError(
                    "trailing-if condition did not lower to a conditional".to_string(),
                ));
            };
            compiled.push(Effect::new(
                crate::effects::ConditionalEffect::new(
                    lowered_conditional.condition.clone(),
                    lowered_conditional.if_true.clone(),
                    lowered_conditional.if_false.clone(),
                )
                .with_surface(ironsmith_core::ConditionalSurface::TrailingIf),
            ));
            (compiled, choices)
        }
        EffectAst::TrailingUnless { predicate, effects } => {
            let conditional = EffectAst::Conditional {
                predicate: PredicateAst::Not(Box::new(predicate.clone())),
                if_true: effects.clone(),
                if_false: Vec::new(),
            };
            let Some((mut compiled, choices)) =
                try_compile_stack_and_condition_effect(&conditional, ctx)?
            else {
                return Err(CardTextError::ParseError(
                    "failed to lower trailing-unless condition".to_string(),
                ));
            };
            let Some(lowered_conditional_effect) = compiled.pop() else {
                return Err(CardTextError::ParseError(
                    "trailing-unless condition lowered without an effect".to_string(),
                ));
            };
            let Some(lowered_conditional) =
                lowered_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
            else {
                return Err(CardTextError::ParseError(
                    "trailing-unless condition did not lower to a conditional".to_string(),
                ));
            };
            compiled.push(Effect::new(
                crate::effects::ConditionalEffect::new(
                    lowered_conditional.condition.clone(),
                    lowered_conditional.if_true.clone(),
                    lowered_conditional.if_false.clone(),
                )
                .with_surface(ironsmith_core::ConditionalSurface::TrailingUnless),
            ));
            (compiled, choices)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            let mut effective_if_true = if_true.clone();
            if let Some(antecedent) = predicate_object_filter_antecedent(predicate) {
                bind_condition_antecedent_in_effects(
                    &mut effective_if_true,
                    &antecedent,
                    ConditionAntecedentBinding::IncludeRandomWithCountObjects,
                );
            }
            bind_random_count_condition_antecedent_in_effects(&mut effective_if_true, predicate);
            if let Some(counter_type) = predicate_source_counter_antecedent(predicate) {
                bind_condition_counter_antecedent_in_effects(&mut effective_if_true, counter_type);
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
            let predicate_references_it = predicate_uses_implicit_object_reference(predicate)
                || predicate_references_tag(predicate, IT_TAG);

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
                    Effect::new(crate::effects::TargetOnlyEffect::new(choice)).tag(tag.clone()),
                );
                condition_reference_tag = Some(tag);
            }

            let original_last_tag = ctx.last_object_tag.clone();
            ctx.last_object_tag = condition_reference_tag.clone().or(saved_last_tag.clone());
            let original_source_object_antecedent = ctx.source_object_antecedent;
            if ctx.last_object_tag.is_none()
                && antecedent_choice.is_none()
                && predicate_uses_implicit_object_reference(predicate)
            {
                ctx.source_object_antecedent = true;
            }
            let condition =
                compile_condition_from_predicate_ast(predicate, ctx, &condition_reference_tag)?;
            ctx.last_object_tag = original_last_tag;
            ctx.source_object_antecedent = original_source_object_antecedent;

            let true_effects = if matches!(predicate, PredicateAst::ItIsSoulbondPaired)
                && let Some(reference_tag) = condition_reference_tag.as_deref()
            {
                rewrite_effects_tag_relation(
                    true_effects,
                    reference_tag,
                    TaggedOpbjectRelation::IsTaggedObject,
                    TaggedOpbjectRelation::SoulbondPartnerOfTagged,
                )
            } else {
                true_effects
            };

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

fn predicate_uses_implicit_object_reference(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::ItIsLandCard
        | PredicateAst::ItIsSoulbondPaired
        | PredicateAst::ItMatches(_)
        | PredicateAst::ItMatchedLastKnown(_)
        | PredicateAst::TargetMatches(_) => true,
        PredicateAst::Not(inner) => predicate_uses_implicit_object_reference(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_uses_implicit_object_reference(left)
                || predicate_uses_implicit_object_reference(right)
        }
        _ => false,
    }
}

pub(super) fn try_compile_attachment_and_setup_effect(
    _effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    Ok(None)
}
