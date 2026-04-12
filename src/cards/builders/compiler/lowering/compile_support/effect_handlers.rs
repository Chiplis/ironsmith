use super::*;

pub(super) fn try_compile_timing_and_control_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Cant {
            restriction,
            duration,
            condition,
        } => {
            let restriction = resolve_restriction_it_tag(restriction, &current_reference_env(ctx))?;
            if let Some(condition) = condition {
                match &restriction {
                    crate::effect::Restriction::Untap(filter) => {
                        let apply = crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Filter(filter.clone()),
                            crate::continuous::Modification::DoesntUntap,
                            duration.clone(),
                        )
                        .with_condition(condition.clone())
                        .lock_filter_at_resolution();
                        (vec![Effect::new(apply)], Vec::new())
                    }
                    other => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported conditioned restriction: {other:?}"
                        )));
                    }
                }
            } else {
                (
                    vec![Effect::cant_until(restriction, duration.clone())],
                    Vec::new(),
                )
            }
        }
        EffectAst::PlayFromGraveyardUntilEot { player } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::grant_play_from_graveyard_until_eot(player_filter);
            (vec![effect], Vec::new())
        }
        EffectAst::AdditionalLandPlays {
            count,
            player,
            duration,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::additional_land_plays(
                resolve_value_it_tag(count, &current_reference_env(ctx))?,
                player_filter,
                duration.clone(),
            );
            (vec![effect], Vec::new())
        }
        EffectAst::ReduceNextSpellCostThisTurn {
            player,
            filter,
            reduction,
        } => {
            let mut player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            (
                vec![Effect::new(
                    crate::effects::GrantNextSpellCostReductionEffect::new(
                        player_filter,
                        resolved_filter,
                        reduction.clone(),
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::GrantNextSpellAbilityThisTurn {
            player,
            filter,
            ability,
        } => {
            let mut player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            let mut lowered = lower_granted_abilities_ast(std::slice::from_ref(ability))?;
            let Some(ability) = lowered.pop() else {
                return Err(CardTextError::ParseError(
                    "temporary next-spell grant did not lower to a static ability".to_string(),
                ));
            };
            (
                vec![Effect::grant_next_spell_ability_this_turn(
                    player_filter,
                    resolved_filter,
                    ability,
                )],
                Vec::new(),
            )
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            let mut effects = vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter.clone(),
                crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                *allow_land,
                *allow_any_color_for_cast,
            ))];
            if *without_paying_mana_cost {
                effects.push(Effect::new(
                    crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                        resolved_tag,
                        player_filter,
                    ),
                ));
            }
            (effects, Vec::new())
        }
        EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag,
            player,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            (
                vec![Effect::new(
                    crate::effects::GrantTaggedSpellLifeCostByManaValueEffect::new(
                        resolved_tag,
                        player_filter,
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::GrantPlayTaggedUntilYourNextTurn {
            tag,
            player,
            allow_land,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            (
                vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    resolved_tag,
                    player_filter,
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd,
                    *allow_land,
                    false,
                ))],
                Vec::new(),
            )
        }
        EffectAst::CastTagged {
            tag,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            cost_reduction,
        } => {
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            let effect = Effect::cast_tagged(
                resolved_tag,
                *allow_land,
                *as_copy,
                *without_paying_mana_cost,
                cost_reduction.clone(),
            );
            (vec![effect], Vec::new())
        }
        EffectAst::RegisterZoneReplacement {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            duration,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
            };
            let effect = Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
                spec,
                *from_zone,
                *to_zone,
                *replacement_zone,
                mode,
            ));
            (vec![effect], choices)
        }
        EffectAst::ExileInsteadOfGraveyardThisTurn { player } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let effect = Effect::exile_instead_of_graveyard_this_turn(player_filter);
            (vec![effect], Vec::new())
        }
        EffectAst::GainControl {
            target,
            player,
            duration,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (controller, mut controller_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            choices.append(&mut controller_choices);
            let runtime_modification = if matches!(controller, PlayerFilter::You) {
                crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            } else {
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    controller,
                )
            };
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec.clone(),
                    runtime_modification,
                    duration.clone(),
                )),
                &spec,
                ctx,
                "controlled",
            );
            (vec![effect], choices)
        }
        EffectAst::ControlPlayer { player, duration } => {
            let (start, duration) = match duration {
                ControlDurationAst::UntilEndOfTurn => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::DuringNextTurn => (
                    crate::game_state::PlayerControlStart::NextTurn,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::Forever => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::Forever,
                ),
                ControlDurationAst::AsLongAsYouControlSource => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilSourceLeaves,
                ),
            };

            let mut choices = Vec::new();
            if let PlayerFilter::Target(inner) = player {
                let spec = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
                choices.push(spec);
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else {
                ctx.last_player_filter = Some(player.clone());
            }

            let effect = Effect::control_player(player.clone(), start, duration);
            (vec![effect], choices)
        }
        EffectAst::ControlCombatChoicesThisTurn {
            attackers,
            blockers,
        } => {
            let effect = Effect::control_combat_choices_this_turn(*attackers, *blockers);
            (vec![effect], Vec::new())
        }
        EffectAst::ExtraTurnAfterTurn { player, anchor } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = match anchor {
                ExtraTurnAnchorAst::CurrentTurn => Effect::extra_turn_player(player_filter),
                ExtraTurnAnchorAst::ReferencedTurn => {
                    Effect::extra_turn_after_next_turn_player(player_filter)
                }
            };
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextEndStep { player, effects } => {
            let (delayed_effects, choices) = compile_effects_preserving_last_effect(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::DelayedUntilNextUpkeep { player, effects } => {
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_upkeep(player_filter),
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
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_draw_step(player_filter),
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
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (delayed_effects, nested_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(player_filter),
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
                Trigger::end_of_combat(),
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
                            Trigger::is_dealt_damage(ChooseSpec::Source),
                            delayed_effects,
                            false,
                            watched_tag,
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                compile_trigger_spec(TriggerSpec::IsDealtDamage(resolved_filter)),
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
                            Trigger::this_dies(),
                            delayed_effects,
                            false,
                            watched_tag,
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                compile_trigger_spec(TriggerSpec::PutIntoGraveyard(
                                    resolved_filter,
                                )),
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
                            compile_trigger_spec(TriggerSpec::PutIntoGraveyardFromZone {
                                filter: resolved_filter,
                                from: *from,
                            }),
                            delayed_effects,
                            false,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
                _ => {
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            compile_trigger_spec(trigger.clone()),
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
                Trigger::this_dies(),
                delayed_effects,
                true,
                target_tag,
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
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Destroy { target } => {
            compile_tagged_effect_for_target(target, ctx, "destroyed", |spec| {
                Effect::new(crate::effects::DestroyEffect::with_spec(spec))
            })?
        }
        EffectAst::DestroyNoRegeneration { target } => {
            compile_tagged_effect_for_target(target, ctx, "destroyed", |spec| {
                Effect::new(crate::effects::DestroyNoRegenerationEffect::with_spec(spec))
            })?
        }
        EffectAst::DestroyAllAttachedTo { filter, target } => {
            let (target_spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut prelude = Vec::new();
            let mut choices = choices;
            let target_tag = if let ChooseSpec::Tagged(tag) = &target_spec {
                tag.as_str().to_string()
            } else {
                if !choose_spec_targets_object(&target_spec) || !target_spec.is_target() {
                    return Err(CardTextError::ParseError(
                        "destroy-attached target must be an object target or tagged object"
                            .to_string(),
                    ));
                }
                let tag = ctx.next_tag("attachment_target");
                prelude.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                        .tag(tag.clone()),
                );
                tag
            };
            ctx.last_object_tag = Some(target_tag.clone());

            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            resolved_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(target_tag.as_str()),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                });

            let (mut filter_prelude, filter_choices) =
                target_context_prelude_for_filter(&resolved_filter);
            for choice in filter_choices {
                push_choice(&mut choices, choice);
            }

            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.append(&mut filter_prelude);
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::DestroyAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::DestroyAllNoRegeneration { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = Effect::new(crate::effects::DestroyNoRegenerationEffect::all(
                resolved_filter,
            ));
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            use crate::effect::EffectMode;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];
            let auto_tag = if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                ctx.last_object_tag = Some(tag.clone());
                Some(tag)
            } else {
                None
            };
            for (_name, color) in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!("Destroy all {}.", filter.description());
                let mut effect = Effect::destroy_all(filter);
                if let Some(tag) = &auto_tag {
                    effect = effect.tag(tag.clone());
                }
                modes.push(EffectMode {
                    description,
                    effects: vec![effect],
                });
            }
            prelude.push(Effect::choose_one(modes));
            (prelude, choices)
        }
        EffectAst::DestroyAllOfChosenColorNoRegeneration { filter } => {
            use crate::effect::EffectMode;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];
            let auto_tag = if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                ctx.last_object_tag = Some(tag.clone());
                Some(tag)
            } else {
                None
            };
            for (_name, color) in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!(
                    "Destroy all {}. They can't be regenerated.",
                    filter.description()
                );
                let mut effect =
                    Effect::new(crate::effects::DestroyNoRegenerationEffect::all(filter));
                if let Some(tag) = &auto_tag {
                    effect = effect.tag(tag.clone());
                }
                modes.push(EffectMode {
                    description,
                    effects: vec![effect],
                });
            }
            prelude.push(Effect::choose_one(modes));
            (prelude, choices)
        }
        EffectAst::ExileAll { filter, face_down } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if let Some(player_filter) = infer_player_filter_from_object_filter(&resolved_filter) {
                ctx.last_player_filter = Some(player_filter);
            }
            let keep_last_object_tag =
                resolved_filter.tagged_constraints.iter().any(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    )
                });
            let mut effect = Effect::new(
                crate::effects::ExileEffect::all(resolved_filter).with_face_down(*face_down),
            );
            if ctx.auto_tag_object_targets && !keep_last_object_tag {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            (prelude, choices)
        }
        EffectAst::Exile { target, face_down } => {
            if let Some(compiled) = lower_hand_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_counted_non_target_exile_target(target, *face_down, ctx)?
            {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_single_non_target_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = if spec.count().is_single() && !*face_down {
                Effect::move_to_zone(spec.clone(), Zone::Exile, true)
            } else {
                Effect::new(
                    crate::effects::ExileEffect::with_spec(spec.clone()).with_face_down(*face_down),
                )
            };
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::ExileWhenSourceLeaves { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let ChooseSpec::Tagged(tag) = spec.base() else {
                return Err(CardTextError::ParseError(
                    "cannot compile 'exile ... when this source leaves' without tagged context"
                        .to_string(),
                ));
            };
            let effect = Effect::new(crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
                tag.clone(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::SacrificeSourceWhenLeaves { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let ChooseSpec::Tagged(tag) = spec.base() else {
                return Err(CardTextError::ParseError(
                    "cannot compile 'sacrifice this source when ... leaves' without tagged context"
                        .to_string(),
                ));
            };
            let effect = Effect::new(
                crate::effects::ScheduleEffectsWhenTaggedLeavesEffect::new(
                    tag.clone(),
                    vec![Effect::sacrifice_source()],
                    PlayerFilter::You,
                )
                .with_current_source_as_ability_source(),
            );
            (vec![effect], choices)
        }
        EffectAst::ExileUntilSourceLeaves { target, face_down } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::new(
                crate::effects::ExileUntilEffect::source_leaves(spec.clone())
                    .with_face_down(*face_down),
            );
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            (vec![effect], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
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
        EffectAst::CopySpell {
            target,
            count,
            player,
            may_choose_new_targets,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let copy_effect = Effect::with_id(
                id.0,
                Effect::new(crate::effects::CopySpellEffect::new_for_player(
                    spec.clone(),
                    count.clone(),
                    player_filter.clone(),
                )),
            );
            let retarget_effect = if *may_choose_new_targets {
                Some(Effect::may_choose_new_targets_player(
                    id,
                    player_filter.clone(),
                ))
            } else {
                None
            };
            let mut compiled = vec![copy_effect];
            if let Some(retarget) = retarget_effect {
                compiled.push(retarget);
            }
            (compiled, choices)
        }
        EffectAst::RetargetStackObject {
            target,
            mode,
            chooser,
            require_change,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (chooser_filter, chooser_choices) =
                resolve_effect_player_filter(*chooser, ctx, true, true, true)?;
            for choice in chooser_choices {
                push_choice(&mut choices, choice);
            }

            let mut effect = crate::effects::RetargetStackObjectEffect::new(spec.clone())
                .with_chooser(chooser_filter);

            if *require_change {
                effect = effect.require_change();
            }

            let compiled_mode = match mode {
                RetargetModeAst::All => crate::effects::RetargetMode::All,
                RetargetModeAst::OneToFixed { target: fixed } => {
                    let (fixed_spec, fixed_choices) =
                        resolve_target_spec_with_choices(fixed, &current_reference_env(ctx))?;
                    for choice in fixed_choices {
                        push_choice(&mut choices, choice);
                    }
                    crate::effects::RetargetMode::OneToFixed(fixed_spec)
                }
            };
            effect = effect.with_mode(compiled_mode);

            let effect = tag_object_target_effect(Effect::new(effect), &spec, ctx, "retargeted");
            (vec![effect], choices)
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => {
            let saved_last_tag = ctx.last_object_tag.clone();
            let (true_effects, true_choices) = compile_effects(if_true, ctx)?;
            let true_last_tag = ctx.last_object_tag.clone();
            ctx.last_object_tag = saved_last_tag.clone();
            let (false_effects, false_choices) = compile_effects(if_false, ctx)?;
            let predicate_references_it = matches!(
                predicate,
                PredicateAst::ItIsLandCard
                    | PredicateAst::ItIsSoulbondPaired
                    | PredicateAst::ItMatches(_)
            ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == IT_TAG)
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
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Enchant { filter } => {
            let spec = filter.target_spec();
            let effect = Effect::attach_to(spec.clone());
            (vec![effect], vec![spec])
        }
        EffectAst::Attach { object, target } => {
            let (objects, object_choices) =
                resolve_attach_object_spec(object, &current_reference_env(ctx))?;
            let (target, target_choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut choices = Vec::new();
            for choice in object_choices {
                push_choice(&mut choices, choice);
            }
            for choice in target_choices {
                push_choice(&mut choices, choice);
            }
            (vec![Effect::attach_objects(objects, target)], choices)
        }
        EffectAst::PutSticker { target, action } => match target {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() =>
            {
                let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
                let tag = ctx.next_tag("stickered");
                let tag_key = TagKey::from(tag.as_str());
                let choose_effect = crate::effects::ChooseObjectsEffect::new(
                    resolved_filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag_key.clone(),
                )
                .in_zone(choice_zone);
                ctx.last_object_tag = Some(tag.as_str().to_string());
                (
                    vec![
                        Effect::new(choose_effect),
                        Effect::put_sticker(ChooseSpec::Tagged(tag_key), *action),
                    ],
                    Vec::new(),
                )
            }
            _ => compile_effect_for_target(target, ctx, |spec| Effect::put_sticker(spec, *action))?,
        },
        EffectAst::Investigate { count } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            (vec![Effect::investigate(count)], Vec::new())
        }
        EffectAst::Amass { subtype, amount } => {
            let mut effect = Effect::amass(*subtype, *amount);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("amassed");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], Vec::new())
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}
