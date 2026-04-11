use super::*;

pub(super) fn try_compile_continuous_and_modifier_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Monstrosity { amount } => {
            (vec![Effect::monstrosity(amount.clone())], Vec::new())
        }
        EffectAst::RemoveUpToAnyCounters {
            amount,
            target,
            counter_type,
            up_to,
        } => {
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            compile_tagged_effect_for_target(target, ctx, "counters", |spec| {
                let effect = if let Some(counter_type) = counter_type {
                    if *up_to {
                        Effect::remove_up_to_counters(*counter_type, amount.clone(), spec)
                    } else {
                        Effect::remove_counters(*counter_type, amount.clone(), spec)
                    }
                } else {
                    Effect::remove_up_to_any_counters(amount.clone(), spec)
                };
                Effect::with_id(id.0, effect)
            })?
        }
        EffectAst::MoveAllCounters { from, to } => {
            let (from_spec, mut choices) =
                resolve_target_spec_with_choices(from, &current_reference_env(ctx))?;
            let (to_spec, to_choices) =
                resolve_target_spec_with_choices(to, &current_reference_env(ctx))?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_all_counters(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            (vec![effect], choices)
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
        } => {
            let resolved_power = resolve_value_it_tag(power, &current_reference_env(ctx))?;
            let resolved_toughness = resolve_value_it_tag(toughness, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec,
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: resolved_power.clone(),
                        toughness: resolved_toughness.clone(),
                    },
                    duration.clone(),
                )
                .require_creature_target();
                if let Some(condition) = condition {
                    apply = apply.with_condition(condition.clone());
                }
                Effect::new(apply)
            })?
        }
        EffectAst::SwitchPowerToughness { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "switched_pt", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec(
                        spec,
                        crate::continuous::Modification::SwitchPowerToughness,
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })?
        }
        EffectAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_pt", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        })?,
        EffectAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            card_types,
            subtypes,
            colors,
            abilities,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "animated_creature", |spec| {
            let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddCardTypes(card_types.clone()),
                duration.clone(),
            )
            .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
                power: power.clone(),
                toughness: toughness.clone(),
                sublayer: crate::continuous::PtSublayer::Setting,
            })
            .resolve_set_pt_values_at_resolution();
            if let Some(colors) = colors {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::SetColors(*colors),
                );
            }
            if !subtypes.is_empty() {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                );
            }
            for ability in abilities {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::AddAbility(ability.clone()),
                );
            }
            Effect::new(apply)
        })?,
        EffectAst::AddCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddCardTypes(card_types.clone()),
                duration.clone(),
            ))
        })?,
        EffectAst::RemoveCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::RemoveCardTypes(card_types.clone()),
                duration.clone(),
            ))
        })?,
        EffectAst::AddSubtypes {
            target,
            subtypes,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "subtyped", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                duration.clone(),
            ))
        })?,
        EffectAst::SetBasePower {
            power,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_power", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPower {
                        value: power.clone(),
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        })?,
        EffectAst::SetColors {
            target,
            colors,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_colors", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::SetColors(*colors),
                duration.clone(),
            ))
        })?,
        EffectAst::MakeColorless { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "set_colorless", |spec| {
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::MakeColorless,
                    duration.clone(),
                ))
            })?
        }
        EffectAst::PumpForEach {
            power_per,
            toughness_per,
            target,
            count,
            duration,
        } => {
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::pump_for_each(
                    spec,
                    *power_per,
                    *toughness_per,
                    resolved_count.clone(),
                    duration.clone(),
                )
            })?
        }
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("pumped");
            let effect = Effect::new(
                crate::effects::ApplyContinuousEffect::new_runtime(
                    crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                    },
                    duration.clone(),
                )
                .lock_filter_at_resolution(),
            )
            .tag_all(tag.clone());
            ctx.last_object_tag = Some(tag);
            (vec![effect], Vec::new())
        }
        EffectAst::ScalePowerToughnessAll {
            filter,
            power,
            toughness,
            multiplier,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let scaled_stat = |value: Value| {
                if *multiplier == 1 {
                    value
                } else {
                    Value::Scaled(Box::new(value), *multiplier)
                }
            };
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        ChooseSpec::Iterated,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: if *power {
                                scaled_stat(Value::PowerOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                            toughness: if *toughness {
                                scaled_stat(Value::ToughnessOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
        } => {
            let id = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for pump clause".to_string())
            })?;
            let power_value = if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            };
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: power_value.clone(),
                            toughness: Value::Fixed(*toughness),
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })?
        }
        EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                modifications[0].clone(),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for modification in modifications.iter().skip(1) {
                apply = apply.with_additional_modification(modification.clone());
            }

            (vec![Effect::new(apply)], Vec::new())
        }
        EffectAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
        } => {
            let abilities = lower_granted_abilities_ast(abilities)?;
            if abilities.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove RemoveAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                crate::continuous::Modification::RemoveAbility(abilities[0].clone()),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for ability in abilities.iter().skip(1) {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::RemoveAbility(ability.clone()),
                );
            }

            (vec![Effect::new(apply)], Vec::new())
        }
        EffectAst::GrantAbilitiesChoiceAll {
            filter,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesChoiceAll with no abilities"
                        .to_string(),
                ));
            }
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let modes = modifications
                .iter()
                .map(|modification| EffectMode {
                    description: String::new(),
                    effects: vec![Effect::new(
                        crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                            modification.clone(),
                            duration.clone(),
                        )
                        .lock_filter_at_resolution(),
                    )],
                })
                .collect::<Vec<_>>();
            (vec![Effect::choose_one(modes)], Vec::new())
        }
        EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            let Some(first_modification) = modifications.first() else {
                return Ok(Some(compile_tagged_effect_for_target(
                    target,
                    ctx,
                    "granted",
                    |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
                )?));
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    first_modification.clone(),
                    duration.clone(),
                );

                for modification in modifications.iter().skip(1) {
                    apply = apply.with_additional_modification(modification.clone());
                }

                Effect::new(apply)
            })?
        }
        EffectAst::GrantToTarget {
            target,
            grantable,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
            Effect::grant(grantable.clone(), spec, *duration)
        })?,
        EffectAst::GrantBySpec {
            spec,
            player,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(&spec.filter, &current_reference_env(ctx))?;
            let player =
                resolve_non_target_player_filter(player.clone(), &current_reference_env(ctx))?;
            let mut resolved_spec = spec.clone();
            resolved_spec.filter = resolved_filter;
            (
                vec![Effect::grant_by_spec(resolved_spec, player, *duration)],
                Vec::new(),
            )
        }
        EffectAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        } => {
            let abilities = lower_granted_abilities_ast(abilities)?;
            let Some(first_ability) = abilities.first() else {
                return Ok(Some(compile_tagged_effect_for_target(
                    target,
                    ctx,
                    "granted",
                    |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
                )?));
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::RemoveAbility(first_ability.clone()),
                    duration.clone(),
                );

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::RemoveAbility(ability.clone()),
                    );
                }

                Effect::new(apply)
            })?
        }
        EffectAst::GrantAbilitiesChoiceToTarget {
            target,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Ok(Some(compile_tagged_effect_for_target(
                    target,
                    ctx,
                    "granted",
                    |spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)),
                )?));
            }

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let modes = abilities
                    .iter()
                    .zip(modifications.iter())
                    .map(|(ability, modification)| EffectMode {
                        description: granted_ability_mode_description(ability, &spec)
                            .unwrap_or_default(),
                        effects: vec![Effect::new(
                            crate::effects::ApplyContinuousEffect::with_spec(
                                spec.clone(),
                                modification.clone(),
                                duration.clone(),
                            ),
                        )],
                    })
                    .collect::<Vec<_>>();
                Effect::choose_one(modes)
            })?
        }
        EffectAst::Transform { target } => {
            compile_tagged_effect_for_target(target, ctx, "transformed", Effect::transform)?
        }
        EffectAst::Meld {
            result_name,
            enters_tapped,
            enters_attacking,
        } => (
            vec![Effect::new(
                crate::effects::MeldEffect::new(result_name.clone())
                    .enters_tapped(*enters_tapped)
                    .enters_attacking(*enters_attacking),
            )],
            Vec::new(),
        ),
        EffectAst::Convert { target } => {
            compile_tagged_effect_for_target(target, ctx, "converted", Effect::convert)?
        }
        EffectAst::Flip { target } => {
            compile_tagged_effect_for_target(target, ctx, "flipped", Effect::flip)?
        }
        EffectAst::GrantAbilityToSource { ability } => {
            let lowered = lower_parsed_ability(ability.clone())?;
            (
                vec![Effect::grant_object_ability_to_source(
                    lowered.into_runtime(),
                )],
                Vec::new(),
            )
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_player_turn_and_counter_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::RingTemptsYou { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::ring_tempts_player)?
        }
        EffectAst::VentureIntoDungeon {
            player,
            undercity_if_no_active,
        } => compile_player_effect_from_filter(*player, ctx, true, |filter| {
            if *undercity_if_no_active {
                Effect::venture_into_undercity_player(filter)
            } else {
                Effect::venture_into_dungeon_player(filter)
            }
        })?,
        EffectAst::BecomeMonarch { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::become_monarch_player)?
        }
        EffectAst::TakeInitiative { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::take_initiative_player)?
        }
        EffectAst::DoubleManaPool { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::double_mana_pool_player)?
        }
        EffectAst::ExchangeLifeTotals { player1, player2 } => {
            compile_exchange_life_totals_effect(*player1, *player2, ctx)?
        }
        EffectAst::SetLifeTotal { amount, player } => {
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                Effect::set_life_total_player(amount.clone(), filter)
            })?
        }
        EffectAst::SkipTurn { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::skip_turn_player)?
        }
        EffectAst::SkipCombatPhases { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_combat_phases_player,
        )?,
        EffectAst::SkipNextCombatPhaseThisTurn { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::skip_next_combat_phase_this_turn_player,
        )?,
        EffectAst::SkipDrawStep { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::skip_draw_step_player)?
        }
        EffectAst::Regenerate { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::regenerate(spec.clone(), crate::effect::Until::EndOfTurn),
                &spec,
                ctx,
                "regenerated",
            );
            (vec![effect], choices)
        }
        EffectAst::RegenerateAll { filter } => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::regenerate(
                ChooseSpec::all(filter.clone()),
                crate::effect::Until::EndOfTurn,
            ));
            (prelude, choices)
        }
        EffectAst::Mill { count, player } => compile_player_effect_with_generated_object_tag(
            *player,
            ctx,
            true,
            "milled",
            || Effect::mill(count.clone()),
            |filter| Effect::mill_player(count.clone(), filter),
        )?,
        EffectAst::PoisonCounters { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::poison_counters(count.clone()),
            |filter| Effect::poison_counters_player(count.clone(), filter),
        )?,
        EffectAst::EnergyCounters { count, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::energy_counters(count.clone()),
            |filter| Effect::energy_counters_player(count.clone(), filter),
        )?,
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}
