// ============================================================================
// Stack Resolution
// ============================================================================

/// Resolve the top entry on the stack.
///
/// This function:
/// 1. Pops the top entry from the stack
/// 2. Validates targets
/// 3. Executes effects
/// 4. Moves spell to graveyard (if spell, not ability)
///
/// Note: May effects will be auto-declined. Use `resolve_stack_entry_with` to
/// provide a decision maker for interactive May choices.
pub fn resolve_stack_entry(game: &mut GameState) -> Result<(), GameLoopError> {
    let mut auto_dm = crate::decision::AutoPassDecisionMaker;
    resolve_stack_entry_full(game, &mut auto_dm, None)
}

/// Resolve the top entry on the stack with both a decision maker and trigger queue.
///
/// Use this for ETB replacement effects that need player decisions (like Mox Diamond).
fn resolve_stack_entry_with_dm_and_triggers(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    resolve_stack_entry_full(game, decision_maker, Some(trigger_queue))
}

/// Resolve the top entry on the stack with an optional decision maker.
///
/// If a decision maker is provided, May effects will prompt the player.
/// Otherwise, May effects are auto-declined.
pub fn resolve_stack_entry_with(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    resolve_stack_entry_full(game, decision_maker, None)
}

/// Resolve the top entry on the stack with optional decision maker and trigger queue.
///
/// If a trigger_queue is provided, saga lore counters are processed immediately.
/// Otherwise, saga processing must be handled by the caller.
fn resolve_stack_entry_full(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    mut trigger_queue: Option<&mut TriggerQueue>,
) -> Result<(), GameLoopError> {
    let entry = game
        .pop_from_stack()
        .ok_or_else(|| GameLoopError::InvalidState("Stack is empty".to_string()))?;

    // Get the object for this stack entry
    let obj = game.object(entry.object_id).cloned();

    // Create execution context
    // Resolution effects use EventCause::from_effect to distinguish from cost effects
    let mut ctx = ExecutionContext::new(entry.object_id, entry.controller, decision_maker)
        .with_optional_costs_paid(entry.optional_costs_paid.clone())
        .with_cause(EventCause::from_effect(entry.object_id, entry.controller));
    if let Some(x) = entry.x_value {
        ctx = ctx.with_x(x);
    }
    if let Some(defending) = entry.defending_player {
        ctx = ctx.with_defending_player(defending);
    }
    if let Some(triggering_event) = entry.triggering_event.clone() {
        ctx = ctx.with_triggering_event(triggering_event);
    }
    if let Some(source_snapshot) = entry.source_snapshot.clone() {
        ctx = ctx.with_source_snapshot(source_snapshot);
    }
    if !entry.tagged_objects.is_empty() {
        ctx = ctx.with_tagged_objects(entry.tagged_objects.clone());
    }
    // Pass pre-chosen modes from casting (per MTG rule 601.2b)
    if let Some(ref modes) = entry.chosen_modes {
        ctx = ctx.with_chosen_modes(Some(modes.clone()));
    }
    apply_keyword_payment_tags_for_resolution(game, &entry, &mut ctx);

    // Convert targets and validate them
    // Per MTG Rule 608.2b, if ALL targets are now illegal, the spell/ability fizzles
    let (valid_targets, all_targets_invalid) = validate_stack_entry_targets(game, &entry);

    // If the spell/ability had targets and ALL are now invalid, it fizzles
    if !entry.targets.is_empty() && all_targets_invalid {
        // Spell fizzles - move to graveyard without executing effects
        if let Some(obj) = &obj
            && obj.zone == Zone::Stack
            && !entry.is_ability
        {
            // Move spell to owner's graveyard (via replacement effects)
            let _ = crate::effects::zones::apply_zone_change(
                game,
                entry.object_id,
                Zone::Stack,
                Zone::Graveyard,
                &mut *decision_maker,
            );
        }
        return Ok(());
    }

    // Check intervening-if condition at resolution time
    // If the condition is false, the ability does nothing (but doesn't fizzle)
    if let Some(ref condition) = entry.intervening_if
        && let Some(ref triggering_event) = entry.triggering_event
        && !crate::triggers::verify_intervening_if(
            game,
            condition,
            entry.controller,
            triggering_event,
            entry.object_id,
            None,
        )
    {
        // Condition no longer true - ability resolves but does nothing
        return Ok(());
    }
    // If no triggering event is set (shouldn't happen for triggered abilities),
    // we allow the ability to proceed rather than creating a fake event

    ctx = ctx.with_targets(valid_targets);

    // Snapshot target objects for "last known information" before effects execute
    // This allows effects to access power/controller of targets even after they're exiled
    ctx.snapshot_targets(game);

    // Get effects to execute
    // For abilities with stored effects (like triggered abilities), use those directly
    // even if the source object no longer exists (e.g., undying triggers from dead creatures)
    let effects = if let Some(ref ability_effects) = entry.ability_effects {
        ability_effects.clone()
    } else if let Some(obj) = &obj {
        get_effects_for_stack_entry(game, &entry, obj)
    } else {
        Vec::new()
    };

    // ETB replacement is resolved when the spell actually moves to the battlefield.
    let etb_replacement_result: Option<(bool, bool, Zone)> = None;

    let mut all_events = Vec::new();
    for effect in &effects {
        if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
            all_events.extend(outcome.events);
        }
    }
    // Process events from effect outcomes for triggers
    if let Some(ref mut tq) = trigger_queue {
        for event in all_events {
            let event = game.ensure_trigger_event_provenance(event);
            let triggers = check_triggers(game, &event);
            for t in triggers {
                tq.add(t);
            }
        }
    }

    // Process pending primitive trigger events emitted by effects and zone changes.
    if let Some(ref mut tq) = trigger_queue {
        drain_pending_trigger_events(game, tq);
    }

    // If this was a saga's final chapter ability, mark it as resolved
    // (the saga will be sacrificed as a state-based action)
    if let Some(saga_id) = entry.saga_final_chapter_source {
        mark_saga_final_chapter_resolved(game, saga_id);
    }

    // Move spell to appropriate zone after resolution
    if let Some(obj) = &obj {
        if obj.zone == Zone::Stack && obj.is_permanent() {
            // Handle ETB replacement: if player didn't satisfy the replacement, redirect
            if let Some((enters, enters_tapped, redirect_zone)) = etb_replacement_result {
                if !enters {
                    // Permanent goes to redirect zone instead of battlefield
                    let _ = crate::effects::zones::apply_zone_change(
                        game,
                        entry.object_id,
                        Zone::Stack,
                        redirect_zone,
                        &mut *decision_maker,
                    );
                    return Ok(());
                }

                // Copy optional_costs_paid to the permanent before moving to battlefield
                if let Some(perm) = game.object_mut(entry.object_id) {
                    perm.optional_costs_paid = entry.optional_costs_paid.clone();
                }

                // Track creature ETBs for trap conditions
                if obj.is_creature() {
                    *game
                        .creatures_entered_this_turn
                        .entry(entry.controller)
                        .or_insert(0) += 1;
                }

                // Interactive replacement was already processed above - skip second ETB processing
                // and move directly to battlefield (avoids double-processing)
                let new_id = game.move_object(entry.object_id, Zone::Battlefield);
                if let Some(id) = new_id {
                    // Apply enters tapped if needed (e.g., shock land not paying life)
                    if enters_tapped {
                        game.tap(id);
                    }

                    if let Some(ref mut tq) = trigger_queue {
                        // Drain pending ZoneChangeEvent emitted by move_object.
                        drain_pending_trigger_events(game, tq);
                    }

                    // Check for ETB triggers
                    if let Some(ref mut tq) = trigger_queue {
                        let etb_event_provenance = game
                            .provenance_graph
                            .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                        let etb_event = if enters_tapped {
                            TriggerEvent::new_with_provenance(
                                EnterBattlefieldEvent::tapped(id, Zone::Stack),
                                etb_event_provenance,
                            )
                        } else {
                            TriggerEvent::new_with_provenance(
                                EnterBattlefieldEvent::new(id, Zone::Stack),
                                etb_event_provenance,
                            )
                        };
                        let etb_event = game.ensure_trigger_event_provenance(etb_event);
                        let etb_triggers = check_triggers(game, &etb_event);
                        for trigger in etb_triggers {
                            tq.add(trigger);
                        }
                    }
                }
                return Ok(());
            }

            // No interactive replacement was handled above - use normal ETB processing
            // Copy optional_costs_paid to the permanent before moving to battlefield
            // (so ETB triggers can access kick count, etc.)
            if let Some(perm) = game.object_mut(entry.object_id) {
                perm.optional_costs_paid = entry.optional_costs_paid.clone();
                // Preserve Convoke/Improvise contributors for later triggered ability resolution.
                perm.keyword_payment_contributions_to_cast =
                    entry.keyword_payment_contributions.clone();
            }

            // Track creature ETBs for trap conditions (before the object moves zones)
            if obj.is_creature() {
                *game
                    .creatures_entered_this_turn
                    .entry(entry.controller)
                    .or_insert(0) += 1;
            }

            // It's a permanent spell, move to battlefield with ETB processing
            // This handles replacement effects like "enters tapped" or "enters with counters"
            let etb_result = game.move_object_with_etb_processing_with_dm(
                entry.object_id,
                Zone::Battlefield,
                decision_maker,
            );

            // Note: Use the new ID from ETB result since zone change creates a new object
            if let Some(result) = etb_result {
                // If this is an Aura, attach it to its target as it enters
                if obj.subtypes.contains(&Subtype::Aura)
                    && let Some(Target::Object(target_id)) = entry
                        .targets
                        .iter()
                        .find(|t| matches!(t, Target::Object(_)))
                {
                    if let Some(aura) = game.object_mut(result.new_id) {
                        aura.attached_to = Some(*target_id);
                    }
                    if let Some(target) = game.object_mut(*target_id)
                        && !target.attachments.contains(&result.new_id)
                    {
                        target.attachments.push(result.new_id);
                    }
                    game.continuous_effects.record_attachment(result.new_id);
                }

                // Check for ETB triggers and add them to the trigger queue
                if let Some(ref mut tq) = trigger_queue {
                    // Drain pending ZoneChangeEvent emitted by ETB move processing.
                    drain_pending_trigger_events(game, tq);

                    let etb_event_provenance = game
                        .provenance_graph
                        .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                    let etb_event = if result.enters_tapped {
                        TriggerEvent::new_with_provenance(
                            EnterBattlefieldEvent::tapped(result.new_id, Zone::Stack),
                            etb_event_provenance,
                        )
                    } else {
                        TriggerEvent::new_with_provenance(
                            EnterBattlefieldEvent::new(result.new_id, Zone::Stack),
                            etb_event_provenance,
                        )
                    };
                    let etb_event = game.ensure_trigger_event_provenance(etb_event);
                    let etb_triggers = check_triggers(game, &etb_event);
                    for trigger in etb_triggers {
                        tq.add(trigger);
                    }
                }

                // If it's a saga, add its initial lore counter and check for chapter triggers
                if obj.subtypes.contains(&Subtype::Saga) {
                    if let Some(tq) = trigger_queue {
                        // Process immediately with the provided trigger queue
                        add_lore_counter_and_check_chapters(game, result.new_id, tq);
                    } else {
                        // Create a temporary trigger queue for immediate processing
                        let mut temp_queue = TriggerQueue::new();
                        add_lore_counter_and_check_chapters(game, result.new_id, &mut temp_queue);
                        // Note: triggers will be put on stack in advance_priority
                    }
                }
            }
        } else if obj.zone == Zone::Stack {
            // It's an instant/sorcery
            let has_rebound = matches!(entry.casting_method, CastingMethod::Normal)
                && obj.abilities.iter().any(|ability| {
                    ability.functions_in(&Zone::Stack)
                        && matches!(
                            &ability.kind,
                            AbilityKind::Static(static_ability)
                                if static_ability.id()
                                    == crate::static_abilities::StaticAbilityId::Rebound
                        )
                });

            // Check if cast with flashback/escape/jump-start/granted escape (exiles after resolution)
            let should_exile = match &entry.casting_method {
                CastingMethod::Normal => false,
                CastingMethod::Alternative(idx) => obj
                    .alternative_casts
                    .get(*idx)
                    .map(|m| m.exiles_after_resolution())
                    .unwrap_or(false),
                CastingMethod::GrantedEscape { .. } => true, // Granted escape always exiles
                CastingMethod::GrantedFlashback => true,     // Granted flashback always exiles
                CastingMethod::PlayFrom {
                    use_alternative: Some(idx),
                    zone,
                    ..
                } => {
                    // Check if the alternative cost used exiles after resolution
                    crate::decision::resolve_play_from_alternative_method(
                        game,
                        entry.controller,
                        obj,
                        *zone,
                        *idx,
                    )
                    .map(|m| m.exiles_after_resolution())
                    .unwrap_or(false)
                }
                CastingMethod::PlayFrom {
                    use_alternative: None,
                    ..
                } => {
                    // Normal cost via Yawgmoth's Will - replacement effect handles exile
                    false
                }
            };

            if has_rebound {
                if let crate::event_processor::EventOutcome::Proceed(result) =
                    crate::effects::zones::apply_zone_change(
                        game,
                        entry.object_id,
                        Zone::Stack,
                        Zone::Exile,
                        &mut *decision_maker,
                    )
                    && result.final_zone == Zone::Exile
                    && let Some(exiled_id) = result.new_object_id
                {
                    game.delayed_triggers.push(crate::triggers::DelayedTrigger {
                        trigger: crate::triggers::Trigger::beginning_of_upkeep(
                            crate::target::PlayerFilter::Specific(entry.controller),
                        ),
                        effects: vec![Effect::may_single(Effect::new(
                            crate::effects::CastSourceEffect::new()
                                .without_paying_mana_cost()
                                .require_exile(),
                        ))],
                        one_shot: true,
                        x_value: entry.x_value,
                        not_before_turn: None,
                        expires_at_turn: None,
                        target_objects: vec![exiled_id],
                        ability_source: None,
                        controller: entry.controller,
                        choices: vec![],
                        tagged_objects: std::collections::HashMap::new(),
                    });
                }
            } else if should_exile {
                let _ = crate::effects::zones::apply_zone_change(
                    game,
                    entry.object_id,
                    Zone::Stack,
                    Zone::Exile,
                    &mut *decision_maker,
                );
            } else {
                // Process zone change through replacement effects
                // (e.g., Yawgmoth's Will exiles cards going to graveyard)
                let _ = crate::effects::zones::apply_zone_change(
                    game,
                    entry.object_id,
                    Zone::Stack,
                    Zone::Graveyard,
                    &mut *decision_maker,
                );
            }
        }
        // Abilities just disappear from the stack
    }

    Ok(())
}

/// Get effects for a stack entry.
fn get_effects_for_stack_entry(
    _game: &GameState,
    entry: &StackEntry,
    obj: &crate::object::Object,
) -> Vec<Effect> {
    // If this is an ability with stored effects, use those directly
    if let Some(ref effects) = entry.ability_effects {
        return effects.clone();
    }

    // For spells, check the spell_effect field (instants/sorceries)
    if let Some(ref effects) = obj.spell_effect {
        return effects.clone();
    }

    // Permanent spells (creatures, artifacts, enchantments, etc.) don't have effects
    // that execute on resolution - they just enter the battlefield.
    // Don't fall back to looking at their abilities.
    if obj.is_permanent() {
        return Vec::new();
    }

    // For ability stack entries without stored effects, look in the object's abilities
    // (This is a fallback for edge cases, but normally ability_effects should be set)
    for ability in &obj.abilities {
        match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                return triggered.effects.clone();
            }
            AbilityKind::Activated(activated) => {
                return activated.effects.clone();
            }
            _ => {}
        }
    }

    Vec::new()
}
