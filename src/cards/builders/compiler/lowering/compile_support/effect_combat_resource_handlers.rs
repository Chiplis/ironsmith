use super::*;

pub(super) fn try_compile_combat_and_damage_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::DealDamage { amount, target } => {
            let mut resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
                && !ctx.iterated_player
            {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut resolved_amount,
                    &PlayerFilter::Target(Box::new(filter.clone())),
                );
            }
            let (effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    Effect::deal_damage(resolved_amount.clone(), spec)
                })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if matches!(
                target,
                TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
            ) {
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }
            (effects, choices)
        }
        EffectAst::DealDistributedDamage { amount, target } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                Effect::new(crate::effects::DealDistributedDamageEffect::new(
                    resolved_amount.clone(),
                    spec,
                ))
            })?
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            let (source_spec, mut choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            let mut damage_target_spec = if source == target {
                source_spec.clone()
            } else {
                let (target_spec, target_choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                target_spec
            };

            let mut effects = Vec::new();
            let mut damage_source_spec = source_spec.clone();
            let per_target_source_spec = if source == target {
                ChooseSpec::Iterated
            } else {
                source_spec.clone()
            };

            if source_spec.is_target() {
                let source_tag = ctx.next_tag("damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                damage_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            if !damage_target_spec.is_target()
                && let ChooseSpec::Object(filter) | ChooseSpec::All(filter) =
                    damage_target_spec.base()
            {
                let mut per_target_damage =
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        per_target_source_spec.clone(),
                        Effect::deal_damage(
                            Value::PowerOf(Box::new(per_target_source_spec.clone())),
                            ChooseSpec::Iterated,
                        ),
                    ));
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("damaged");
                    ctx.last_object_tag = Some(tag.clone());
                    per_target_damage = per_target_damage.tag(tag);
                }
                effects.push(Effect::for_each(filter.clone(), vec![per_target_damage]));
            } else {
                let damage_effect = tag_object_target_effect(
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        damage_source_spec.clone(),
                        Effect::deal_damage(
                            Value::PowerOf(Box::new(damage_source_spec.clone())),
                            damage_target_spec.clone(),
                        ),
                    )),
                    &damage_target_spec,
                    ctx,
                    "damaged",
                );
                effects.push(damage_effect);
            }

            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if matches!(
                target,
                TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
            ) {
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }

            (effects, choices)
        }
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            let (spec1, mut choices) =
                resolve_target_spec_with_choices(creature1, &current_reference_env(ctx))?;
            let (spec2, other_choices) =
                resolve_target_spec_with_choices(creature2, &current_reference_env(ctx))?;
            for choice in other_choices {
                push_choice(&mut choices, choice);
            }
            let effect = Effect::fight(spec1.clone(), spec2.clone());
            (vec![effect], choices)
        }
        EffectAst::FightIterated { creature2 } => {
            let (spec2, choices) =
                resolve_target_spec_with_choices(creature2, &current_reference_env(ctx))?;
            let effect = Effect::fight(ChooseSpec::Iterated, spec2);
            (vec![effect], choices)
        }
        EffectAst::Clash { opponent } => match opponent {
            ClashOpponentAst::Opponent => (
                vec![Effect::new(
                    crate::effects::ClashEffect::against_any_opponent(),
                )],
                Vec::new(),
            ),
            ClashOpponentAst::TargetOpponent => {
                let choice = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Opponent));
                (
                    vec![Effect::new(
                        crate::effects::ClashEffect::against_target_opponent(),
                    )],
                    vec![choice],
                )
            }
            ClashOpponentAst::DefendingPlayer => (
                vec![Effect::new(
                    crate::effects::ClashEffect::against_defending_player(),
                )],
                Vec::new(),
            ),
        },
        EffectAst::DealDamageEach { amount, filter } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("damaged");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::deal_damage(resolved_amount, ChooseSpec::Iterated).tag(tag)],
            );
            (vec![effect], Vec::new())
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_board_state_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    use crate::effect::EffectMode;

    let compiled = match effect {
        EffectAst::PutCounters {
            counter_type,
            count,
            target,
            target_count,
            distributed,
        } => {
            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }
            let mut put_counters =
                crate::effects::PutCountersEffect::new(*counter_type, count.clone(), spec.clone());
            if let Some(target_count) = target_count {
                put_counters = put_counters.with_target_count(*target_count);
            }
            if *distributed {
                put_counters = put_counters.with_distributed(true);
            }
            let effect =
                tag_object_target_effect(Effect::new(put_counters), &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            (vec![effect], choices)
        }
        EffectAst::PutOrRemoveCounters {
            put_counter_type,
            put_count,
            remove_counter_type,
            remove_count,
            put_mode_text,
            remove_mode_text,
            target,
            target_count,
        } => {
            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }

            let put_effect =
                Effect::put_counters(*put_counter_type, put_count.clone(), spec.clone());
            let remove_effect =
                Effect::remove_counters(*remove_counter_type, remove_count.clone(), spec.clone());

            let effect = Effect::choose_one(vec![
                EffectMode {
                    description: put_mode_text.clone(),
                    effects: vec![put_effect],
                },
                EffectMode {
                    description: remove_mode_text.clone(),
                    effects: vec![remove_effect],
                },
            ]);

            let effect = tag_object_target_effect(effect, &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            (vec![effect], choices)
        }
        EffectAst::ForEachCounterKindPutOrRemove { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            (
                vec![Effect::new(
                    crate::effects::ForEachCounterKindPutOrRemoveEffect::new(spec),
                )],
                choices,
            )
        }
        EffectAst::PutCountersAll {
            counter_type,
            count,
            filter,
        } => {
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::put_counters(
                    *counter_type,
                    count.clone(),
                    ChooseSpec::Iterated,
                )],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::RemoveCountersAll {
            amount,
            filter,
            counter_type,
            up_to,
        } => {
            let iterated = ChooseSpec::Iterated;
            let inner = if let Some(counter_type) = counter_type {
                if *up_to {
                    Effect::remove_up_to_counters(*counter_type, amount.clone(), iterated.clone())
                } else {
                    Effect::remove_counters(*counter_type, amount.clone(), iterated.clone())
                }
            } else {
                Effect::remove_up_to_any_counters(amount.clone(), iterated.clone())
            };
            let effect = Effect::for_each(filter.clone(), vec![inner]);
            (vec![effect], Vec::new())
        }
        EffectAst::DoubleCountersOnEach {
            counter_type,
            filter,
        } => {
            let iterated = ChooseSpec::Iterated;
            let count = Value::CountersOn(Box::new(iterated.clone()), Some(*counter_type));
            let effect = Effect::for_each(
                filter.clone(),
                vec![Effect::put_counters(*counter_type, count, iterated)],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::Proliferate { count } => (vec![Effect::proliferate(count.clone())], Vec::new()),
        EffectAst::Tap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            (vec![effect], choices)
        }
        EffectAst::TapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::tap_all(resolved_filter));
            (prelude, choices)
        }
        EffectAst::Untap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            (vec![effect], choices)
        }
        EffectAst::PhaseOut { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::phase_out(spec.clone())
            } else {
                Effect::new(crate::effects::PhaseOutEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_out");
            (vec![effect], choices)
        }
        EffectAst::RemoveFromCombat { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
                    spec.clone(),
                )),
                &spec,
                ctx,
                "removed_from_combat",
            );
            (vec![effect], choices)
        }
        EffectAst::TapOrUntap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let tap_effect = Effect::tap(spec.clone());
            let untap_effect = Effect::untap(spec.clone());
            let modes = vec![
                EffectMode {
                    description: "Tap".to_string(),
                    effects: vec![tap_effect],
                },
                EffectMode {
                    description: "Untap".to_string(),
                    effects: vec![untap_effect],
                },
            ];
            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "tap_or_untap");
            (vec![effect], choices)
        }
        EffectAst::TapOrUntapAll {
            tap_filter,
            untap_filter,
        } => {
            let resolved_tap = resolve_it_tag(tap_filter, &current_reference_env(ctx))?;
            let resolved_untap = resolve_it_tag(untap_filter, &current_reference_env(ctx))?;
            let (mut prelude, mut choices) = target_context_prelude_for_filter(&resolved_tap);
            let (_, untap_choices) = target_context_prelude_for_filter(&resolved_untap);
            for choice in untap_choices {
                push_choice(&mut choices, choice);
            }
            let modes = vec![
                EffectMode {
                    description: "Tap".to_string(),
                    effects: vec![Effect::tap_all(resolved_tap)],
                },
                EffectMode {
                    description: "Untap".to_string(),
                    effects: vec![Effect::untap_all(resolved_untap)],
                },
            ];
            prelude.push(Effect::choose_one(modes));
            (prelude, choices)
        }
        EffectAst::UntapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::untap_all(resolved_filter));
            (prelude, choices)
        }
        EffectAst::GrantProtectionChoice {
            target,
            allow_colorless,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut modes = Vec::new();
            if *allow_colorless {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Colorless);
                modes.push(EffectMode {
                    description: "Colorless".to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];

            for (name, color) in colors {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Color(
                    ColorSet::from(color),
                ));
                modes.push(EffectMode {
                    description: name.to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }

            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "protected");
            (vec![effect], choices)
        }
        EffectAst::Earthbend { counters } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::EarthbendEffect::new(
                    spec.clone(),
                    *counters,
                )),
                &spec,
                ctx,
                "earthbend",
            );
            (vec![effect], vec![spec])
        }
        EffectAst::Behold { subtype, count } => {
            (vec![Effect::behold(*subtype, *count)], Vec::new())
        }
        EffectAst::Explore { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect =
                tag_object_target_effect(Effect::explore(spec.clone()), &spec, ctx, "explored");
            (vec![effect], choices)
        }
        EffectAst::OpenAttraction => (vec![Effect::open_attraction()], Vec::new()),
        EffectAst::ManifestTopCardOfLibrary { player } => (
            vec![Effect::manifest_top_card_of_library(
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?,
            )],
            Vec::new(),
        ),
        EffectAst::ManifestDread => (vec![Effect::manifest_dread()], Vec::new()),
        EffectAst::Populate {
            count,
            enters_tapped,
            enters_attacking,
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
        } => {
            let mut effect = Effect::new(
                crate::effects::PopulateEffect::new(count.clone())
                    .enters_tapped(*enters_tapped)
                    .attacking(*enters_attacking)
                    .haste(*has_haste)
                    .sacrifice_at_next_end_step(*sacrifice_at_next_end_step)
                    .exile_at_next_end_step(*exile_at_next_end_step)
                    .exile_at_end_of_combat(*exile_at_end_of_combat)
                    .sacrifice_at_end_of_combat(*sacrifice_at_end_of_combat),
            );
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], Vec::new())
        }
        EffectAst::Bolster { amount } => (vec![Effect::bolster(*amount)], Vec::new()),
        EffectAst::Support { amount } => (vec![Effect::support(*amount)], Vec::new()),
        EffectAst::Adapt { amount } => (vec![Effect::adapt(*amount)], Vec::new()),
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_player_resource_and_choice_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Draw { count, player } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::draw(count.clone()),
                |filter| Effect::target_draws(count.clone(), filter),
            )?
        }
        EffectAst::DrawForEachTaggedMatching {
            player,
            tag,
            filter,
        } => {
            let resolved_player =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            (
                vec![Effect::new(
                    crate::effects::DrawForEachTaggedMatchingEffect::new(
                        resolved_player,
                        resolved_tag,
                        resolved_filter,
                    ),
                )],
                Vec::new(),
            )
        }
        EffectAst::Counter { target } => {
            compile_tagged_effect_for_target(target, ctx, "countered", Effect::counter)?
        }
        EffectAst::CounterUnlessPays {
            target,
            mana,
            life,
            additional_generic,
        } => {
            let additional_generic = additional_generic
                .as_ref()
                .map(|value| resolve_value_it_tag(value, &current_reference_env(ctx)))
                .transpose()?;
            compile_tagged_effect_for_target(target, ctx, "countered", |spec| {
                Effect::counter_unless_pays_with_life_and_additional(
                    spec,
                    mana.clone(),
                    life.clone(),
                    additional_generic.clone(),
                )
            })?
        }
        EffectAst::LoseLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::lose_life(amount.clone()),
                |filter| Effect::lose_life_player(amount.clone(), filter),
            )?
        }
        EffectAst::GainLife { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::gain_life(amount.clone()),
                |filter| Effect::gain_life_player(amount.clone(), ChooseSpec::Player(filter)),
            )?
        }
        EffectAst::CreateEmblem { player, text } => {
            let emblem = compile_emblem_description_from_text(text)?;
            let (filter, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = if matches!(filter, PlayerFilter::You) {
                Effect::create_emblem(emblem)
            } else {
                Effect::for_players(filter, vec![Effect::create_emblem(emblem)])
            };
            (vec![effect], choices)
        }
        EffectAst::LoseGame { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::lose_the_game,
            Effect::lose_the_game_player,
        )?,
        EffectAst::WinGame { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::win_the_game,
            Effect::win_the_game_player,
        )?,
        EffectAst::ExileTopOfLibrary {
            count,
            player,
            tags,
            accumulated_tags,
        } => {
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut effect =
                crate::effects::ExileTopOfLibraryEffect::new(resolved_count, player_filter.clone());
            for tag in tags {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                effect = effect.tag_moved(resolved_tag);
            }
            for tag in accumulated_tags {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                effect = effect.append_tagged(resolved_tag);
            }
            if let Some(tag) = tags.first() {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            }
            ctx.last_player_filter = Some(player_filter);
            (vec![Effect::new(effect)], choices)
        }
        EffectAst::RearrangeLookedCardsInLibrary { tag, player, count } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            (
                vec![Effect::rearrange_looked_cards_in_library(
                    resolved_tag,
                    player_filter,
                    *count,
                )],
                choices,
            )
        }
        EffectAst::SearchLibrarySlotsToHand {
            slots,
            player,
            reveal,
            progress_tag,
        } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let refs = current_reference_env(ctx);
            let resolved_slots = slots
                .iter()
                .map(|slot| {
                    let resolved_filter = resolve_it_tag(&slot.filter, &refs)?;
                    Ok(if slot.optional {
                        crate::effects::SearchLibrarySlot::optional(resolved_filter)
                    } else {
                        crate::effects::SearchLibrarySlot::required(resolved_filter)
                    })
                })
                .collect::<Result<Vec<_>, CardTextError>>()?;
            let resolved_tag = resolve_it_tag_key(progress_tag, &refs)?;
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            (
                vec![Effect::search_library_slots_to_hand(
                    resolved_slots,
                    player_filter,
                    *reveal,
                    resolved_tag,
                )],
                choices,
            )
        }
        EffectAst::MayMoveToZone {
            target,
            zone,
            player,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (decider, player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            for choice in player_choices {
                push_choice(&mut choices, choice);
            }
            (
                vec![Effect::may_move_to_zone(spec, *zone, decider)],
                choices,
            )
        }
        EffectAst::PreventAllCombatDamage { duration } => (
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        ),
        EffectAst::PreventAllCombatDamageFromSource { duration, source } => {
            compile_effect_for_target(source, ctx, |spec| {
                Effect::prevent_all_combat_damage_from(spec, duration.clone())
            })?
        }
        EffectAst::PreventAllCombatDamageToPlayers { duration } => (
            vec![Effect::prevent_all_combat_damage_to_players(
                duration.clone(),
            )],
            Vec::new(),
        ),
        EffectAst::PreventAllCombatDamageToYou { duration } => (
            vec![Effect::prevent_all_combat_damage_to_you(duration.clone())],
            Vec::new(),
        ),
        EffectAst::PreventDamage {
            amount,
            target,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::prevent_damage(amount.clone(), spec, duration.clone())
            })?
        }
        EffectAst::PreventAllDamageToTarget { target, duration } => {
            if let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                (
                    vec![Effect::prevent_all_damage_to(
                        resolved_filter,
                        duration.clone(),
                    )],
                    Vec::new(),
                )
            } else {
                compile_effect_for_target(target, ctx, |spec| {
                    Effect::prevent_all_damage_to_target(spec, duration.clone())
                })?
            }
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount,
            target,
            duration,
            counter_type,
        } => {
            let follow_up = vec![Effect::put_counters(
                *counter_type,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::AnyTarget,
            )];
            match amount {
                Some(amount) => {
                    let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
                    compile_effect_for_target(target, ctx, |spec| {
                        Effect::new(
                            crate::effects::PreventDamageEffect::new(
                                amount.clone(),
                                spec,
                                duration.clone(),
                            )
                            .with_follow_up_effects(follow_up.clone()),
                        )
                    })?
                }
                None => compile_effect_for_target(target, ctx, |spec| {
                    Effect::new(
                        crate::effects::PreventAllDamageToTargetEffect::new(spec, duration.clone())
                            .with_follow_up_effects(follow_up.clone()),
                    )
                })?,
            }
        }
        EffectAst::PreventNextTimeDamage { source, target } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::PreventNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::PreventNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let target_spec = match target {
                PreventNextTimeDamageTargetAst::AnyTarget => {
                    crate::effects::PreventNextTimeDamageTarget::AnyTarget
                }
                PreventNextTimeDamageTargetAst::You => {
                    crate::effects::PreventNextTimeDamageTarget::You
                }
            };
            (
                vec![Effect::new(
                    crate::effects::PreventNextTimeDamageEffect::new(source_spec, target_spec),
                )],
                Vec::new(),
            )
        }
        EffectAst::RedirectNextDamageFromSourceToTarget { amount, target } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::RedirectNextDamageToTargetEffect::new(
                    amount.clone(),
                    spec,
                ))
            })?
        }
        EffectAst::RedirectNextTimeDamageToSource { source, target } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::RedirectNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::RedirectNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::RedirectNextTimeDamageToSourceEffect::new(
                    source_spec.clone(),
                    spec,
                ))
            })?
        }
        EffectAst::PreventDamageEach {
            amount,
            filter,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let effect = Effect::for_each(
                filter,
                vec![Effect::prevent_damage(
                    amount,
                    ChooseSpec::Iterated,
                    duration.clone(),
                )],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::AddMana { mana, player } => compile_player_effect(
            *player,
            ctx,
            true,
            || Effect::add_mana(mana.clone()),
            |filter| Effect::add_mana_player(mana.clone(), filter),
        )?,
        EffectAst::AddManaScaled {
            mana,
            amount,
            player,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                    mana.clone(),
                    amount.clone(),
                    filter,
                ))
            })?
        }
        EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || {
                    if let Some(colors) = available_colors.clone() {
                        Effect::add_mana_of_any_color_restricted(amount.clone(), colors)
                    } else {
                        Effect::add_mana_of_any_color(amount.clone())
                    }
                },
                |filter| {
                    if let Some(colors) = available_colors.clone() {
                        Effect::add_mana_of_any_color_restricted_player(
                            amount.clone(),
                            filter,
                            colors,
                        )
                    } else {
                        Effect::add_mana_of_any_color_player(amount.clone(), filter)
                    }
                },
            )?
        }
        EffectAst::AddManaAnyOneColor { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::add_mana_of_any_one_color(amount.clone()),
                |filter| Effect::add_mana_of_any_one_color_player(amount.clone(), filter),
            )?
        }
        EffectAst::AddManaChosenColor {
            amount,
            player,
            fixed_option,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                if let Some(fixed) = fixed_option {
                    Effect::new(
                        crate::effects::mana::AddManaOfChosenColorEffect::with_fixed_option(
                            amount.clone(),
                            filter,
                            *fixed,
                        ),
                    )
                } else {
                    Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
                        amount.clone(),
                        filter,
                    ))
                }
            })?
        }
        EffectAst::AddManaFromLandCouldProduce {
            amount,
            player,
            land_filter,
            allow_colorless,
            same_type,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect_from_filter(*player, ctx, true, |filter| {
                Effect::add_mana_of_land_produced_types_player(
                    amount.clone(),
                    filter,
                    land_filter.clone(),
                    *allow_colorless,
                    *same_type,
                )
            })?
        }
        EffectAst::AddManaCommanderIdentity { amount, player } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_player_effect(
                *player,
                ctx,
                true,
                || Effect::add_mana_from_commander_color_identity(amount.clone()),
                |filter| {
                    Effect::add_mana_from_commander_color_identity_player(amount.clone(), filter)
                },
            )?
        }
        EffectAst::AddManaImprintedColors => (
            vec![Effect::new(
                crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
            )],
            Vec::new(),
        ),
        EffectAst::Scry { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::scry(count.clone()),
            |filter| Effect::scry_player(count.clone(), filter),
        )?,
        EffectAst::Fateseal { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::fateseal(count.clone()),
            |filter| Effect::fateseal_player(count.clone(), filter),
        )?,
        EffectAst::Discover { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::discover(count.clone()),
            |filter| Effect::discover_player(count.clone(), filter),
        )?,
        EffectAst::ConsultTopOfLibrary {
            player,
            mode,
            filter,
            stop_rule,
            all_tag,
            match_tag,
        } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_all_tag = resolve_it_tag_key(all_tag, &current_reference_env(ctx))?;
            let resolved_match_tag = resolve_it_tag_key(match_tag, &current_reference_env(ctx))?;
            let resolved_stop_rule = match stop_rule {
                crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch => {
                    crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                }
                crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value) => {
                    crate::effects::ConsultTopOfLibraryStopRule::MatchCount(resolve_value_it_tag(
                        value,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let resolved_mode = match mode {
                crate::cards::builders::LibraryConsultModeAst::Reveal => {
                    crate::effects::consult_helpers::LibraryConsultMode::Reveal
                }
                crate::cards::builders::LibraryConsultModeAst::Exile => {
                    crate::effects::consult_helpers::LibraryConsultMode::Exile
                }
            };
            ctx.last_object_tag = Some(resolved_match_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            (
                vec![Effect::consult_top_of_library(
                    player_filter,
                    resolved_mode,
                    resolved_filter,
                    resolved_stop_rule,
                    resolved_all_tag,
                    resolved_match_tag,
                )],
                choices,
            )
        }
        EffectAst::PutTaggedRemainderOnBottomOfLibrary {
            tag,
            keep_tagged,
            order,
            player,
        } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_keep_tagged = keep_tagged
                .as_ref()
                .map(|tag| resolve_it_tag_key(tag, &current_reference_env(ctx)))
                .transpose()?;
            let resolved_order = match order {
                crate::cards::builders::LibraryBottomOrderAst::Random => {
                    crate::effects::consult_helpers::LibraryBottomOrder::Random
                }
                crate::cards::builders::LibraryBottomOrderAst::ChooserChooses => {
                    crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
                }
            };
            (
                vec![Effect::put_tagged_remainder_on_library_bottom(
                    resolved_tag,
                    resolved_keep_tagged,
                    resolved_order,
                    player_filter,
                )],
                choices,
            )
        }
        EffectAst::BecomeBasicLandTypeChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
                Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })?
        }
        EffectAst::BecomeBasicLandType {
            target,
            subtype,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
            Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::fixed(
                spec,
                *subtype,
                duration.clone(),
            ))
        })?,
        EffectAst::BecomeCreatureTypeChoice {
            target,
            duration,
            excluded_subtypes,
        } => {
            compile_tagged_effect_for_target(target, ctx, "become_creature_type_choice", |spec| {
                Effect::new(crate::effects::BecomeCreatureTypeChoiceEffect::new(
                    spec,
                    duration.clone(),
                    excluded_subtypes.clone(),
                ))
            })?
        }
        EffectAst::BecomeColorChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_color_choice", |spec| {
                Effect::new(crate::effects::BecomeColorChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })?
        }
        EffectAst::BecomeCopy {
            target,
            source,
            duration,
            preserve_source_abilities,
        } => {
            let refs = current_reference_env(ctx);
            let (target_spec, mut choices) = resolve_target_spec_with_choices(target, &refs)?;
            let (source_spec, source_choices) = resolve_target_spec_with_choices(source, &refs)?;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }

            let effect = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target_spec.clone(),
                crate::effects::continuous::RuntimeModification::CopyOf {
                    source: source_spec,
                    preserve_source_abilities: *preserve_source_abilities,
                },
                duration.clone(),
            ));
            let effect = tag_object_target_effect(effect, &target_spec, ctx, "copied");
            (vec![effect], choices)
        }
        EffectAst::Surveil { count, player } => compile_player_effect(
            *player,
            ctx,
            false,
            || Effect::surveil(count.clone()),
            |filter| Effect::surveil_player(count.clone(), filter),
        )?,
        EffectAst::PayMana { cost, player } => {
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
                Effect::new(crate::effects::PayManaEffect::new(
                    cost.clone(),
                    ChooseSpec::Player(filter),
                ))
            })?
        }
        EffectAst::PayEnergy { amount, player } => {
            compile_player_effect_from_filter(*player, ctx, false, |filter| {
                Effect::new(crate::effects::PayEnergyEffect::new(
                    amount.clone(),
                    ChooseSpec::Player(filter),
                ))
            })?
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}
