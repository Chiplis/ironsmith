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
    let mut effect_manager = ContinuousEffectManager::new();
    for effect in effects {
        effect_manager.add_effect(effect.clone());
    }
    let calculation = CalculationContext {
        objects,
        effects: &effect_manager,
        battlefield,
        game,
        current_object: source,
    };
    let layer = super::value_context::LayerValueContext::direct(
        &calculation,
        source,
        controller,
        effects,
        commanders,
    );
    crate::effects::helpers::value_eval::resolve_continuous(value, layer)
}

/// Apply all layers to calculate final characteristics.
pub(super) fn calculate_with_layers(
    object: &Object,
    ctx: &CalculationContext,
) -> CalculatedCharacteristics {
    use crate::dependency::needs_baseline_dependency_sort;
    use crate::dependency::sort_layer_effects;
    use crate::dependency::sort_layer_effects_with_baseline_and_started_groups;

    let mut chars = initial_characteristics(object);
    if chars.world_supertype_since.is_some() {
        chars.world_supertype_since = ctx.effects.get_entry_timestamp(object.id).or(Some(0));
    }
    let calc_guard = CharacteristicCalculationGuard::begin(object.id, &chars);
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
        let layer_effects = match effects_by_layer.get(&layer) {
            Some(effects) => effects,
            None => {
                if layer == Layer::Copy {
                    let had_world = chars.supertypes.contains(&Supertype::World);
                    apply_face_down_layer(object, &mut chars);
                    update_world_supertype_since(
                        &mut chars,
                        had_world,
                        ctx.effects.get_entry_timestamp(object.id).unwrap_or(0),
                    );
                    calc_guard.update(&chars);
                }
                if layer == Layer::Type {
                    apply_reconfigure_attached_type_rule(object, &mut chars);
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
                sort_layer_effects_with_baseline_and_started_groups(
                    layer_effects,
                    baseline,
                    ctx.objects,
                    ctx.game,
                    &started_groups,
                )
            } else {
                sort_layer_effects(layer_effects)
            }
        };

        // Apply effects in dependency order
        for effect in sorted_effects {
            if layer == Layer::Ability {
                apply_ability_counters_through(
                    object,
                    &mut chars,
                    &ability_counters,
                    &mut next_ability_counter,
                    Some(effect.timestamp),
                );
                prune_ability_gain_prohibitions(&mut chars);
                calc_guard.update(&chars);
            }
            let effect_active = if needs_source_tracking {
                continuous_effect_group_started(effect, &started_groups)
                    || effect_source_is_active(effect, &source_state)
            } else {
                true
            };

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
                    );
                }

                // Layer 2: Control
                Modification::ChangeController(new_controller) => {
                    chars.controller = *new_controller;
                }
                Modification::ChangeText { .. } => {
                    // Text changes are handled separately.
                }
                Modification::SetTextBox(overlay) => {
                    chars.compiled_card_text = overlay.compiled_card_text.clone();
                    chars.abilities = overlay.abilities.clone().into();
                    chars.static_abilities = extract_static_abilities(&overlay.abilities).into();
                }
                Modification::SetName(name) => {
                    chars.name = name.clone().into();
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
                    remove_card_types_and_prune_subtypes(&mut chars.card_types, &mut chars.subtypes, types);
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
                    // Blood Moon: Only replace LAND subtypes, keep non-land subtypes
                    // Per MTG rules, type-changing effects that set land types only affect
                    // land subtypes (Plains, Island, Swamp, Mountain, Forest, Urza's, etc.)
                    // Non-land subtypes (Saga, Aura, creature types) are preserved.

                    replace_subtypes_in_family(&mut chars.subtypes, types, SubtypeFamily::Land);
                }
                Modification::SetAuraAttachmentFilter(filter) => {
                    chars.aura_attach_filter = Some(filter.clone());
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
                    push_static_ability_once(&mut chars, ability.clone());
                }
                Modification::AddAbilityGeneric(ability) => {
                    let bound_ability =
                        bind_effect_controller_in_ability(ability, effect.controller);
                    if let AbilityKind::Static(static_ability) = &bound_ability.kind {
                        push_static_ability_once(&mut chars, static_ability.clone());
                    } else {
                        chars.abilities.push(bound_ability);
                    }
                }
                Modification::SetAbilities(abilities) => {
                    chars.abilities = abilities.clone().into();
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
                    use crate::static_ability_processor::get_all_continuous_effects;

                    let effects = get_all_continuous_effects(ctx.game);
                    let mut candidate_ids: Vec<_> = ctx.objects.keys().copied().collect();
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

                        if !filter_matches_with_characteristics(
                            filter,
                            candidate,
                            &candidate_chars,
                            ctx.game,
                            effect.controller,
                            effect.source,
                        ) {
                            continue;
                        }

                        for ability in &candidate_chars.abilities {
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
                                activated.timing = crate::ability::ActivationTiming::OncePerTurn;
                            }
                            chars.abilities.push(copied);
                        }
                    }
                }
                Modification::CopyStaticAbilityVariants {
                    filter,
                    selectors,
                    exclude_source_id,
                } => {
                    use crate::static_ability_processor::get_all_continuous_effects;

                    let effects = get_all_continuous_effects(ctx.game);
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
                        effect.controller,
                        effect.source,
                    );
                }
                Modification::CopyTriggeredAbilities {
                    filter,
                    exclude_source_name,
                    exclude_source_id,
                } => {
                    use crate::ability::AbilityKind;
                    use crate::static_ability_processor::get_all_continuous_effects;

                    let effects = get_all_continuous_effects(ctx.game);
                    let mut candidate_ids: Vec<_> = ctx.objects.keys().copied().collect();
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

                        if !filter_matches_with_characteristics(
                            filter,
                            candidate,
                            &candidate_chars,
                            ctx.game,
                            effect.controller,
                            effect.source,
                        ) {
                            continue;
                        }

                        for ability in &candidate_chars.abilities {
                            if matches!(ability.kind, AbilityKind::Triggered(_)) {
                                chars.abilities.push(ability.clone());
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
                    chars.static_abilities.retain(|candidate| {
                        candidate != ability
                            && !(ability.id() == crate::static_abilities::StaticAbilityId::Banding
                                && candidate.id()
                                    == crate::static_abilities::StaticAbilityId::BandsWithOther)
                    });
                }
                Modification::RemoveAbilityGeneric { ability, .. } => {
                    chars
                        .abilities
                        .retain(|candidate| !object_ability_matches_loss(candidate, ability));
                    if let AbilityKind::Static(static_ability) = &ability.kind {
                        chars.static_abilities.retain(|candidate| {
                            candidate != static_ability
                                && !(static_ability.id()
                                    == crate::static_abilities::StaticAbilityId::Banding
                                    && candidate.id()
                                        == crate::static_abilities::StaticAbilityId::BandsWithOther)
                        });
                    }
                }
                Modification::RemoveAllAbilities => {
                    chars.abilities.clear();
                    chars.static_abilities.clear();
                    abilities_removed = true;
                }
                Modification::RemoveAllAbilitiesExceptMana => {
                    chars
                        .abilities
                        .retain(|ability| ability_is_mana_for_object(ability, ctx.game, object));
                    chars.static_abilities.clear();
                    abilities_removed = true;
                }
                Modification::CantBeBlocked => {
                    push_static_ability_once(&mut chars, StaticAbility::unblockable());
                }
                Modification::CantAttack => {
                    push_static_ability_once(&mut chars, StaticAbility::defender());
                }
                Modification::CantBlock => {
                    push_static_ability_once(&mut chars, StaticAbility::cant_block());
                }
                Modification::DoesntUntap => {
                    push_static_ability_once(&mut chars, StaticAbility::doesnt_untap());
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
            update_world_supertype_since(
                &mut chars,
                had_world,
                ctx.effects.get_entry_timestamp(object.id).unwrap_or(0),
            );
            calc_guard.update(&chars);
        } else if layer == Layer::Type {
            apply_reconfigure_attached_type_rule(object, &mut chars);
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

    // Check for LevelAbilities if abilities weren't removed
    let level_pt = if !abilities_removed {
        get_level_ability_pt(object)
    } else {
        None
    };

    // If level abilities set P/T, use that as the "base" for layer 7b
    if let Some((lp, lt)) = level_pt {
        chars.power = Some(lp);
        chars.toughness = Some(lt);
        calc_guard.update(&chars);
    }

    // Apply Layer 7 effects in sublayer order
    apply_layer_7_effects(
        object,
        ctx,
        &mut chars,
        abilities_removed,
        &calc_guard,
        &mut started_groups,
    );

    // Add abilities from level tiers if not removed
    if !abilities_removed {
        apply_level_granted_abilities(object, &mut chars);
        prune_ability_gain_prohibitions(&mut chars);
        calc_guard.update(&chars);
    }

    add_intrinsic_basic_land_mana_abilities(&mut chars);
    prune_ability_gain_prohibitions(&mut chars);
    calc_guard.update(&chars);

    retain_active_static_abilities(&mut chars, ctx.game, object.id);
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
) {
    use crate::dependency::needs_baseline_dependency_sort;
    use crate::dependency::sort_layer_effects;
    use crate::dependency::sort_layer_effects_with_baseline_and_started_groups;

    let effects = ctx.effects.effects_sorted();
    let mut all_effects: Option<Vec<ContinuousEffect>> = None;

    // Track P/T through sublayers
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
            sort_layer_effects_with_baseline_and_started_groups(
                &pt_effects,
                &baseline,
                ctx.objects,
                ctx.game,
                started_groups,
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

    // Apply in order, interleaving counters at the right point in 7c
    for effect in &pt_effects {
        let effect_active = if needs_source_tracking {
            continuous_effect_group_started(effect, started_groups)
                || effect_source_is_active(effect, &source_state)
        } else {
            true
        };

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
                apply_counter_modifications(object, &mut power, &mut toughness);
                counters_applied = true;
                chars.power = power;
                chars.toughness = toughness;
                calc_guard.update(chars);
            }
        }

        // If we're past sublayer 7c (now in 7d Switching) and counters weren't applied,
        // apply them now (at the end of 7c)
        if effect_sublayer == Some(PtSublayer::Switching) && !counters_applied {
            apply_counter_modifications(object, &mut power, &mut toughness);
            counters_applied = true;
            chars.power = power;
            chars.toughness = toughness;
            calc_guard.update(chars);
        }

        match &effect.modification {
            Modification::SetPower { value, .. } => {
                power = Some(resolve_value_with_context(
                    value,
                    ctx,
                    effect.source,
                    effect.controller,
                ));
            }
            Modification::SetToughness { value, .. } => {
                toughness = Some(resolve_value_with_context(
                    value,
                    ctx,
                    effect.source,
                    effect.controller,
                ));
            }
            Modification::SetPowerToughness {
                power: p,
                toughness: t,
                ..
            } => {
                power = Some(resolve_value_with_context(
                    p,
                    ctx,
                    effect.source,
                    effect.controller,
                ));
                toughness = Some(resolve_value_with_context(
                    t,
                    ctx,
                    effect.source,
                    effect.controller,
                ));
            }
            Modification::ModifyPower(delta) => {
                if let Some(ref mut p) = power {
                    *p += delta;
                }
            }
            Modification::ModifyToughness(delta) => {
                if let Some(ref mut t) = toughness {
                    *t += delta;
                }
            }
            Modification::ModifyPowerToughness {
                power: dp,
                toughness: dt,
            } => {
                if let Some(ref mut p) = power {
                    *p += dp;
                }
                if let Some(ref mut t) = toughness {
                    *t += dt;
                }
            }
            Modification::ModifyPowerToughnessValue {
                power: power_value,
                toughness: toughness_value,
            } => {
                let dp =
                    resolve_value_with_context(power_value, ctx, effect.source, effect.controller);
                let dt = resolve_value_with_context(
                    toughness_value,
                    ctx,
                    effect.source,
                    effect.controller,
                );
                if let Some(ref mut p) = power {
                    *p += dp;
                }
                if let Some(ref mut t) = toughness {
                    *t += dt;
                }
            }
            Modification::ModifyPowerToughnessByColorCount {
                power_multiplier,
                toughness_multiplier,
            } => {
                let color_count = chars.colors.count() as i32;
                if let Some(ref mut p) = power {
                    *p += power_multiplier * color_count;
                }
                if let Some(ref mut t) = toughness {
                    *t += toughness_multiplier * color_count;
                }
            }
            Modification::SwitchPowerToughness => {
                std::mem::swap(&mut power, &mut toughness);
            }
            Modification::CopyOf { .. }
            | Modification::ChangeController(_)
            | Modification::ChangeText { .. }
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
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana
            | Modification::CantBeBlocked
            | Modification::CantAttack
            | Modification::CantBlock
            | Modification::DoesntUntap => {}
        }

        chars.power = power;
        chars.toughness = toughness;
        calc_guard.update(chars);
    }

    // If counters still haven't been applied (no 7c or 7d effects, or all 7c effects
    // had earlier timestamps), apply them now at the end of 7c
    if !counters_applied {
        apply_counter_modifications(object, &mut power, &mut toughness);
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
) {
    let (power_delta, toughness_delta) = object.pt_counter_deltas();
    if let Some(p) = power {
        *p += power_delta;
    }
    if let Some(t) = toughness {
        *t += toughness_delta;
    }
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

    effect_target_applies_to_direct(effect, object, chars, ctx.objects, ctx.game)
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
                filter_matches_with_characteristics(
                    filter,
                    obj,
                    &chars,
                    ctx.game,
                    filter_ctx.you.unwrap_or(obj.owner),
                    filter_ctx.source.unwrap_or(ctx.current_object),
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

pub(super) fn continuous_single_player(
    value: &Value,
    ctx: &CalculationContext<'_>,
    player_filter: &PlayerFilter,
    controller: PlayerId,
    source: ObjectId,
) -> PlayerId {
    let players = continuous_value_players(ctx, player_filter, controller, source);
    match players.as_slice() {
        [player] => *player,
        [] => panic!(
            "unsupported continuous-effect value {value:?}: player filter {player_filter:?} has no state-resolvable player"
        ),
        _ => panic!(
            "unsupported continuous-effect value {value:?}: player filter {player_filter:?} is ambiguous"
        ),
    }
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

    let mut baseline = HashMap::with_capacity(objects.len());
    for &id in objects.keys() {
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
            | Modification::RemoveAbilityGeneric { .. }
            | Modification::RemoveAllAbilities
            | Modification::RemoveAllAbilitiesExceptMana
    )
}

pub(super) fn layer_needs_source_activity_tracking<'a>(
    layer_effects: &[&ContinuousEffect],
    all_effects: impl IntoIterator<Item = &'a ContinuousEffect>,
    layer: Layer,
) -> bool {
    layer_effects
        .iter()
        .any(|effect| effect.originating_static_ability.is_some())
        && all_effects.into_iter().any(|effect| {
            effect.modification.layer() <= layer
                && effect_can_change_static_ability_presence(effect)
        })
}

pub(super) fn tracked_source_ids_for_layer(
    layer_effects: &[&ContinuousEffect],
) -> HashSet<ObjectId> {
    layer_effects
        .iter()
        .filter_map(|effect| {
            effect
                .originating_static_ability
                .as_ref()
                .map(|_| effect.source)
        })
        .collect()
}

pub(super) fn effect_source_is_active(
    effect: &ContinuousEffect,
    source_state: &HashMap<ObjectId, CalculatedCharacteristics>,
) -> bool {
    let Some(originating_static_ability) = &effect.originating_static_ability else {
        return true;
    };

    source_state.get(&effect.source).is_some_and(|chars| {
        chars
            .static_abilities
            .iter()
            .any(|ability| ability.instance_id() == originating_static_ability.instance_id())
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
        crate::dependency::apply_modification_to_chars_for_dependency(
            &effect.modification,
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
        crate::dependency::apply_modification_to_chars_for_dependency(
            &effect.modification,
            &mut updated,
            object,
            game,
        );
        source_state.insert(id, updated);
    }
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
    use crate::static_abilities::StaticAbilityId;

    if object.counters.get(&counter_type).copied().unwrap_or(0) == 0 {
        return;
    }

    if counter_type == CounterType::Decayed {
        if !chars
            .static_abilities
            .iter()
            .any(|a| a.id() == StaticAbilityId::CantBlock)
        {
            push_static_ability_once(chars, StaticAbility::cant_block());
        }
        chars.abilities.push(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            crate::resolution::ResolutionProgram::from_effects(vec![crate::effect::Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    crate::triggers::Trigger::end_of_combat(),
                    vec![crate::effect::Effect::sacrifice_source()],
                    true,
                    Vec::new(),
                    crate::target::PlayerFilter::You,
                ),
            )]),
        ));
        return;
    }

    // Check if this counter grants an ability
    if let Some(ability_id) = counter_type.granted_ability() {
        // Check if we already have this ability (avoid duplicates)
        let already_has = chars.static_abilities.iter().any(|a| a.id() == ability_id);
        if already_has {
            return;
        }

        // Add the appropriate static ability based on the counter type
        let ability: Option<StaticAbility> = match ability_id {
            StaticAbilityId::Deathtouch => Some(StaticAbility::deathtouch()),
            StaticAbilityId::Flying => Some(StaticAbility::flying()),
            StaticAbilityId::FirstStrike => Some(StaticAbility::first_strike()),
            StaticAbilityId::DoubleStrike => Some(StaticAbility::double_strike()),
            StaticAbilityId::Hexproof => Some(StaticAbility::hexproof()),
            StaticAbilityId::Indestructible => Some(StaticAbility::indestructible()),
            StaticAbilityId::Lifelink => Some(StaticAbility::lifelink()),
            StaticAbilityId::Menace => Some(StaticAbility::menace()),
            StaticAbilityId::Reach => Some(StaticAbility::reach()),
            StaticAbilityId::Trample => Some(StaticAbility::trample()),
            StaticAbilityId::Vigilance => Some(StaticAbility::vigilance()),
            StaticAbilityId::Haste => Some(StaticAbility::haste()),
            _ => None,
        };

        if let Some(sa) = ability {
            push_static_ability_once(chars, sa);
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
) {
    for grant in &object.temporary_static_ability_grants {
        let Some(ability) = grant.materialize() else {
            continue;
        };
        if chars
            .static_abilities
            .iter()
            .any(|existing| existing.id() == ability.id())
        {
            continue;
        }
        push_static_ability_once(chars, ability);
    }
}

/// Get P/T override from level abilities if applicable.
pub(super) fn get_level_ability_pt(object: &Object) -> Option<(i32, i32)> {
    let level_count = object
        .counters
        .get(&CounterType::Level)
        .copied()
        .unwrap_or(0);

    for ability in object.abilities.iter() {
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

/// Get abilities granted by the current level tier.
pub(super) fn get_level_granted_abilities(object: &Object) -> Vec<StaticAbility> {
    let level_count = object
        .counters
        .get(&CounterType::Level)
        .copied()
        .unwrap_or(0);

    for ability in object.abilities.iter() {
        if let AbilityKind::Static(s) = &ability.kind
            && let Some(levels) = s.level_abilities()
        {
            // Find the matching tier
            for tier in levels.iter().rev() {
                if tier.applies_at_level(level_count) {
                    // Abilities are now stored as the new type directly
                    return tier.abilities.clone();
                }
            }
        }
    }
    Vec::new()
}

/// Apply the active level tier, including object abilities stored inside a
/// source-filtered static carrier because `LevelAbility` itself stores only
/// static abilities.
pub(super) fn apply_level_granted_abilities(
    object: &Object,
    chars: &mut CalculatedCharacteristics,
) {
    for ability in get_level_granted_abilities(object) {
        for granted in ability.source_granted_inline_abilities() {
            match &granted.kind {
                AbilityKind::Static(static_ability) => {
                    push_static_ability_once(chars, static_ability.clone());
                }
                _ if !chars.abilities.contains(granted) => chars.abilities.push(granted.clone()),
                _ => {}
            }
        }
        push_static_ability_once(chars, ability);
    }
}
