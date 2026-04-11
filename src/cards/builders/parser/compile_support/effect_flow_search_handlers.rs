use super::*;

pub(super) fn try_compile_flow_and_iteration_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::May { effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-effect branch is unsupported".to_string(),
                ));
            }
            if let Some(compiled) = lower_may_imprint_from_hand_effect(effects, ctx)? {
                return Ok(Some(compiled));
            }
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-effect branch is unsupported".to_string(),
                ));
            }
            let effect = Effect::may(inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::MayByPlayer { player, effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-by-player effect branch is unsupported".to_string(),
                ));
            }
            if matches!(player, PlayerAst::You | PlayerAst::Implicit)
                && let Some(compiled) = lower_may_imprint_from_hand_effect(effects, ctx)?
            {
                return Ok(Some(compiled));
            }
            let (player_filter, mut player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-by-player effect branch is unsupported".to_string(),
                ));
            }
            let effect = Effect::may_player(player_filter, inner_effects);
            let mut choices = inner_choices;
            choices.append(&mut player_choices);
            (vec![effect], choices)
        }
        EffectAst::RollDie { player, sides } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            ctx.last_player_filter = Some(player_filter.clone());
            (vec![Effect::roll_die(*sides, player_filter)], Vec::new())
        }
        EffectAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn => (
            vec![Effect::new(
                crate::effects::RetainManaUntilEndOfTurnEffect::you(),
            )],
            Vec::new(),
        ),
        EffectAst::RepeatThisProcessMay => (
            vec![Effect::new(crate::effects::RepeatProcessPromptEffect::new(
                "You may repeat this process any number of times",
            ))],
            Vec::new(),
        ),
        EffectAst::UnlessPays {
            effects,
            player,
            mana,
        } => {
            if effects.len() == 1
                && let EffectAst::ForEachObject {
                    filter,
                    effects: per_object_effects,
                } = &effects[0]
            {
                let rewritten = EffectAst::ForEachObject {
                    filter: filter.clone(),
                    effects: vec![EffectAst::UnlessPays {
                        effects: per_object_effects.clone(),
                        player: *player,
                        mana: mana.clone(),
                    }],
                };
                return Ok(Some(compile_effect(&rewritten, ctx)?));
            }

            let previous_last_player_filter = ctx.last_player_filter.clone();
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let player_filter = resolve_unless_player_filter(
                *player,
                &current_reference_env(ctx),
                previous_last_player_filter,
            )?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::unless_pays(inner_effects, player_filter, mana.clone());
            (vec![effect], inner_choices)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        } => {
            if effects.len() == 1
                && let EffectAst::ForEachObject {
                    filter,
                    effects: per_object_effects,
                } = &effects[0]
            {
                let rewritten = EffectAst::ForEachObject {
                    filter: filter.clone(),
                    effects: vec![EffectAst::UnlessAction {
                        effects: per_object_effects.clone(),
                        alternative: alternative.clone(),
                        player: *player,
                    }],
                };
                return Ok(Some(compile_effect(&rewritten, ctx)?));
            }

            let previous_last_player_filter = ctx.last_player_filter.clone();
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let (alt_effects, alt_choices) = compile_effects(alternative, ctx)?;
            let player_filter = resolve_unless_player_filter(
                *player,
                &current_reference_env(ctx),
                previous_last_player_filter,
            )?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = Effect::unless_action(inner_effects, alt_effects, player_filter);
            let mut choices = inner_choices;
            choices.extend(alt_choices);
            (vec![effect], choices)
        }
        EffectAst::IfResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for if clause".to_string())
            })?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(condition);
                    ctx.bind_unbound_x_to_last_effect = true;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect = Effect::if_then(condition, predicate, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::WhenResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for when clause".to_string())
            })?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(condition);
                    ctx.bind_unbound_x_to_last_effect = true;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(*predicate);
            let effect =
                Effect::reflexive_trigger(condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::ForEachOpponent { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_opponent(inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayersFiltered { filter, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = try_compile_simultaneous_each_player_scry(filter.clone(), &inner_effects)
                .unwrap_or_else(|| Effect::for_players(filter.clone(), inner_effects));
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayer { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect =
                try_compile_simultaneous_each_player_scry(PlayerFilter::Any, &inner_effects)
                    .unwrap_or_else(|| Effect::for_players(PlayerFilter::Any, inner_effects));
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTargetPlayers { count, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let target_spec =
                ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)).with_count(*count);
            let choose_targets =
                Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()));
            let effect = try_compile_simultaneous_each_player_scry(
                PlayerFilter::target_player(),
                &inner_effects,
            )
            .unwrap_or_else(|| Effect::for_players(PlayerFilter::target_player(), inner_effects));
            let mut choices = vec![target_spec];
            for choice in inner_choices {
                push_choice(&mut choices, choice);
            }
            (vec![choose_targets, effect], choices)
        }
        EffectAst::ForEachObject { filter, effects } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = None;
                    ctx.last_object_tag = Some(IT_TAG.to_string());
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let effect = Effect::for_each(resolved_filter, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTagged { tag, effects } => {
            let effective_tag = if tag.as_str() == IT_TAG {
                ctx.last_object_tag
                    .clone()
                    .unwrap_or_else(|| IT_TAG.to_string())
            } else {
                tag.as_str().to_string()
            };

            let (inner_effects, inner_choices) = compile_effects_in_iterated_player_context(
                effects,
                ctx,
                Some(effective_tag.clone()),
            )?;
            let effect = Effect::for_each_tagged(effective_tag, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTaggedPlayer { tag, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_tagged_player(tag.clone(), inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::RepeatProcess {
            effects,
            continue_effect_index,
            continue_predicate,
        } => {
            let (body_effects, choices, condition) = with_preserved_lowering_context(
                ctx,
                |_| {},
                |ctx| compile_repeat_process_body(effects, *continue_effect_index, ctx),
            )?;
            let effect = Effect::repeat_process(
                body_effects,
                condition,
                effect_predicate_from_if_result(*continue_predicate),
            );
            (vec![effect], choices)
        }
        EffectAst::ForEachOpponentDoesNot { .. } => {
            return Err(CardTextError::ParseError(
                "for each opponent who doesn't must follow an opponent clause".to_string(),
            ));
        }
        EffectAst::ForEachPlayerDoesNot { .. } => {
            return Err(CardTextError::ParseError(
                "for each player who doesn't must follow a player clause".to_string(),
            ));
        }
        EffectAst::ForEachOpponentDid { .. } => {
            return Err(CardTextError::ParseError(
                "for each opponent who ... this way must follow an opponent clause".to_string(),
            ));
        }
        EffectAst::ForEachPlayerDid { .. } => {
            return Err(CardTextError::ParseError(
                "for each player who ... this way must follow a player clause".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_token_generation_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::CreateTokenWithMods {
            name,
            count,
            dynamic_power_toughness,
            player,
            attached_to,
            tapped,
            attacking,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
        } => {
            let token = token_definition_for(name.as_str())
                .or_else(|| {
                    dynamic_power_toughness
                        .as_ref()
                        .and_then(|_| token_definition_for(format!("0/0 {name}").as_str()))
                })
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut effect = if matches!(player_filter, PlayerFilter::You) {
                crate::effects::CreateTokenEffect::you(token, count.clone())
            } else {
                crate::effects::CreateTokenEffect::new(token, count.clone(), player_filter.clone())
            };
            if *tapped {
                effect = effect.tapped();
            }
            if *attacking {
                effect = effect.attacking();
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_end_of_combat();
            }
            if *sacrifice_at_end_of_combat {
                effect = effect.sacrifice_at_end_of_combat();
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step();
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step();
            }
            let mut effect = Effect::new(effect);
            let resolved_dynamic_pt = dynamic_power_toughness
                .as_ref()
                .map(|(power, toughness)| {
                    Ok::<_, CardTextError>((
                        resolve_value_it_tag(power, &current_reference_env(ctx))?,
                        resolve_value_it_tag(toughness, &current_reference_env(ctx))?,
                    ))
                })
                .transpose()?;
            let needs_created_tag = ctx.auto_tag_object_targets
                || attached_to.is_some()
                || resolved_dynamic_pt.is_some();
            let mut created_tag: Option<String> = None;
            if needs_created_tag {
                let tag = ctx.next_tag("created");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                created_tag = Some(tag);
            }

            let mut compiled = vec![effect];
            if let Some((power, toughness)) = resolved_dynamic_pt {
                let Some(created_tag) = created_tag.clone() else {
                    return Err(CardTextError::InvariantViolation(
                        "dynamic token pt requires created token tag to be present".to_string(),
                    ));
                };
                compiled.extend(
                    compile_effect_for_target(
                        &TargetAst::Tagged(TagKey::from(created_tag.as_str()), None),
                        ctx,
                        |spec| {
                            Effect::set_base_power_toughness(
                                power.clone(),
                                toughness.clone(),
                                spec,
                                Until::Forever,
                            )
                        },
                    )?
                    .0,
                );
            }
            if let Some(target) = attached_to {
                let (target_spec, target_choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                let Some(created_tag) = created_tag else {
                    return Err(CardTextError::InvariantViolation(
                        "attached token creation requires created token tag to be present"
                            .to_string(),
                    ));
                };
                let objects = ChooseSpec::All(ObjectFilter::tagged(created_tag));
                compiled.push(Effect::attach_objects(objects, target_spec));
            }
            (compiled, choices)
        }
        EffectAst::CreateTokenCopy {
            object,
            count,
            player,
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
            half_power_toughness_round_up,
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        } => {
            let ObjectRefAst::Tagged(tag) = object;
            let tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                ChooseSpec::Tagged(tag),
                count,
                player_filter,
            );
            if *enters_tapped {
                effect = effect.enters_tapped(true);
            }
            if *enters_attacking {
                effect = effect.attacking(true);
            }
            if let Some(attack_player) = attack_target_player_or_planeswalker_controlled_by {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                effect =
                    effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter);
            }
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_eoc(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            if let Some(colors) = set_colors {
                effect = effect.set_colors(*colors);
            }
            if let Some(card_types) = set_card_types {
                effect = effect.set_card_types(card_types.clone());
            }
            if let Some(subtypes) = set_subtypes {
                effect = effect.set_subtypes(subtypes.clone());
            }
            for card_type in added_card_types {
                effect = effect.added_card_type(*card_type);
            }
            for subtype in added_subtypes {
                effect = effect.added_subtype(*subtype);
            }
            for supertype in removed_supertypes {
                effect = effect.removed_supertype(*supertype);
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                effect = effect.set_base_power_toughness(*power, *toughness);
            }
            for ability in granted_abilities {
                effect = effect.grant_static_ability(ability.clone());
            }
            let mut effect = Effect::new(effect);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::CreateTokenCopyFromSource {
            source,
            count,
            player,
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
            half_power_toughness_round_up,
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let (mut source_spec, source_choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }
            if let Some(last_tag) = ctx.last_object_tag.as_deref()
                && str_starts_with(last_tag, "exile_cost_")
                && let ChooseSpec::Object(filter) = &source_spec
                && filter.zone == Some(Zone::Exile)
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                })
            {
                source_spec = ChooseSpec::Tagged(TagKey::from(last_tag));
            }
            let mut effect =
                crate::effects::CreateTokenCopyEffect::new(source_spec, count, player_filter);
            if *enters_tapped {
                effect = effect.enters_tapped(true);
            }
            if *enters_attacking {
                effect = effect.attacking(true);
            }
            if let Some(attack_player) = attack_target_player_or_planeswalker_controlled_by {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                effect =
                    effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter);
            }
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_eoc(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            if let Some(colors) = set_colors {
                effect = effect.set_colors(*colors);
            }
            if let Some(card_types) = set_card_types {
                effect = effect.set_card_types(card_types.clone());
            }
            if let Some(subtypes) = set_subtypes {
                effect = effect.set_subtypes(subtypes.clone());
            }
            for card_type in added_card_types {
                effect = effect.added_card_type(*card_type);
            }
            for subtype in added_subtypes {
                effect = effect.added_subtype(*subtype);
            }
            for supertype in removed_supertypes {
                effect = effect.removed_supertype(*supertype);
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                effect = effect.set_base_power_toughness(*power, *toughness);
            }
            for ability in granted_abilities {
                effect = effect.grant_static_ability(ability.clone());
            }

            let mut effect = Effect::new(effect);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_search_and_reorder_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::SearchLibrary {
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value,
            tapped,
        } => {
            let (chooser_filter, chooser_choices) = if matches!(*chooser, PlayerAst::Implicit)
                && matches!(*player, PlayerAst::That)
                && filter.owner.is_some()
                && ctx.last_player_filter.as_ref().is_some_and(|filter| {
                    !matches!(
                        filter,
                        PlayerFilter::IteratedPlayer | PlayerFilter::TaggedPlayer(_)
                    )
                }) {
                (PlayerFilter::You, Vec::new())
            } else {
                resolve_effect_player_filter(*chooser, ctx, true, true, true)?
            };
            let (player_filter, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            for choice in chooser_choices {
                push_choice(&mut choices, choice);
            }
            let count = *count;
            let mut filter = filter.clone();
            if filter.owner.is_none() && !matches!(player_filter, PlayerFilter::You) {
                filter.owner = Some(player_filter.clone());
            }
            ctx.last_player_filter = Some(
                filter
                    .owner
                    .clone()
                    .unwrap_or_else(|| player_filter.clone()),
            );
            let use_search_effect = *shuffle
                && count.min == 0
                && count.max == Some(1)
                && count_value.is_none()
                && *destination != Zone::Battlefield;
            if use_search_effect {
                let mut effect = Effect::new(
                    crate::effects::SearchLibraryEffect::new(
                        filter,
                        *destination,
                        chooser_filter.clone(),
                        player_filter.clone(),
                        *reveal,
                    )
                    .with_search_mode(*search_mode),
                );
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("searched");
                    ctx.last_object_tag = Some(tag.clone());
                    effect = effect.tag(tag);
                }
                let effects = vec![effect];
                (effects, choices)
            } else {
                let tag = ctx.next_tag("searched");
                if ctx.auto_tag_object_targets {
                    ctx.last_object_tag = Some(tag.clone());
                }
                let mut generic_search_filter = ObjectFilter::default();
                generic_search_filter.owner = filter.owner.clone();
                let choose_description = if filter == generic_search_filter {
                    if count.max == Some(1) {
                        "card"
                    } else {
                        "cards"
                    }
                } else {
                    "objects"
                };
                let choose = crate::effects::ChooseObjectsEffect::new(
                    filter,
                    count,
                    chooser_filter.clone(),
                    tag.clone(),
                )
                .with_count_value_opt(count_value.clone())
                .in_zone(Zone::Library)
                .with_description(choose_description);
                let choose = match search_mode {
                    crate::effect::SearchSelectionMode::Exact => choose.as_search(),
                    crate::effect::SearchSelectionMode::Optional => choose.as_optional_search(),
                    crate::effect::SearchSelectionMode::AllMatching => {
                        choose.as_all_matching_search()
                    }
                };
                let choose = if *reveal { choose.reveal() } else { choose };

                let to_top = matches!(destination, Zone::Library);
                let move_effect = if *destination == Zone::Battlefield {
                    Effect::put_onto_battlefield(
                        ChooseSpec::Iterated,
                        *tapped,
                        player_filter.clone(),
                    )
                } else {
                    Effect::move_to_zone(ChooseSpec::Iterated, *destination, to_top)
                };
                let mut sequence_effects = vec![Effect::new(choose)];
                if *shuffle && *destination == Zone::Library {
                    sequence_effects.push(Effect::shuffle_library_player(player_filter.clone()));
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                } else {
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                    if *shuffle {
                        sequence_effects.push(Effect::shuffle_library_player(player_filter));
                    }
                }
                let sequence = crate::effects::SequenceEffect::new(sequence_effects);
                (vec![Effect::new(sequence)], std::mem::take(&mut choices))
            }
        }
        EffectAst::ShuffleHandAndGraveyardIntoLibrary { player } => {
            compile_player_effect_from_filter(
                *player,
                ctx,
                true,
                Effect::shuffle_hand_and_graveyard_into_library_player,
            )?
        }
        EffectAst::ShuffleGraveyardIntoLibrary { player } => compile_player_effect_from_filter(
            *player,
            ctx,
            true,
            Effect::shuffle_graveyard_into_library_player,
        )?,
        EffectAst::ReorderGraveyard { player } => {
            compile_player_effect_from_filter(*player, ctx, true, Effect::reorder_graveyard_player)?
        }
        EffectAst::ReorderTopOfLibrary { tag } => {
            let effective_tag = if tag.as_str() == IT_TAG {
                ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "cannot resolve 'them' without prior tagged object".to_string(),
                    )
                })?
            } else {
                tag.as_str().to_string()
            };
            (
                vec![Effect::new(crate::effects::ReorderLibraryTopEffect::new(
                    effective_tag,
                ))],
                Vec::new(),
            )
        }
        EffectAst::ShuffleLibrary { player } => {
            if ctx
                .last_object_tag
                .as_ref()
                .is_some_and(|tag| tag.starts_with("searched"))
                && ctx
                    .last_player_filter
                    .as_ref()
                    .is_some_and(|filter| *filter != PlayerFilter::You)
            {
                (
                    vec![Effect::shuffle_library_player(
                        ctx.last_player_filter.clone().expect("checked above"),
                    )],
                    Vec::new(),
                )
            } else {
                compile_player_effect_from_filter(
                    *player,
                    ctx,
                    true,
                    Effect::shuffle_library_player,
                )?
            }
        }
        EffectAst::VoteOption { option, effects } => {
            let mut option_effects_ast = effects.clone();
            force_implicit_vote_token_controller_you(&mut option_effects_ast);
            let (repeat_effects, repeat_choices) = compile_effects(&option_effects_ast, ctx)?;
            (
                vec![Effect::repeat_effects(
                    Value::VoteCount(option.clone()),
                    repeat_effects,
                )],
                repeat_choices,
            )
        }
        EffectAst::VoteStart { .. }
        | EffectAst::VoteStartObjects { .. }
        | EffectAst::VoteExtra { .. } => {
            return Err(CardTextError::ParseError(
                "vote clauses must appear together".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}
