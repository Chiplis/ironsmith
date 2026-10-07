/// Resolve a Value to an i32 (direct version without CalculationContext).
pub(crate) fn resolve_value_direct(
    value: &Value,
    objects: &ObjectMap,
    effects: &[ContinuousEffect],
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    source: ObjectId,
    controller: PlayerId,
    game: &crate::game_state::GameState,
) -> i32 {
    resolve_value_direct_for_recipient(
        value,
        objects,
        effects,
        battlefield,
        commanders,
        source,
        source,
        controller,
        game,
    )
}

/// Resolve a Value to an i32 (direct version without CalculationContext).
pub(crate) fn resolve_value_direct_for_recipient(
    value: &Value,
    objects: &ObjectMap,
    effects: &[ContinuousEffect],
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    source: ObjectId,
    recipient: ObjectId,
    controller: PlayerId,
    game: &crate::game_state::GameState,
) -> i32 {
    resolve_value_direct_for_recipient_impl(
        value, objects, effects, battlefield, commanders, source, recipient, controller, game,
        None, None,
    )
}

pub(crate) fn resolve_characteristic_value_direct_for_recipient(
    value: &Value,
    objects: &ObjectMap,
    effects: &[ContinuousEffect],
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    source: ObjectId,
    recipient: ObjectId,
    controller: PlayerId,
    game: &crate::game_state::GameState,
    error: &mut Option<(&'static str, i128)>,
    evidence_error: &mut Option<&'static str>,
    numeric_origin: Option<&crate::continuous::AbilityOrigin>,
) -> i32 {
    resolve_value_direct_for_recipient_impl(
        value, objects, effects, battlefield, commanders, source, recipient, controller, game,
        Some((error,evidence_error)), numeric_origin,
    )
}

fn resolve_value_direct_for_recipient_impl(
    value: &Value,
    objects: &ObjectMap,
    effects: &[ContinuousEffect],
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    source: ObjectId,
    recipient: ObjectId,
    controller: PlayerId,
    game: &crate::game_state::GameState,
    error: Option<(&mut Option<(&'static str, i128)>,&mut Option<&'static str>)>,
    numeric_origin: Option<&crate::continuous::AbilityOrigin>,
) -> i32 {
    let mut effect_manager = ContinuousEffectManager::new();
    for effect in effects {
        effect_manager.add_effect(effect.clone());
    }
    let calculation = CalculationContext {
        objects,
        effects: &effect_manager,
        battlefield,
        game,
        current_object: recipient,
    };
    let layer = super::value_context::LayerValueContext::direct(
        &calculation,
        source,
        controller,
        effects,
        commanders,
    ).with_numeric_origin(numeric_origin);
    match error {
        Some((error,evidence_error)) => crate::effects::helpers::value_eval::resolve_continuous_characteristic(value, layer, error,evidence_error),
        None => crate::effects::helpers::value_eval::resolve_continuous(value, layer),
    }
}

/// Apply all layers to calculate final characteristics.
pub(super) fn calculate_with_layers(
    object: &Object,
    ctx: &CalculationContext,
) -> CalculatedCharacteristics {
    use crate::dependency::needs_baseline_dependency_sort;
    use crate::dependency::sort_layer_effects;
    use crate::dependency::sort_layer_effects_with_baseline_and_started_groups;

    let mut chars = initial_characteristics(object, ctx.game.turn.turn_number);
    if chars.world_supertype_since.is_some() {
        chars.world_supertype_since = ctx.effects.get_entry_timestamp(object.id).or(Some(0));
    }
    let calc_guard = CharacteristicCalculationGuard::begin(ctx.game, object.id, &chars);
    let mut started_groups = HashSet::new();

    // Get all effects sorted by layer/sublayer/timestamp
    let effects = ctx.effects.effects_sorted();
    let mut all_effects: Option<Vec<ContinuousEffect>> = None;

    // Group effects by layer for dependency-aware sorting within each layer
    let mut effects_by_layer: HashMap<Layer, Vec<&ContinuousEffect>> = HashMap::with_capacity(7);
    for effect in &effects {
        effects_by_layer
            .entry(effect.modification.layer())
            .or_default()
            .push(*effect);
    }

    // Process layers in order (1-6); Layer 7 is handled by sublayer below.
    let layers = [
        Layer::Copy,
        Layer::Control,
        Layer::Text,
        Layer::Type,
        Layer::Color,
        Layer::Ability,
    ];

    // Track which abilities have been removed (for dependency detection)
    let mut abilities_removed = false;
    let ability_counters = ability_counter_timestamps(object, ctx.effects);
    let mut next_ability_counter = 0;

    for layer in layers {
        if layer == Layer::Ability {
            add_intrinsic_abilities(&mut chars);
            calc_guard.update(&chars);
        }
        let layer_effects = match effects_by_layer.get(&layer) {
            Some(effects) => effects,
            None => {
                if layer == Layer::Copy {
                    let had_world = chars.supertypes.contains(&Supertype::World);
                    apply_face_down_layer(object, &mut chars);
                    apply_room_no_unlocked_door_layer(object, &mut chars, ctx.game);
                    update_world_supertype_since(
                        &mut chars,
                        had_world,
                        ctx.effects.get_entry_timestamp(object.id).unwrap_or(0),
                    );
                    calc_guard.update(&chars);
                }
                if layer == Layer::Type {
                    apply_reconfigure_attached_type_rule(object, &mut chars);
                    apply_ring_bearer_legendary_rule(object, &mut chars, ctx.game);
                    calc_guard.update(&chars);
                }
                if layer == Layer::Ability {
                    apply_ability_counters_through(
                        object,
                        &mut chars,
                        &ability_counters,
                        &mut next_ability_counter,
                        None,
                    );
                    prune_ability_gain_prohibitions(&mut chars);
                    calc_guard.update(&chars);
                }
                continue;
            }
        };
        let needs_source_tracking =
            layer_needs_source_activity_tracking(layer_effects, effects.iter().copied(), layer);
        let needs_sort_baseline = needs_baseline_dependency_sort(layer_effects, ctx.game);
        let baseline = if needs_sort_baseline {
            let all_effects =
                all_effects.get_or_insert_with(|| effects.iter().map(|e| (*e).clone()).collect());
            Some(build_layer_baseline(
                ctx.objects,
                all_effects,
                ctx.battlefield,
                ctx.game.commander_objects(),
                ctx.game,
                layer,
                None,
            ))
        } else {
            None
        };
        let tracked_source_ids =
            needs_source_tracking.then(|| tracked_source_ids_for_layer(layer_effects));
        let mut source_state = if needs_source_tracking {
            let all_effects =
                all_effects.get_or_insert_with(|| effects.iter().map(|e| (*e).clone()).collect());
            build_object_baseline_for_ids(
                ctx.objects,
                all_effects,
                ctx.battlefield,
                ctx.game.commander_objects(),
                ctx.game,
                layer,
                None,
                tracked_source_ids
                    .as_ref()
                    .expect("tracked sources should exist when source tracking is enabled"),
            )
        } else {
            HashMap::new()
        };

        // Apply dependency-aware sorting within this layer
        // This handles Rule 613.8 - effects that depend on each other
        let sorted_effects = {
            if needs_sort_baseline {
                let baseline = baseline
                    .as_ref()
                    .expect("baseline should exist when dependency sorting needs it");
                let started_groups_for_sort = crate::dependency::started_groups_for_sort(
                    effects.iter().copied(),
                    layer,
                    baseline,
                    ctx.objects,
                    ctx.game,
                );
                sort_layer_effects_with_baseline_and_started_groups(
                    layer_effects,
                    baseline,
                    ctx.objects,
                    ctx.game,
                    &started_groups_for_sort,
                )
            } else {
                sort_layer_effects(layer_effects)
            }
        };

        // Apply effects in dependency order
        for effect in sorted_effects {
            if layer == Layer::Ability {
                if !crate::continuous::is_land_type_rules_text_ability_loss(effect) {
                    apply_ability_counters_through(
                        object,
                        &mut chars,
                        &ability_counters,
                        &mut next_ability_counter,
                        Some(effect.timestamp),
                    );
                }
                prune_ability_gain_prohibitions(&mut chars);
                calc_guard.update(&chars);
            }
            let effect_active = if needs_source_tracking {
                continuous_effect_group_started(effect, &started_groups)
                    || effect_source_is_active(effect, &source_state)
            } else {
                true
            };

            if effect.has_source_controller_context()
                && !ctx
                    .objects
                    .contains_key(&effect.source_controller_context_host())
            {
                continue;
            }
            let bound_effect = if needs_source_tracking && effect_active {
                bind_effect_controller_to_layer_frame(effect, &source_state)
            } else {
                std::borrow::Cow::Borrowed(effect)
            };
            let effect = bound_effect.as_ref();
            if needs_source_tracking && effect_active {
                advance_layer_source_state(
                    &mut source_state,
                    effect,
                    ctx.objects,
                    ctx.battlefield,
                    ctx.game.commander_objects(),
                    ctx.game,
                );
            }

            if !effect_active {
                continue;
            }

            // Check if this effect applies to our object
            if !effect_applies_to_or_started(effect, &started_groups, object, &chars, ctx) {
                continue;
            }

            mark_continuous_effect_group_started(effect, &mut started_groups);
            // Apply the modification. Record the transition timestamp, rather
            // than merely the permanent's entry timestamp: CR 704.5k compares
            // how long each permanent has continuously had the World
            // supertype, including type- and copy-changing effects.
            let had_world = chars.supertypes.contains(&Supertype::World);
            chars.abilities.begin_effect(effect);
            match &effect.modification {
                // Layer 1: Copy
                Modification::CopyOf {
                    copiable_values,
                    preserve_source_abilities,
                    name_override,
                    name_override_surface,
                    add_supertypes,
                    ..
                } => {
                    // Per MTG rule 707.2, copying copies the copiable values:
                    // name, mana cost, color indicator, card type, subtype, supertype,
                    // rules text, power, toughness, and loyalty.
                    // It does NOT copy counters, damage, or other non-copiable state.
                    copy_characteristics_from_copiable_values(
                        copiable_values,
                        &mut chars,
                        *preserve_source_abilities,
                        name_override,
                        name_override_surface,
                        add_supertypes,
                        Some(effect.into()),
                    );
                }

                // Layer 2: Control
                Modification::ChangeController(new_controller) => {
                    chars.controller = *new_controller;
                }
                Modification::ChangeControllerToEffectController => {
                    chars.controller = effect.controller;
                }
                Modification::ChangeText { .. } => {
                    // Legacy wire vocabulary. New instructions use RewriteText.
                }
                Modification::RewriteText(change) => text_changes::apply_text_change(&mut chars, *change, object),
                Modification::SetTextBox(overlay) => {
                    chars.compiled_card_text = overlay.compiled_card_text.clone();
                    replace_rules_text_abilities(
                        &mut chars,
                        overlay.abilities.to_vec(),
                        Some(effect.into()),
                        false,
                    );
                    chars.static_abilities = extract_static_abilities(&chars.abilities).into();
                }
                Modification::SetName(name) => {
                    chars.name = name.clone().into();
                    chars.alternate_name = None;
                }
                Modification::InsertNameWords {
                    words,
                    after_word_count,
                } => {
                    chars.name =
                        insert_name_sticker_words(&chars.name, words, *after_word_count).into();
                }

                // Layer 4: Type changes
                Modification::AddCardTypes(types) => {
                    for t in types {
                        if !chars.card_types.contains(t) {
                            chars.card_types.push(*t);
                        }
                    }
                }
                Modification::RemoveCardTypes(types) => {
                    remove_card_types_and_prune_subtypes(
                        &mut chars.card_types,
                        &mut chars.subtypes,
                        types,
                    );
                }
                Modification::SetCardTypes(types) => {
                    replace_card_types_and_prune_subtypes(
                        &mut chars.card_types,
                        &mut chars.subtypes,
                        types,
                    );
                }
                Modification::AddSubtypes(types) => {
                    for t in types {
                        if !chars.subtypes.contains(t) {
                            chars.subtypes.push(*t);
                        }
                    }
                }
                Modification::RemoveSubtypes(types) => {
                    chars.subtypes.retain(|t| !types.contains(t));
                }
                Modification::AddAllSubtypesOfFamily(family) => {
                    for subtype in family.all_subtypes() {
                        if !chars.subtypes.contains(subtype) {
                            chars.subtypes.push(*subtype);
                        }
                    }
                }
                Modification::RemoveAllSubtypesOfFamily(family) => {
                    chars.subtypes.retain(|t| !t.belongs_to_family(*family));
                }
                Modification::SetSubtypes(types) => {
                    // CR 205.1a: the new subtypes replace the existing subtypes
                    // of the same family only. Blood Moon replaces land types
                    // and keeps a Saga a Saga; "creatures are Goblins" replaces
                    // creature types and keeps an Aura an Aura.

                    replace_subtypes_for_set(&mut chars.subtypes, types);
                }
                Modification::SetAuraAttachmentFilter(filter) => {
                    replace_enchant_metadata(&mut chars, filter);
                }
                Modification::AddSupertypes(types) => {
                    for t in types {
                        if !chars.supertypes.contains(t) {
                            chars.supertypes.push(*t);
                        }
                    }
                }
                Modification::RemoveSupertypes(types) => {
                    chars.supertypes.retain(|t| !types.contains(t));
                }
                Modification::RemoveAllCreatureTypes => {
                    chars.subtypes.retain(|t| !t.is_creature_type());
                }

                // Layer 5: Color changes
                Modification::AddColors(colors) => {
                    chars.colors = chars.colors.union(*colors);
                }
                Modification::RemoveColors(colors) => {
                    // Remove each color in the set
                    use crate::color::Color;
                    for color in [
                        Color::White,
                        Color::Blue,
                        Color::Black,
                        Color::Red,
                        Color::Green,
                    ] {
                        if colors.contains(color) {
                            chars.colors = chars.colors.without(color);
                        }
                    }
                }
                Modification::SetColors(colors) => {
                    chars.colors = *colors;
                }
                Modification::MakeColorless => {
                    chars.colors = ColorSet::COLORLESS;
                }

                // Layer 6: Ability changes
                Modification::AddAbility(ability) => {
                    push_granted_static_ability(&mut chars, ability.clone());
                }
                Modification::AddAbilityGeneric(ability) => {
                    let bound_ability =
                        bind_effect_controller_in_ability(ability, effect.controller);
                    if let AbilityKind::Static(static_ability) = &bound_ability.kind {
                        push_granted_static_ability(&mut chars, static_ability.clone());
                    } else {
                        chars.abilities.push(bound_ability);
                    }
                }
                Modification::SetAbilities(abilities) => {
                    chars.abilities.replace_with_origin(abilities.clone(), Some(effect.into()));
                    chars.static_abilities = extract_static_abilities(abilities).into();
                }
                Modification::CopyActivatedAbilities {
                    filter,
                    counter,
                    include_mana,
                    only_loyalty,
                    exclude_source_name,
                    exclude_source_id,
                    force_once_each_turn,
                } => {
                    use crate::ability::AbilityKind;
                    // Reuse this derivation's effect set. Re-entering static
                    // effect generation here recursively rebuilds the board.
                    let effects = all_effects.get_or_insert_with(|| {
                        effects.iter().map(|effect| (**effect).clone()).collect()
                    });
                    let donor_context =
                        continuous_filter_context(ctx.game, effect.controller, effect.source);
                    let mut candidate_ids =
                        ability_copy_candidate_ids(ctx.objects, filter, &donor_context);
                    candidate_ids.sort();

                    for candidate_id in candidate_ids {
                        let Some(candidate) = ctx.objects.get(&candidate_id) else {
                            continue;
                        };
                        if *exclude_source_id && candidate.id == object.id {
                            continue;
                        }
                        if *exclude_source_name && candidate.name == object.name {
                            continue;
                        }
                        if let Some(counter_type) = counter
                            && candidate.counters.get(counter_type).copied().unwrap_or(0) == 0
                        {
                            continue;
                        }

                        let Some(candidate_chars) = calculate_characteristics_with_effects_simple(
                            candidate.id,
                            ctx.objects,
                            &effects,
                            ctx.battlefield,
                            ctx.game.commander_objects(),
                            ctx.game,
                        ) else {
                            continue;
                        };

                        if !filter_matches_with_characteristics_in_context(
                            filter,
                            candidate,
                            &candidate_chars,
                            ctx.game,
                            &donor_context,
                        ) {
                            continue;
                        }

                        for (ability_index, ability) in candidate_chars.abilities.iter().enumerate()
                        {
                            let AbilityKind::Activated(activated) = &ability.kind else {
                                continue;
                            };
                            if *only_loyalty && !activated.is_loyalty_ability() {
                                continue;
                            }
                            if ability_is_mana_for_object(ability, ctx.game, candidate)
                                && !*include_mana
                            {
                                continue;
                            }
                            let mut copied = ability.clone();
                            if *force_once_each_turn
                                && let AbilityKind::Activated(activated) = &mut copied.kind
                            {
                                // CR 602.5b / 113.3: the once-each-turn limit is added to the
                                // borrowed ability's own restrictions, never replacing them.
                                crate::continuous::add_once_each_turn_activation_limit(activated);
                            }
                            chars.abilities.push_with_origin(
                                copied,
                                AbilityOrigin::Borrowed {
                                    effect: effect.into(),
                                    source: candidate.id,
                                    origin: Box::new(
                                        candidate_chars
                                            .abilities
                                            .origin(ability_index)
                                            .unwrap()
                                            .clone(),
                                    ),
                                },
                            );
                        }
                    }
                }
                Modification::CopyStaticAbilityVariants {
                    filter,
                    selectors,
                    exclude_source_id,
                } => {
                    // Reuse this derivation's effect set. Re-entering static
                    // effect generation here recursively rebuilds the board.
                    let effects = all_effects.get_or_insert_with(|| {
                        effects.iter().map(|effect| (**effect).clone()).collect()
                    });
                    copy_static_ability_variants_into(
                        &mut chars,
                        filter,
                        selectors,
                        *exclude_source_id,
                        object,
                        ctx.objects,
                        &effects,
                        ctx.battlefield,
                        ctx.game.commander_objects(),
                        ctx.game,
                        effect,
                    );
                }
                Modification::CopyTriggeredAbilities {
                    filter,
                    exclude_source_name,
                    exclude_source_id,
                } => {
                    use crate::ability::AbilityKind;
                    // Reuse this derivation's effect set. Re-entering static
                    // effect generation here recursively rebuilds the board.
                    let effects = all_effects.get_or_insert_with(|| {
                        effects.iter().map(|effect| (**effect).clone()).collect()
                    });
                    let donor_context =
                        continuous_filter_context(ctx.game, effect.controller, effect.source);
                    let mut candidate_ids =
                        ability_copy_candidate_ids(ctx.objects, filter, &donor_context);
                    candidate_ids.sort();

                    for candidate_id in candidate_ids {
                        let Some(candidate) = ctx.objects.get(&candidate_id) else {
                            continue;
                        };
                        if *exclude_source_id && candidate.id == object.id {
                            continue;
                        }
                        if *exclude_source_name && candidate.name == object.name {
                            continue;
                        }

                        let Some(candidate_chars) = calculate_characteristics_with_effects_simple(
                            candidate.id,
                            ctx.objects,
                            &effects,
                            ctx.battlefield,
                            ctx.game.commander_objects(),
                            ctx.game,
                        ) else {
                            continue;
                        };

                        if !filter_matches_with_characteristics_in_context(
                            filter,
                            candidate,
                            &candidate_chars,
                            ctx.game,
                            &donor_context,
                        ) {
                            continue;
                        }

                        for (slot, ability) in candidate_chars.abilities.iter().enumerate() {
                            if matches!(ability.kind, AbilityKind::Triggered(_)) {
                                chars.abilities.push_with_origin(ability.clone(), AbilityOrigin::Borrowed {
                                    effect: effect.into(),
                                    source: candidate.id,
                                    origin: Box::new(candidate_chars.abilities.origin(slot)
                                        .expect("copied trigger retains its donor origin").clone()),
                                });
                            }
                        }
                    }
                }
                Modification::AddCombatDamageDrawAbility => {
                    chars.abilities.push(Ability::triggered(
                        crate::triggers::Trigger::this_deals_combat_damage_to_player(
                            crate::target::PlayerFilter::Any,
                        ),
                        vec![crate::effect::Effect::draw(1)],
                    ));
                }
                Modification::RemoveAbility(ability) => {
                    chars.abilities.retain(|candidate| {
                        !matches!(&candidate.kind, AbilityKind::Static(existing)
                            if static_ability_matches_loss(existing, ability))
                    });
                    chars
                        .static_abilities
                        .retain(|candidate| !static_ability_matches_loss(candidate, ability));
                }
                Modification::RemoveStaticAbilityFamily(id) => {
                    chars.abilities.retain(|candidate| {
                        !matches!(&candidate.kind, AbilityKind::Static(ability) if ability.id() == *id)
                    });
                    chars
                        .static_abilities
                        .retain(|candidate| candidate.id() != *id);
                }
                Modification::RemoveAbilityGeneric { ability, .. } => {
                    chars
                        .abilities
                        .retain(|candidate| !object_ability_matches_loss(candidate, ability));
                    if let AbilityKind::Static(static_ability) = &ability.kind {
                        chars.static_abilities.retain(|candidate| {
                            !static_ability_matches_loss(candidate, static_ability)
                        });
                    }
                }
                Modification::RemoveAllAbilities | Modification::RemoveLandRulesTextAbilities => {
                    remove_all_abilities_for_effect(effect, &mut chars);
                    abilities_removed = true;
                }
                Modification::RemoveAllAbilitiesExceptMana => {
                    chars
                        .abilities
                        .retain(|ability| ability_is_mana_for_object(ability, ctx.game, object));
                    chars.static_abilities.clear();
                    abilities_removed = true;
                }
                Modification::Restriction(restriction) => {
                    push_granted_static_ability(&mut chars, restriction.ability().clone());
                }

                // Layer 7: P/T changes are handled separately below.
                Modification::SetPower { .. }
                | Modification::SetToughness { .. }
                | Modification::SetPowerToughness { .. }
                | Modification::ModifyPower(_)
                | Modification::ModifyToughness(_)
                | Modification::ModifyPowerToughness { .. }
                | Modification::ModifyPowerToughnessValue { .. }
                | Modification::ModifyPowerToughnessByColorCount { .. }
                | Modification::SwitchPowerToughness => {}
            }

            enforce_ability_gain_prohibitions(&mut chars, &effect.modification);

            update_world_supertype_since(
                &mut chars,
                had_world,
                effect
                    .timestamp
                    .max(ctx.effects.get_entry_timestamp(object.id).unwrap_or(0)),
            );

            calc_guard.update(&chars);
        }

        if layer == Layer::Copy {
            let had_world = chars.supertypes.contains(&Supertype::World);
            apply_face_down_layer(object, &mut chars);
            apply_room_no_unlocked_door_layer(object, &mut chars, ctx.game);
            update_world_supertype_since(
                &mut chars,
                had_world,
                ctx.effects.get_entry_timestamp(object.id).unwrap_or(0),
            );
            calc_guard.update(&chars);
        } else if layer == Layer::Type {
            apply_reconfigure_attached_type_rule(object, &mut chars);
            apply_ring_bearer_legendary_rule(object, &mut chars, ctx.game);
            calc_guard.update(&chars);
        } else if layer == Layer::Ability {
            apply_ability_counters_through(
                object,
                &mut chars,
                &ability_counters,
                &mut next_ability_counter,
                None,
            );
            prune_ability_gain_prohibitions(&mut chars);
            calc_guard.update(&chars);
        }
    }

    // Now handle Layer 7 (P/T) with proper sublayer ordering
    // We need to collect P/T effects and apply them in sublayer order

    // The surviving layer-six view, including grants after an earlier loss,
    // controls the level symbol. A historical remove-all flag is not its state.
    let level_pt = get_level_ability_pt(object, &chars.abilities);
    apply_level_granted_abilities(object, &mut chars);
    prune_ability_gain_prohibitions(&mut chars);
    calc_guard.update(&chars);

    // Apply Layer 7 effects in sublayer order. A level symbol's "base P/T"
    // is a layer-7b effect with the leveler's timestamp (CR 711.2b, 613.4b).
    apply_layer_7_effects(
        object,
        ctx,
        &mut chars,
        abilities_removed,
        &calc_guard,
        &mut started_groups,
        level_pt,
    );

    prune_ability_gain_prohibitions(&mut chars);
    calc_guard.update(&chars);

    refresh_active_static_abilities(&mut chars, ctx.game, object.id);
    calc_guard.update(&chars);

    chars
}

/// Apply Layer 7 effects (P/T modifications) in sublayer order.
///
/// Per Rule 613.4, the sublayers are:
/// - 7a: CDAs that define P/T
/// - 7b: Effects that set P/T to specific values
/// - 7c: Effects that modify P/T (including +1/+1 and -1/-1 counters)
/// - 7d: Effects that switch P/T
///
/// IMPORTANT: Per Rule 613.4c, counters are part of sublayer 7c, not a separate sublayer.
/// All 7c effects (including counters) are applied in timestamp order together.
pub(super) fn apply_layer_7_effects(
    object: &Object,
    ctx: &CalculationContext,
    chars: &mut CalculatedCharacteristics,
    _abilities_removed: bool,
    calc_guard: &CharacteristicCalculationGuard,
    started_groups: &mut HashSet<ContinuousEffectGroupId>,
    level_pt: Option<(i32, i32)>,
) {
    use crate::dependency::needs_baseline_dependency_sort;
    use crate::dependency::sort_layer_effects;
    use crate::dependency::sort_layer_effects_with_baseline_and_started_groups;

    let effects = ctx.effects.effects_sorted();
    let mut all_effects: Option<Vec<ContinuousEffect>> = None;

    // Track P/T through sublayers
    chars.record_base_pt();
    let mut power = chars.power;
    let mut toughness = chars.toughness;

    // Collect all Layer 7 effects that apply to this object
    let pt_effects: Vec<&ContinuousEffect> = effects
        .iter()
        .copied()
        .filter(|e| e.modification.layer() == Layer::PowerToughness)
        .collect();
    let needs_source_tracking = layer_needs_source_activity_tracking(
        &pt_effects,
        effects.iter().copied(),
        Layer::PowerToughness,
    );
    let tracked_source_ids =
        needs_source_tracking.then(|| tracked_source_ids_for_layer(&pt_effects));
    let mut source_state = if needs_source_tracking {
        let all_effects =
            all_effects.get_or_insert_with(|| effects.iter().map(|e| (*e).clone()).collect());
        build_object_baseline_for_ids(
            ctx.objects,
            all_effects,
            ctx.battlefield,
            ctx.game.commander_objects(),
            ctx.game,
            Layer::PowerToughness,
            None,
            tracked_source_ids
                .as_ref()
                .expect("tracked sources should exist when source tracking is enabled"),
        )
    } else {
        HashMap::new()
    };

    // Sort by sublayer with dependency handling inside each sublayer.
    let pt_effects = {
        if needs_baseline_dependency_sort(&pt_effects, ctx.game) {
            let all_effects =
                all_effects.get_or_insert_with(|| effects.iter().map(|e| (*e).clone()).collect());
            let baseline = build_layer_baseline(
                ctx.objects,
                all_effects,
                ctx.battlefield,
                ctx.game.commander_objects(),
                ctx.game,
                Layer::PowerToughness,
                None,
            );
            let started_groups_for_sort = crate::dependency::started_groups_for_sort(
                effects.iter().copied(),
                Layer::PowerToughness,
                &baseline,
                ctx.objects,
                ctx.game,
            );
            sort_layer_effects_with_baseline_and_started_groups(
                &pt_effects,
                &baseline,
                ctx.objects,
                ctx.game,
                &started_groups_for_sort,
            )
        } else {
            sort_layer_effects(&pt_effects)
        }
    };

    // Get counter timestamp for proper 7c ordering
    // Counters get a timestamp when the object enters the battlefield or when new counters are added
    let counter_timestamp = ctx.effects.get_latest_counter_timestamp(object.id);

    // Track whether we've applied counter modifications (for 7c ordering)
    let mut counters_applied = false;

    // Level-symbol P/T joins sublayer 7b in timestamp order (CR 711.2b).
    let level_timestamp = ctx.effects.get_object_timestamp(object.id).unwrap_or(0);
    let mut pending_level_pt = level_pt;

    // Apply in order, interleaving counters at the right point in 7c
    for effect in &pt_effects {
        if let Some((level_power, level_toughness)) = pending_level_pt
            && effect.modification.pt_sublayer().is_some_and(|sublayer| {
                sublayer > PtSublayer::Setting
                    || (sublayer == PtSublayer::Setting && effect.timestamp > level_timestamp)
            })
        {
            power = Some(level_power);
            toughness = Some(level_toughness);
            chars.base_power = power;
            chars.base_toughness = toughness;
            pending_level_pt = None;
            chars.power = power;
            chars.toughness = toughness;
            calc_guard.update(chars);
        }

        let effect_active = if needs_source_tracking {
            continuous_effect_group_started(effect, started_groups)
                || effect_source_is_active(effect, &source_state)
        } else {
            true
        };

        if effect.has_source_controller_context()
            && !ctx
                .objects
                .contains_key(&effect.source_controller_context_host())
        {
            continue;
        }
        let bound_effect = if needs_source_tracking && effect_active {
            bind_effect_controller_to_layer_frame(effect, &source_state)
        } else {
            std::borrow::Cow::Borrowed(*effect)
        };
        let effect = bound_effect.as_ref();

        if needs_source_tracking && effect_active {
            advance_layer_source_state(
                &mut source_state,
                effect,
                ctx.objects,
                ctx.battlefield,
                ctx.game.commander_objects(),
                ctx.game,
            );
        }

        if !effect_active
            || !effect_applies_to_or_started(effect, started_groups, object, chars, ctx)
        {
            continue;
        }

        mark_continuous_effect_group_started(effect, started_groups);

        let effect_sublayer = effect.modification.pt_sublayer();

        // If we're in sublayer 7c (Modifying) and counters haven't been applied yet,
        // check if we should apply them now based on timestamp
        if effect_sublayer == Some(PtSublayer::Modifying) && !counters_applied {
            // Apply counters before this effect if their timestamp is earlier
            if counter_timestamp.is_none_or(|ct| ct <= effect.timestamp) {
                apply_counter_modifications(
                    object,
                    &mut power,
                    &mut toughness,
                    &mut chars.numeric_range_error,
                );
                counters_applied = true;
                chars.power = power;
                chars.toughness = toughness;
                calc_guard.update(chars);
            }
        }

        // If we're past sublayer 7c (now in 7d Switching) and counters weren't applied,
        // apply them now (at the end of 7c)
        if effect_sublayer == Some(PtSublayer::Switching) && !counters_applied {
            apply_counter_modifications(
                object,
                &mut power,
                &mut toughness,
                &mut chars.numeric_range_error,
            );
            counters_applied = true;
            chars.power = power;
            chars.toughness = toughness;
            calc_guard.update(chars);
        }

        match &effect.modification {
            Modification::SetPower { value, .. } => {
                power = Some(crate::effects::helpers::value_eval::resolve_continuous_characteristic(
                    value,
                    super::value_context::LayerValueContext::new(ctx, effect.source, effect.controller)
                        .with_numeric_origin(effect.originating_ability.as_ref().map(|origin| &origin.ability)),
                    &mut chars.numeric_range_error,
                    &mut chars.numeric_choice_error,
                ));
            }
            Modification::SetToughness { value, .. } => {
                toughness = Some(crate::effects::helpers::value_eval::resolve_continuous_characteristic(
                    value,
                    super::value_context::LayerValueContext::new(ctx, effect.source, effect.controller)
                        .with_numeric_origin(effect.originating_ability.as_ref().map(|origin| &origin.ability)),
                    &mut chars.numeric_range_error,
                    &mut chars.numeric_choice_error,
                ));
            }
            Modification::SetPowerToughness {
                power: p,
                toughness: t,
                ..
            } => {
                let mut resolve = |value: &Value| {
                    crate::effects::helpers::value_eval::resolve_continuous_characteristic(
                        value,
                        super::value_context::LayerValueContext::new(ctx, effect.source, effect.controller)
                            .with_numeric_origin(effect.originating_ability.as_ref().map(|origin| &origin.ability)),
                        &mut chars.numeric_range_error,
                        &mut chars.numeric_choice_error,
                    )
                };
                power = Some(resolve(p));
                toughness = Some(resolve(t));
            }
            Modification::ModifyPower(delta) => {
                add_pt_checked(
                    &mut power,
                    i128::from(*delta),
                    &mut chars.numeric_range_error,
                    "power",
                );
            }
            Modification::ModifyToughness(delta) => {
                add_pt_checked(
                    &mut toughness,
                    i128::from(*delta),
                    &mut chars.numeric_range_error,
                    "toughness",
                );
            }
            Modification::ModifyPowerToughness {
                power: dp,
                toughness: dt,
            } => {
                add_pt_checked(
                    &mut power,
                    i128::from(*dp),
                    &mut chars.numeric_range_error,
                    "power",
                );
                add_pt_checked(
                    &mut toughness,
                    i128::from(*dt),
                    &mut chars.numeric_range_error,
                    "toughness",
                );
            }
            Modification::ModifyPowerToughnessValue {
                power: power_value,
                toughness: toughness_value,
            } => {
                let mut resolve = |value: &Value| {
                    crate::effects::helpers::value_eval::resolve_continuous_characteristic(
                        value,
                        super::value_context::LayerValueContext::new(ctx, effect.source, effect.controller)
                            .with_numeric_origin(effect.originating_ability.as_ref().map(|origin| &origin.ability)),
                        &mut chars.numeric_range_error,
                        &mut chars.numeric_choice_error,
                    )
                };
                let dp = resolve(power_value);
                let dt = resolve(toughness_value);
                add_pt_checked(
                    &mut power,
                    i128::from(dp),
                    &mut chars.numeric_range_error,
                    "power",
                );
                add_pt_checked(
                    &mut toughness,
                    i128::from(dt),
                    &mut chars.numeric_range_error,
                    "toughness",
                );
            }
            Modification::ModifyPowerToughnessByColorCount {
                power_multiplier,
                toughness_multiplier,
            } => {
                let color_count = chars.colors.count() as i32;
                add_pt_checked(
                    &mut power,
                    i128::from(*power_multiplier) * i128::from(color_count),
                    &mut chars.numeric_range_error,
                    "power",
                );
                add_pt_checked(
                    &mut toughness,
                    i128::from(*toughness_multiplier) * i128::from(color_count),
                    &mut chars.numeric_range_error,
                    "toughness",
                );
            }
            Modification::SwitchPowerToughness => {
                std::mem::swap(&mut power, &mut toughness);
            }
            Modification::CopyOf { .. }
            | Modification::ChangeController(_)
            | Modification::ChangeControllerToEffectController
            | Modification::ChangeText { .. }
            | Modification::RewriteText(_)
            | Modification::SetTextBox(_)
            | Modification::SetName(_)
            | Modification::InsertNameWords { .. }
            | Modification::AddCardTypes(_)
            | Modification::RemoveCardTypes(_)
            | Modification::SetCardTypes(_)
            | Modification::AddSubtypes(_)
            | Modification::AddAllSubtypesOfFamily(_)
            | Modification::RemoveSubtypes(_)
            | Modification::RemoveAllSubtypesOfFamily(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAuraAttachmentFilter(_)
            | Modification::AddSupertypes(_)
            | Modification::RemoveSupertypes(_)
            | Modification::RemoveAllCreatureTypes
            | Modification::AddColors(_)
            | Modification::RemoveColors(_)
            | Modification::SetColors(_)
            | Modification::MakeColorless
            | Modification::AddAbility(_)
            | Modification::AddAbilityGeneric(_)
            | Modification::SetAbilities(_)
            | Modification::CopyActivatedAbilities { .. }
            | Modification::CopyStaticAbilityVariants { .. }
            | Modification::CopyTriggeredAbilities { .. }
            | Modification::AddCombatDamageDrawAbility
            | Modification::RemoveAbility(_)
            | Modification::RemoveStaticAbilityFamily(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveLandRulesTextAbilities
            | Modification::RemoveAllAbilitiesExceptMana
            | Modification::Restriction(_) => {}
        }

        chars.power = power;
        chars.toughness = toughness;
        if effect_sublayer.is_some_and(|layer| layer <= PtSublayer::Setting) {
            chars.record_base_pt();
        }
        calc_guard.update(chars);
    }

    if let Some((level_power, level_toughness)) = pending_level_pt {
        power = Some(level_power);
        toughness = Some(level_toughness);
        chars.base_power = power;
        chars.base_toughness = toughness;
    }

    // If counters still haven't been applied (no 7c or 7d effects, or all 7c effects
    // had earlier timestamps), apply them now at the end of 7c
    if !counters_applied {
        apply_counter_modifications(
            object,
            &mut power,
            &mut toughness,
            &mut chars.numeric_range_error,
        );
    }

    chars.power = power;
    chars.toughness = toughness;
    calc_guard.update(chars);
}

/// Apply P/T counter modifications to power and toughness.
/// Per Rule 613.4c, these are part of sublayer 7c.
pub(super) fn apply_counter_modifications(
    object: &Object,
    power: &mut Option<i32>,
    toughness: &mut Option<i32>,
    error: &mut Option<(&'static str, i128)>,
) {
    let (mut power_delta, mut toughness_delta) = (0i128, 0i128);
    for (counter, count) in &object.counters {
        if let Some((power, toughness)) = counter.pt_delta() {
            power_delta += i128::from(power) * i128::from(*count);
            toughness_delta += i128::from(toughness) * i128::from(*count);
        }
    }
    add_pt_checked(power, power_delta, error, "power with counters");
    add_pt_checked(toughness, toughness_delta, error, "toughness with counters");
}

/// Check if an effect applies to a specific object.
///
/// Per Rules 611.2c and 611.3a:
/// - Resolution effects (from spells/abilities) only apply to locked targets
/// - Static ability effects apply to all objects matching their filter
pub(super) fn effect_applies_to(
    effect: &ContinuousEffect,
    object: &Object,
    chars: &CalculatedCharacteristics,
    ctx: &CalculationContext,
) -> bool {
    if effect_target_definitely_excludes_object(effect, object, ctx.objects) {
        return false;
    }
    if !continuous_effect_duration_and_condition_are_active(effect, ctx.game) {
        return false;
    }

    effect_target_applies_to_direct(
        effect,
        object,
        chars,
        ctx.objects,
        ctx.game,
        &std::cell::OnceCell::new(),
    )
}

pub(super) fn effect_applies_to_or_started(
    effect: &ContinuousEffect,
    started_groups: &HashSet<ContinuousEffectGroupId>,
    object: &Object,
    chars: &CalculatedCharacteristics,
    ctx: &CalculationContext,
) -> bool {
    if !continuous_effect_duration_is_active(effect, ctx.game) {
        return false;
    }

    if continuous_effect_group_started(effect, started_groups) {
        return true;
    }

    effect_applies_to(effect, object, chars, ctx)
}

pub(super) fn continuous_filter_context(
    game: &crate::game_state::GameState,
    controller: PlayerId,
    source: ObjectId,
) -> crate::target::FilterContext {
    let mut context = game.filter_context_for(controller, Some(source));
    if let Some(source_object) = game.object(source) {
        for (tag, snapshots) in &source_object.cast_tagged_objects {
            let retained = context.tagged_objects.entry(tag.clone()).or_default();
            for snapshot in snapshots {
                if retained
                    .iter()
                    .all(|existing| existing.stable_id != snapshot.stable_id)
                {
                    retained.push(snapshot.clone());
                }
            }
        }
    }
    let source_exiled = game
        .get_exiled_with_source_links(source)
        .iter()
        .filter_map(|id| {
            game.object(*id)
                .map(|obj| ObjectSnapshot::from_object(obj, game))
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        context
            .tagged_objects
            .insert(TagKey::from(SOURCE_EXILED_TAG), source_exiled);
    }
    context
}

pub(super) fn count_retained_tagged_snapshot_matches(
    filter: &ObjectFilter,
    game: &crate::game_state::GameState,
    filter_ctx: &crate::target::FilterContext,
) -> Option<i32> {
    let only_identity_tag_constraints = !filter.tagged_constraints.is_empty()
        && filter.tagged_constraints.iter().all(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
            )
        });
    if !only_identity_tag_constraints {
        return None;
    }

    let mut seen = std::collections::HashSet::new();
    let count = filter
        .tagged_constraints
        .iter()
        .filter_map(|constraint| filter_ctx.tagged_objects.get(&constraint.tag))
        .flatten()
        .filter(|snapshot| seen.insert(snapshot.stable_id))
        .filter(|snapshot| filter.matches_snapshot(snapshot, filter_ctx, game))
        .count();
    Some(count as i32)
}

pub(super) fn for_each_zone_candidate(
    ctx: &CalculationContext<'_>,
    zone: Zone,
    mut visitor: impl FnMut(&Object),
) {
    match zone {
        Zone::Battlefield => {
            for &id in &ctx.game.battlefield {
                if let Some(obj) = ctx.objects.get(&id) {
                    visitor(obj);
                }
            }
        }
        Zone::Graveyard => {
            for player in &ctx.game.players {
                for &id in &player.graveyard {
                    if let Some(obj) = ctx.objects.get(&id) {
                        visitor(obj);
                    }
                }
            }
        }
        Zone::Hand => {
            for player in &ctx.game.players {
                for &id in &player.hand {
                    if let Some(obj) = ctx.objects.get(&id) {
                        visitor(obj);
                    }
                }
            }
        }
        Zone::Library => {
            for player in &ctx.game.players {
                for &id in &player.library {
                    if let Some(obj) = ctx.objects.get(&id) {
                        visitor(obj);
                    }
                }
            }
        }
        Zone::Stack => {
            for entry in &ctx.game.stack {
                if let Some(obj) = ctx.objects.get(&entry.object_id) {
                    visitor(obj);
                }
            }
        }
        Zone::Exile => {
            for &id in &ctx.game.exile {
                if let Some(obj) = ctx.objects.get(&id) {
                    visitor(obj);
                }
            }
        }
        Zone::Command => {
            for &id in &ctx.game.command_zone {
                if let Some(obj) = ctx.objects.get(&id) {
                    visitor(obj);
                }
            }
        }
        Zone::Ante => {
            for &id in &ctx.game.ante {
                if let Some(obj) = ctx.objects.get(&id) {
                    visitor(obj);
                }
            }
        }
        Zone::OutsideGame => {
            for player in &ctx.game.players {
                for &id in &player.sideboard {
                    if let Some(obj) = ctx.objects.get(&id) {
                        visitor(obj);
                    }
                }
            }
        }
    }
}

pub(super) fn for_each_filter_candidate(
    ctx: &CalculationContext<'_>,
    filter: &ObjectFilter,
    mut visitor: impl FnMut(&Object),
) {
    // Fast path: explicit zone filters and default-battlefield filters can be
    // scanned directly without allocating a candidate ID vector.
    if let Some(zone) = filter.zone {
        for_each_zone_candidate(ctx, zone, visitor);
        return;
    }
    if filter.any_of.is_empty() {
        for_each_zone_candidate(ctx, Zone::Battlefield, visitor);
        return;
    }

    for id in candidate_ids_for_filter(ctx.game, filter) {
        if let Some(obj) = ctx.objects.get(&id) {
            visitor(obj);
        }
    }
}

pub(super) fn count_filter_matches(
    filter: &ObjectFilter,
    ctx: &CalculationContext<'_>,
    filter_ctx: &crate::target::FilterContext,
) -> i32 {
    if let Some(count) = count_retained_tagged_snapshot_matches(filter, ctx.game, filter_ctx) {
        return count;
    }
    let mut count = 0i32;
    for_each_filter_candidate(ctx, filter, |obj| {
        let matches = ctx
            .effects
            .calculate_characteristics(obj.id, ctx.objects, ctx.battlefield, ctx.game)
            .is_some_and(|chars| {
                filter_matches_with_characteristics_in_context(
                    filter, obj, &chars, ctx.game, filter_ctx,
                )
            });
        if matches {
            count += 1;
        }
    });
    count
}

use super::*;

pub(super) fn continuous_value_players(
    ctx: &CalculationContext<'_>,
    player_filter: &PlayerFilter,
    controller: PlayerId,
    source: ObjectId,
) -> Vec<PlayerId> {
    let filter_ctx = continuous_filter_context(ctx.game, controller, source);
    let extreme = match player_filter {
        PlayerFilter::MostLifeTied => ctx
            .game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.life)
            .max(),
        PlayerFilter::LowestLifeTied => ctx
            .game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.life)
            .min(),
        _ => None,
    };
    let most_cards = matches!(player_filter, PlayerFilter::MostCardsInHand)
        .then(|| {
            ctx.game
                .players
                .iter()
                .filter(|player| player.is_in_game())
                .map(|player| player.hand.len())
                .max()
        })
        .flatten();

    ctx.game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .filter(|player| match player_filter {
            PlayerFilter::EffectController => player.id == controller,
            PlayerFilter::MostLifeTied | PlayerFilter::LowestLifeTied => {
                extreme.is_some_and(|life| player.life == life)
            }
            PlayerFilter::MostCardsInHand => {
                most_cards.is_some_and(|cards| player.hand.len() == cards)
            }
            PlayerFilter::CastCardTypeThisTurn(card_type) => ctx
                .game
                .turn_store
                .turn_history
                .spell_cast_snapshot_history()
                .iter()
                .any(|snapshot| {
                    snapshot.controller == player.id && snapshot.card_types.contains(card_type)
                }),
            _ => crate::filter::player_filter_matches_game(
                player_filter,
                player.id,
                ctx.game,
                &filter_ctx,
            ),
        })
        .map(|player| player.id)
        .collect()
}

pub(super) fn required_continuous_value_players(
    value: &Value,
    ctx: &CalculationContext<'_>,
    player_filter: &PlayerFilter,
    controller: PlayerId,
    source: ObjectId,
) -> Vec<PlayerId> {
    let players = continuous_value_players(ctx, player_filter, controller, source);
    if players.is_empty() {
        unsupported_continuous_value(
            value,
            "player filter has no player available in continuous-effect context",
        );
    }
    players
}

pub(super) fn for_each_matching_continuous_object(
    ctx: &CalculationContext<'_>,
    filter: &ObjectFilter,
    controller: PlayerId,
    source: ObjectId,
    mut visitor: impl FnMut(&Object, &CalculatedCharacteristics),
) {
    for_each_filter_candidate(ctx, filter, |object| {
        let Some(chars) = ctx.effects.calculate_characteristics(
            object.id,
            ctx.objects,
            ctx.battlefield,
            ctx.game,
        ) else {
            return;
        };
        if filter_matches_with_characteristics(filter, object, &chars, ctx.game, controller, source)
        {
            visitor(object, &chars);
        }
    });
}

pub(super) fn greatest_shared_creature_type_count_for_filter(
    ctx: &CalculationContext<'_>,
    filter: &ObjectFilter,
    controller: PlayerId,
    source: ObjectId,
) -> i32 {
    let mut subtype_sets = Vec::new();
    for_each_matching_continuous_object(ctx, filter, controller, source, |_, chars| {
        subtype_sets.push(chars.subtypes.to_vec());
    });
    crate::effects::helpers::greatest_shared_creature_type_count(subtype_sets)
}

pub(super) fn unsupported_continuous_value(value: &Value, reason: &str) -> ! {
    panic!("unsupported continuous-effect value {value:?}: {reason}")
}

/// Resolve a Value to an i32 for continuous effect calculations.
///
/// This is used during layer system calculations where we have access to game objects
/// but not a full ExecutionContext. Handles common computed values like Count and SourcePower.
pub(super) fn resolve_value_with_context(
    value: &Value,
    ctx: &CalculationContext<'_>,
    source: ObjectId,
    controller: PlayerId,
) -> i32 {
    crate::effects::helpers::value_eval::resolve_continuous(
        value,
        super::value_context::LayerValueContext::new(ctx, source, controller),
    )
}

/// The objects a CR 613.8 dependency baseline has to cover.
///
/// The baseline answers one question — does applying this effect change which
/// objects that one applies to — so it needs exactly the objects some effect
/// can reach. Nearly every effect names the battlefield, which leaves both
/// libraries and both hands out of a calculation that would only discard them,
/// and those are most of the objects in a real game.
pub(super) struct BaselineScope {
    everything: bool,
    zones: Vec<Zone>,
    ids: Vec<ObjectId>,
}

impl BaselineScope {
    fn covers(&self, object: &crate::object::Object) -> bool {
        self.covers_object(object.zone, object.id)
    }

    fn covers_object(&self, zone: Zone, id: ObjectId) -> bool {
        self.everything || self.zones.contains(&zone) || self.ids.contains(&id)
    }
}

/// Read the zones and objects `effects` can reach.
///
/// Cards that work from a hidden zone say which one: Arcane Adaptation's
/// "creature cards you own that aren't on the battlefield" compiles to filters
/// naming `Hand` and `Library`, and those zones come back in here. A filter
/// that names no zone at all is unrestricted rather than implicitly a
/// battlefield filter, so it gives up the narrowing instead of guessing.
pub(super) fn baseline_scope(effects: &[ContinuousEffect]) -> BaselineScope {
    let mut scope = BaselineScope {
        everything: false,
        // A spell is an object effects routinely reach, and a zone-named filter
        // can still match one through its cast origin.
        zones: vec![Zone::Battlefield, Zone::Stack],
        ids: Vec::new(),
    };
    for effect in effects {
        if effect.has_source_controller_context()
            && !scope.ids.contains(&effect.source_controller_context_host())
        {
            scope.ids.push(effect.source_controller_context_host());
        }
        match &effect.applies_to {
            // An attachment names itself here; whatever it is attached to is a
            // permanent or a player, so the battlefield entry already covers
            // the object the effect lands on.
            EffectTarget::Specific(id) | EffectTarget::AttachedTo(id) => {
                if !scope.ids.contains(id) {
                    scope.ids.push(*id);
                }
            }
            EffectTarget::Source => {
                if !scope.ids.contains(&effect.source_controller_context_host()) {
                    scope.ids.push(effect.source);
                }
            }
            EffectTarget::AllPermanents | EffectTarget::AllCreatures => {}
            EffectTarget::Filter(filter) => match filter.zone {
                Some(zone) => {
                    if !scope.zones.contains(&zone) {
                        scope.zones.push(zone);
                    }
                }
                None => {
                    scope.everything = true;
                    return scope;
                }
            },
        }
    }
    scope
}

pub(super) fn build_layer_baseline(
    objects: &ObjectMap,
    effects: &[ContinuousEffect],
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    game: &crate::game_state::GameState,
    layer: Layer,
    sublayer: Option<PtSublayer>,
) -> HashMap<ObjectId, CalculatedCharacteristics> {
    let mut filtered: Vec<ContinuousEffect> = Vec::with_capacity(effects.len());
    for effect in effects {
        let effect_layer = effect.modification.layer();
        let include = if effect_layer < layer {
            true
        } else if layer == Layer::PowerToughness && effect_layer == Layer::PowerToughness {
            if let Some(current_sublayer) = sublayer {
                effect.modification.pt_sublayer() < Some(current_sublayer)
            } else {
                false
            }
        } else {
            false
        };

        if include {
            filtered.push(effect.clone());
        }
    }

    let scope = baseline_scope(effects);
    let mut baseline = HashMap::with_capacity(objects.len());
    for &id in objects.keys() {
        if !objects.get(&id).is_some_and(|object| scope.covers(object)) {
            continue;
        }
        if let Some(chars) = calculate_characteristics_with_effects_simple_internal(
            id,
            objects,
            &filtered,
            battlefield,
            commanders,
            game,
            layer > Layer::Ability,
        ) {
            baseline.insert(id, chars);
        }
    }

    baseline
}

pub(super) fn build_object_baseline_for_ids(
    objects: &ObjectMap,
    effects: &[ContinuousEffect],
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    game: &crate::game_state::GameState,
    layer: Layer,
    sublayer: Option<PtSublayer>,
    ids: &HashSet<ObjectId>,
) -> HashMap<ObjectId, CalculatedCharacteristics> {
    let mut filtered: Vec<ContinuousEffect> = Vec::with_capacity(effects.len());
    for effect in effects {
        let effect_layer = effect.modification.layer();
        let include = if effect_layer < layer {
            true
        } else if layer == Layer::PowerToughness && effect_layer == Layer::PowerToughness {
            if let Some(current_sublayer) = sublayer {
                effect.modification.pt_sublayer() < Some(current_sublayer)
            } else {
                false
            }
        } else {
            false
        };

        if include {
            filtered.push(effect.clone());
        }
    }

    let mut baseline = HashMap::with_capacity(ids.len());
    for &id in ids {
        if !objects.contains_key(&id) {
            continue;
        }
        if let Some(chars) = calculate_characteristics_with_effects_simple_internal(
            id,
            objects,
            &filtered,
            battlefield,
            commanders,
            game,
            layer > Layer::Ability,
        ) {
            baseline.insert(id, chars);
        }
    }

    baseline
}

pub(super) fn effect_can_change_static_ability_presence(effect: &ContinuousEffect) -> bool {
    matches!(
        effect.modification,
        Modification::CopyOf { .. }
            | Modification::SetTextBox(_)
            | Modification::SetSubtypes(_)
            | Modification::SetAuraAttachmentFilter(_)
            | Modification::SetAbilities(_)
            | Modification::RemoveAbility(_)
            | Modification::RemoveStaticAbilityFamily(_)
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveLandRulesTextAbilities
            | Modification::RemoveAllAbilitiesExceptMana
    )
}

pub(super) fn layer_needs_source_activity_tracking<'a>(
    layer_effects: &[&ContinuousEffect],
    all_effects: impl IntoIterator<Item = &'a ContinuousEffect>,
    layer: Layer,
) -> bool {
    let has_static_sources = layer_effects
        .iter()
        .any(|effect| effect.originating_static_ability.is_some());
    has_static_sources
        && all_effects.into_iter().any(|effect| {
            // Control affects the context read by a source's static abilities even
            // when none of those abilities is removed. Track earlier-layer source
            // facts as well when a later control effect exists.
            effect.modification.layer() == Layer::Control
                || (effect.modification.layer() <= layer
                    && effect_can_change_static_ability_presence(effect))
        })
}

/// Bind a regenerated static descriptor against a prepared layer frame.
/// Its active-source check and baseline scope guarantee the host is present;
/// this internal invariant is not a substitute for discovery error handling.
pub(crate) fn bind_effect_controller_to_layer_frame<'a>(
    effect: &'a ContinuousEffect,
    frame: &HashMap<ObjectId, CalculatedCharacteristics>,
) -> std::borrow::Cow<'a, ContinuousEffect> {
    if !effect.has_source_controller_context() {
        return std::borrow::Cow::Borrowed(effect);
    }
    let controller = frame[&effect.source_controller_context_host()].controller;
    if controller == effect.controller {
        return std::borrow::Cow::Borrowed(effect);
    }
    let mut bound = effect.clone();
    bound.controller = controller;
    std::borrow::Cow::Owned(bound)
}

pub(super) fn tracked_source_ids_for_layer(
    layer_effects: &[&ContinuousEffect],
) -> HashSet<ObjectId> {
    let mut ids = HashSet::new();
    for effect in layer_effects {
        if effect.originating_static_ability.is_some() {
            ids.insert(effect.source);
        }
        if effect.has_source_controller_context() {
            ids.insert(effect.source_controller_context_host());
        }
    }
    ids
}

pub(super) fn effect_source_is_active(
    effect: &ContinuousEffect,
    source_state: &HashMap<ObjectId, CalculatedCharacteristics>,
) -> bool {
    let Some(originating_static_ability) = &effect.originating_static_ability else {
        return true;
    };

    source_state.get(&effect.source).is_some_and(|chars| {
        if chars
            .static_abilities
            .iter()
            .any(|ability| ability.instance_id() == originating_static_ability.instance_id())
        {
            return true;
        }
        // At the start of layer six, active level-tier abilities have not
        // yet been appended. Their generating occurrence remains active
        // while its paired level description survives the earlier layers.
        if let Some(origin) = effect.originating_ability.as_deref()
            && let AbilityOrigin::Level {
                parent, tier, slot, ..
            } = &origin.ability
        {
            return chars.abilities.iter().enumerate().any(|(index, ability)| {
                if chars.abilities.origin(index) != Some(parent.as_ref()) {
                    return false;
                }
                let AbilityKind::Static(description) = &ability.kind else {
                    return false;
                };
                description
                    .level_abilities()
                    .and_then(|tiers| tiers.get(*tier))
                    .and_then(|band| band.abilities.get(*slot))
                    .is_some_and(|granted| {
                        granted.instance_id() == originating_static_ability.instance_id()
                    })
            });
        }
        false
    })
}

pub(super) fn advance_layer_source_state(
    source_state: &mut HashMap<ObjectId, CalculatedCharacteristics>,
    effect: &ContinuousEffect,
    objects: &ObjectMap,
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    game: &crate::game_state::GameState,
) {
    let tracked_ids: Vec<ObjectId> = source_state.keys().copied().collect();
    for id in tracked_ids {
        let Some(object) = objects.get(&id) else {
            continue;
        };
        let Some(chars) = source_state.get(&id).cloned() else {
            continue;
        };

        if !effect_applies_to_direct(
            effect,
            object,
            &chars,
            objects,
            battlefield,
            commanders,
            game,
        ) {
            continue;
        }

        let mut updated = chars;
        crate::dependency::apply_continuous_effect_to_chars_for_dependency(
            effect,
            &mut updated,
            object,
            game,
        );
        source_state.insert(id, updated);
    }
}

pub(super) fn advance_layer_batch_source_state(
    source_state: &mut HashMap<ObjectId, CalculatedCharacteristics>,
    effect: &ContinuousEffect,
    objects: &ObjectMap,
    battlefield: &[ObjectId],
    commanders: &HashSet<ObjectId>,
    game: &crate::game_state::GameState,
    started_groups_by_object: &HashSet<(ContinuousEffectGroupId, ObjectId)>,
    source_active: bool,
) {
    let tracked_ids: Vec<ObjectId> = source_state.keys().copied().collect();
    for id in tracked_ids {
        let group_started =
            continuous_effect_group_started_for_object(effect, id, started_groups_by_object);
        if !source_active && !group_started {
            continue;
        }
        let Some(object) = objects.get(&id) else {
            continue;
        };
        let Some(chars) = source_state.get(&id).cloned() else {
            continue;
        };
        if !group_started
            && !effect_applies_to_direct(
                effect,
                object,
                &chars,
                objects,
                battlefield,
                commanders,
                game,
            )
        {
            continue;
        }

        let mut updated = chars;
        crate::dependency::apply_continuous_effect_to_chars_for_dependency(
            effect,
            &mut updated,
            object,
            game,
        );
        source_state.insert(id, updated);
    }
}

/// Exalted counters compile as a named counter kind (CR 122.1b).
fn is_exalted_counter(counter_type: CounterType) -> bool {
    matches!(counter_type, CounterType::Named(name) if name.eq_ignore_ascii_case("exalted"))
}

/// Add abilities from ability-granting counters (deathtouch counter, flying counter, etc.).
///
/// Per MTG rules, counters like "deathtouch counter" grant the ability to the permanent.
/// This is different from +1/+1 counters which modify P/T directly.
fn add_ability_from_counter(
    object: &Object,
    counter_type: CounterType,
    chars: &mut CalculatedCharacteristics,
) {
    for occurrence in object.counters.ability_occurrences(counter_type) {
        for (slot, ability) in occurrence.abilities.iter().enumerate() {
            if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
                chars.static_abilities.push(static_ability.clone());
            }
            chars.abilities.push_with_origin(
                ability.clone(),
                super::AbilityOrigin::Counter {
                    occurrence: occurrence.origin.clone(),
                    slot,
                },
            );
        }
    }
}

#[cfg(test)]
pub(super) fn add_abilities_from_counters(object: &Object, chars: &mut CalculatedCharacteristics) {
    for &counter_type in object.counters.keys() {
        add_ability_from_counter(object, counter_type, chars);
    }
}

pub(super) fn ability_counter_timestamps(
    object: &Object,
    manager: &ContinuousEffectManager,
) -> Vec<(u64, CounterType)> {
    let mut counters: Vec<_> = object
        .counters
        .iter()
        .filter(|&(&counter_type, &count)| {
            count > 0
                && (counter_type == CounterType::Decayed
                    || is_exalted_counter(counter_type)
                    || counter_type.granted_ability().is_some())
        })
        .map(|(&counter_type, _)| {
            (
                manager
                    .get_counter_timestamp(object.id, counter_type)
                    .unwrap_or(0),
                counter_type,
            )
        })
        .collect();
    counters.sort_by(
        |(left_timestamp, left_counter), (right_timestamp, right_counter)| {
            left_timestamp
                .cmp(right_timestamp)
                .then_with(|| left_counter.description().cmp(&right_counter.description()))
        },
    );
    counters
}

pub(super) fn apply_ability_counters_through(
    object: &Object,
    chars: &mut CalculatedCharacteristics,
    counters: &[(u64, CounterType)],
    next_counter: &mut usize,
    through_timestamp: Option<u64>,
) {
    while let Some(&(timestamp, counter_type)) = counters.get(*next_counter) {
        if through_timestamp.is_some_and(|through| timestamp > through) {
            break;
        }
        add_ability_from_counter(object, counter_type, chars);
        *next_counter += 1;
    }
}

pub(super) fn add_temporary_static_ability_grants(
    object: &Object,
    chars: &mut CalculatedCharacteristics,
    current_turn: u32,
) {
    for (index, grant) in object.temporary_static_ability_grants.iter().enumerate() {
        // Retained registrations can outlive cleanup, including after restore.
        // Expiry determines applicability; storage pruning is not a prerequisite.
        // Keep the original slot so surviving grants retain their exact origin.
        if grant.is_expired(current_turn) {
            continue;
        }
        let Some(ability) = grant.materialize() else {
            continue;
        };
        // Materialize each registered grant as its own ability occurrence.
        // Equal definitions (including cloned runtime values) are not one
        // grant: their replacement effects can each apply to the same event.
        let origin = object
            .temporary_static_ability_grants
            .origin(index)
            .expect("temporary grant and origin remain paired")
            .clone();
        chars.abilities.push_with_origin(
            Ability::static_ability(ability.clone()),
            super::AbilityOrigin::Temporary(origin),
        );
        chars.static_abilities.push(ability);
    }
}

/// Get P/T override from level abilities if applicable.
pub(super) fn get_level_ability_pt(
    object: &Object,
    abilities: &CalculatedAbilities,
) -> Option<(i32, i32)> {
    let level_count = object
        .counters
        .get(&CounterType::Level)
        .copied()
        .unwrap_or(0);

    for ability in abilities.iter() {
        if let AbilityKind::Static(s) = &ability.kind
            && let Some(levels) = s.level_abilities()
        {
            // Find the matching tier (highest tier that applies)
            for tier in levels.iter().rev() {
                if tier.applies_at_level(level_count) {
                    return tier.power_toughness;
                }
            }
        }
    }
    None
}

/// Apply the active level tier, including object abilities stored inside a
/// source-filtered static carrier because `LevelAbility` itself stores only
/// static abilities.
pub(super) fn apply_level_granted_abilities(
    object: &Object,
    chars: &mut CalculatedCharacteristics,
) {
    let level = object
        .counters
        .get(&CounterType::Level)
        .copied()
        .unwrap_or(0);
    // Layer six owns which level descriptions survive. Snapshot them before
    // adding their tier abilities, keeping each independent paired occurrence.
    let active = chars
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(index, ability)| {
            let AbilityKind::Static(ability) = &ability.kind else {
                return None;
            };
            let tiers = ability.level_abilities()?;
            let parent = chars.abilities.origin(index)?.clone();
            let (tier, band) = tiers
                .iter()
                .enumerate()
                .rev()
                .find(|(_, band)| band.applies_at_level(level))?;
            Some((parent, tier, band.abilities.clone()))
        })
        .collect::<Vec<_>>();
    for (parent, tier, abilities) in active {
        for (slot, granted) in abilities.into_iter().enumerate() {
            let origin = AbilityOrigin::Level {
                printed_face: object.card,
                parent: Box::new(parent.clone()),
                tier,
                slot,
            };
            // Source-filtered inline carriers are materialized by their
            // generated continuous effect, once; they are not expanded here.
            chars
                .abilities
                .push_with_origin(Ability::static_ability(granted.clone()), origin);
            chars.static_abilities.push(granted);
        }
    }
}

#[cfg(test)]
mod baseline_scope_tests {
    use super::{BaselineScope, baseline_scope};
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::ids::{ObjectId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    fn effect(applies_to: EffectTarget) -> ContinuousEffect {
        ContinuousEffect::new(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            applies_to,
            Modification::AddAbility(crate::static_abilities::StaticAbility::flying()),
        )
    }

    fn covers(scope: &BaselineScope, zone: Zone) -> bool {
        scope.covers_object(zone, ObjectId::from_raw(999))
    }

    #[test]
    fn ordinary_battlefield_effects_leave_hidden_zones_out() {
        let scope = baseline_scope(&[
            effect(EffectTarget::Filter(
                ObjectFilter::creature().in_zone(Zone::Battlefield),
            )),
            effect(EffectTarget::AllCreatures),
            effect(EffectTarget::Source),
        ]);
        assert!(covers(&scope, Zone::Battlefield));
        assert!(!covers(&scope, Zone::Hand));
        assert!(!covers(&scope, Zone::Library));
        assert!(!covers(&scope, Zone::Graveyard));
    }

    #[test]
    fn a_filter_keeps_the_zone_it_names() {
        // Arcane Adaptation reaches creature cards you own that aren't on the
        // battlefield, and compiles to filters naming those zones.
        let scope = baseline_scope(&[
            effect(EffectTarget::Filter(
                ObjectFilter::creature().in_zone(Zone::Hand),
            )),
            effect(EffectTarget::Filter(
                ObjectFilter::creature().in_zone(Zone::Library),
            )),
        ]);
        assert!(covers(&scope, Zone::Hand));
        assert!(covers(&scope, Zone::Library));
        assert!(covers(&scope, Zone::Battlefield));
        assert!(!covers(&scope, Zone::Graveyard));
    }

    #[test]
    fn an_unrestricted_filter_gives_up_the_narrowing() {
        let mut unrestricted = ObjectFilter::creature();
        unrestricted.zone = None;
        let scope = baseline_scope(&[effect(EffectTarget::Filter(unrestricted))]);
        for zone in [
            Zone::Battlefield,
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ] {
            assert!(
                covers(&scope, zone),
                "an unrestricted filter must cover {zone:?}"
            );
        }
    }

    #[test]
    fn an_individually_named_object_is_covered_wherever_it_is() {
        let hidden = ObjectId::from_raw(77);
        let scope = baseline_scope(&[effect(EffectTarget::Specific(hidden))]);
        assert!(scope.covers_object(Zone::Library, hidden));
        assert!(!scope.covers_object(Zone::Library, ObjectId::from_raw(78)));
    }
}
