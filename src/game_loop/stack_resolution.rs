use super::*;
use crate::executor::rebase_target_scope;
use crate::triggers::Trigger;

pub(super) fn active_target_assignments_for_effect(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    assignments: &[crate::game_state::TargetAssignment],
    cursor: &mut usize,
) -> Vec<crate::game_state::TargetAssignment> {
    let count =
        count_target_selection_slots_for_effect(effect, chosen_modes, consumed_modal_selection);
    let start = *cursor;
    let end = start.saturating_add(count).min(assignments.len());
    *cursor = end;
    assignments[start..end].to_vec()
}

fn representative_segment_targets(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &Effect,
    effect_target_assignments: Vec<crate::game_state::TargetAssignment>,
) -> Result<Option<Vec<crate::executor::ResolvedTarget>>, GameLoopError> {
    ctx.with_temp_target_assignments(effect_target_assignments, |ctx| {
        let Some(spec) = effect.0.get_target_spec() else {
            return Ok(None);
        };
        let object_id =
            match crate::effects::helpers::resolve_single_object_for_effect(game, ctx, spec) {
                Ok(id) => id,
                Err(crate::executor::ExecutionError::InvalidTarget) => return Ok(None),
                Err(err) => return Err(GameLoopError::ResolutionFailed(err.to_string())),
            };
        Ok(Some(vec![crate::executor::ResolvedTarget::Object(
            object_id,
        )]))
    })
}

fn apply_self_replacement_tag_prelude(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<(), GameLoopError> {
    for effect in effects {
        let is_pure_tag_effect = effect
            .downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            .is_some()
            || effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some()
            || effect
                .downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()
                .is_some()
            || effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .is_some_and(|tagged| {
                    tagged
                        .effect
                        .downcast_ref::<crate::effects::SequenceEffect>()
                        .is_some_and(|sequence| sequence.effects.is_empty())
                });
        if !is_pure_tag_effect {
            break;
        }
        crate::executor::execute_effect(game, effect, ctx)
            .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()))?;
    }
    Ok(())
}

fn evaluate_self_replacement_branch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    branch: &crate::resolution::SelfReplacementBranch,
    segment_effects: &[Effect],
    representative_effect: Option<&Effect>,
    representative_assignments: Vec<crate::game_state::TargetAssignment>,
) -> Result<bool, GameLoopError> {
    let original_tagged_objects = ctx.tagged_objects.clone();
    apply_self_replacement_tag_prelude(game, ctx, segment_effects)?;
    let Some(effect) = representative_effect else {
        let result =
            crate::condition_eval::evaluate_condition_resolution(game, &branch.condition, ctx)
                .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()));
        ctx.tagged_objects = original_tagged_objects;
        return result;
    };

    let representative_targets =
        representative_segment_targets(game, ctx, effect, representative_assignments.clone())?;
    let result = ctx
        .with_temp_target_assignments(representative_assignments, |ctx| {
            if let Some(targets) = representative_targets {
                ctx.with_temp_targets(targets, |ctx| {
                    crate::condition_eval::evaluate_condition_resolution(
                        game,
                        &branch.condition,
                        ctx,
                    )
                })
            } else {
                crate::condition_eval::evaluate_condition_resolution(game, &branch.condition, ctx)
            }
        })
        .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()));
    ctx.tagged_objects = original_tagged_objects;
    result
}

pub(crate) fn execute_resolution_program(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    _controller: PlayerId,
    _source_id: ObjectId,
    program: &crate::resolution::ResolutionProgram,
    chosen_modes: Option<&[usize]>,
    valid_target_assignments: &[crate::game_state::TargetAssignment],
) -> Result<Vec<crate::triggers::TriggerEvent>, GameLoopError> {
    let mut all_events = Vec::new();
    let mut consumed_modal_selection = false;
    let mut assignment_cursor = 0usize;
    for segment in &program.segments {
        let (selected_effects, selected_self_replacement) = if segment.self_replacements.is_empty()
        {
            (segment.default_effects.clone(), false)
        } else {
            let representative_effect = segment
                .default_effects
                .iter()
                .find(|effect| effect.0.get_target_spec().is_some())
                .or_else(|| {
                    segment
                        .self_replacements
                        .iter()
                        .flat_map(|branch| branch.replacement_effects.iter())
                        .find(|effect| effect.0.get_target_spec().is_some())
                });
            let representative_assignments = representative_effect
                .map(|effect| {
                    let mut temp_cursor = assignment_cursor;
                    let mut temp_consumed_modal_selection = consumed_modal_selection;
                    active_target_assignments_for_effect(
                        effect,
                        chosen_modes,
                        &mut temp_consumed_modal_selection,
                        valid_target_assignments,
                        &mut temp_cursor,
                    )
                })
                .unwrap_or_default();
            let mut applicable = Vec::new();
            for branch in &segment.self_replacements {
                if evaluate_self_replacement_branch(
                    game,
                    ctx,
                    branch,
                    &segment.default_effects,
                    representative_effect,
                    representative_assignments.clone(),
                )? {
                    applicable.push(branch);
                }
            }

            match applicable.len() {
                0 => (segment.default_effects.clone(), false),
                1 => (applicable[0].replacement_effects.clone(), true),
                _ => {
                    return Err(GameLoopError::ResolutionFailed(
                        "multiple self-replacement branches applied during resolution".to_string(),
                    ));
                }
            }
        };

        if selected_self_replacement {
            apply_self_replacement_tag_prelude(game, ctx, &segment.default_effects)?;
        }

        let mut active_scope: Option<(
            Vec<crate::executor::ResolvedTarget>,
            Vec<crate::game_state::TargetAssignment>,
        )> = None;
        for effect in &selected_effects {
            let effect_target_assignments = active_target_assignments_for_effect(
                effect,
                chosen_modes,
                &mut consumed_modal_selection,
                valid_target_assignments,
                &mut assignment_cursor,
            );
            if !effect_target_assignments.is_empty() {
                let (effect_targets, effect_target_assignments) =
                    rebase_target_scope(&ctx.targets, &effect_target_assignments);
                active_scope = Some((effect_targets, effect_target_assignments));
            }
            let outcome = if let Some((effect_targets, effect_target_assignments)) = &active_scope {
                ctx.with_temp_targets(effect_targets.clone(), |ctx| {
                    ctx.with_temp_target_assignments(effect_target_assignments.clone(), |ctx| {
                        execute_effect(game, effect, ctx)
                    })
                })
            } else {
                execute_effect(game, effect, ctx)
            };
            if let Ok(outcome) = outcome {
                all_events.extend(outcome.events);
            }
        }
    }
    Ok(all_events)
}

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
pub(super) fn resolve_stack_entry_with_dm_and_triggers(
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
pub(super) fn resolve_stack_entry_full(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    mut trigger_queue: Option<&mut TriggerQueue>,
) -> Result<(), GameLoopError> {
    game.refresh_continuous_state();
    let entry = game
        .pop_from_stack()
        .ok_or_else(|| GameLoopError::InvalidState("Stack is empty".to_string()))?;

    // Get the object for this stack entry
    let obj = game.object(entry.object_id).cloned();

    // Create execution context
    // Resolution effects use EventCause::from_effect to distinguish from cost effects
    let execution_source = if entry.is_ability {
        entry.source_snapshot
            .as_ref()
            .map(|snapshot| snapshot.object_id)
            .unwrap_or(entry.object_id)
    } else {
        entry.object_id
    };
    let mut ctx = ExecutionContext::new(execution_source, entry.controller, decision_maker)
        .with_optional_costs_paid(entry.optional_costs_paid.clone())
        .with_casting_method(entry.casting_method.clone())
        .with_cause(EventCause::from_effect(entry.object_id, entry.controller))
        .with_provenance(entry.provenance);
    if let Some(x) = entry.x_value {
        ctx = ctx.with_x(x);
    }
    if let Some(defending) = entry.defending_player {
        ctx = ctx.with_defending_player(defending);
    }
    if entry.chosen_player.is_some() {
        ctx = ctx.with_chosen_player(entry.chosen_player);
    }
    if let Some(triggering_event) = entry.triggering_event.clone() {
        ctx = ctx.with_triggering_event(triggering_event);
    }
    if let Some(source_snapshot) = entry.source_snapshot.clone() {
        ctx = ctx.with_source_snapshot(source_snapshot);
    }
    let mut tagged_objects = entry.tagged_objects.clone();
    let source_exiled = game
        .get_exiled_with_source_links(execution_source)
        .iter()
        .filter_map(|id| {
            game.object(*id).map(|obj| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )
            })
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        tagged_objects.insert(
            crate::tag::TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            source_exiled,
        );
    }
    if !tagged_objects.is_empty() {
        ctx = ctx.with_tagged_objects(tagged_objects);
    }
    // Pass pre-chosen modes from casting (per MTG rule 601.2b)
    if let Some(ref modes) = entry.chosen_modes {
        ctx = ctx.with_chosen_modes(Some(modes.clone()));
    }
    apply_keyword_payment_tags_for_resolution(game, &entry, &mut ctx);

    // Convert targets and validate them
    // Per MTG Rule 608.2b, if ALL targets are now illegal, the spell/ability fizzles
    let target_validation_view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    let (valid_targets, valid_target_assignments, all_targets_invalid) =
        validate_stack_entry_targets_with_view(game, &entry, &target_validation_view);

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
                crate::events::cause::EventCause::from_effect(entry.object_id, entry.controller),
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
            execution_source,
            None,
        )
    {
        // Condition no longer true - ability resolves but does nothing
        return Ok(());
    }
    // If no triggering event is set (shouldn't happen for triggered abilities),
    // we allow the ability to proceed rather than creating a fake event

    ctx = ctx
        .with_targets(valid_targets)
        .with_target_assignments(valid_target_assignments.clone());

    // Snapshot target objects for "last known information" before effects execute
    // This allows effects to access power/controller of targets even after they're exiled
    ctx.snapshot_targets(game);

    // Get effects to execute
    // For abilities with stored effects (like triggered abilities), use those directly
    // even if the source object no longer exists (e.g., undying triggers from dead creatures)
    let program = if let Some(ref ability_effects) = entry.ability_effects {
        ability_effects.clone()
    } else if let Some(obj) = &obj {
        get_effects_for_stack_entry(game, &entry, obj)
    } else {
        crate::resolution::ResolutionProgram::default()
    };

    // ETB replacement is resolved when the spell actually moves to the battlefield.
    let etb_replacement_result: Option<(bool, bool, Zone)> = None;

    let all_events = execute_resolution_program(
        game,
        &mut ctx,
        entry.controller,
        entry.object_id,
        &program,
        entry.chosen_modes.as_deref(),
        &valid_target_assignments,
    )?;
    // Process events from effect outcomes for triggers
    if let Some(ref mut tq) = trigger_queue {
        for event in all_events {
            queue_triggers_from_event(game, tq, event, false);
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
            let chosen_player = entry
                .chosen_player
                .or_else(|| game.chosen_player(entry.object_id));
            // Handle ETB replacement: if player didn't satisfy the replacement, redirect
            if let Some((enters, enters_tapped, redirect_zone)) = etb_replacement_result {
                if !enters {
                    // Permanent goes to redirect zone instead of battlefield
                    let _ = crate::effects::zones::apply_zone_change(
                        game,
                        entry.object_id,
                        Zone::Stack,
                        redirect_zone,
                        crate::events::cause::EventCause::from_effect(
                            entry.object_id,
                            entry.controller,
                        ),
                        &mut *decision_maker,
                    );
                    return Ok(());
                }

                // Copy optional_costs_paid to the permanent before moving to battlefield
                if let Some(perm) = game.object_mut(entry.object_id) {
                    perm.optional_costs_paid = entry.optional_costs_paid.clone();
                }

                // Interactive replacement was already processed above - skip second ETB processing
                // and move directly to battlefield (avoids double-processing)
                let new_id = game.move_object_by_effect(entry.object_id, Zone::Battlefield);
                if let Some(id) = new_id {
                    if let Some(chosen_player) = chosen_player {
                        game.set_chosen_player(id, chosen_player);
                    }
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

            // It's a permanent spell, move to battlefield with ETB processing
            // This handles replacement effects like "enters tapped" or "enters with counters"
            let etb_result = game.move_object_with_etb_processing_with_dm(
                entry.object_id,
                Zone::Battlefield,
                decision_maker,
            );

            // Note: Use the new ID from ETB result since zone change creates a new object
            if let Some(result) = etb_result {
                if let Some(chosen_player) = chosen_player {
                    game.set_chosen_player(result.new_id, chosen_player);
                }
                // If this is an Aura, attach it to its target as it enters
                if obj.subtypes.contains(&Subtype::Aura) {
                    let attached = entry.targets.iter().find_map(|target| match target {
                        Target::Object(id) => Some(crate::object::AttachmentTarget::Object(*id)),
                        Target::Player(id) => Some(crate::object::AttachmentTarget::Player(*id)),
                    });
                    if let Some(target) = attached
                        && game.attach_object_to_target(result.new_id, target)
                    {
                        game.continuous_effects.record_attachment(result.new_id);
                    }
                }

                let cast_with_dash = match &entry.casting_method {
                    CastingMethod::Alternative(idx) => matches!(
                        obj.alternative_casts.get(*idx),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Dash { .. })
                    ),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    } => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Dash { .. })
                    ),
                    _ => false,
                };
                let cast_with_warp = match &entry.casting_method {
                    CastingMethod::Alternative(idx) => matches!(
                        obj.alternative_casts.get(*idx),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Warp { .. })
                    ),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    } => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Warp { .. })
                    ),
                    _ => false,
                };
                let cast_with_suspend = match &entry.casting_method {
                    CastingMethod::Alternative(idx) => matches!(
                        obj.alternative_casts.get(*idx),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Suspend { .. })
                    ),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    } => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Suspend { .. })
                    ),
                    _ => false,
                };
                if cast_with_dash {
                    let dash_haste = crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(result.new_id),
                        crate::continuous::Modification::AddAbility(
                            crate::static_abilities::StaticAbility::haste(),
                        ),
                        crate::effect::Until::EndOfTurn,
                    )
                    .with_source_type(
                        crate::continuous::EffectSourceType::Resolution {
                            locked_targets: vec![result.new_id],
                        },
                    );
                    let _ = crate::executor::execute_effect(
                        game,
                        &crate::effect::Effect::new(dash_haste),
                        &mut crate::executor::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );

                    let return_to_hand = crate::effects::ScheduleDelayedTriggerEffect::new(
                        Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
                        vec![crate::effect::Effect::new(
                            crate::effects::ReturnToHandEffect::with_spec(
                                crate::target::ChooseSpec::SpecificObject(result.new_id),
                            ),
                        )],
                        true,
                        vec![result.new_id],
                        crate::target::PlayerFilter::Specific(entry.controller),
                    );
                    let _ = crate::executor::execute_effect(
                        game,
                        &crate::effect::Effect::new(return_to_hand),
                        &mut crate::executor::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
                }
                if cast_with_suspend && obj.has_card_type(crate::types::CardType::Creature) {
                    let suspend_haste = crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(result.new_id),
                        crate::continuous::Modification::AddAbility(
                            crate::static_abilities::StaticAbility::haste(),
                        ),
                        crate::effect::Until::YouStopControllingThis,
                    );
                    let _ = crate::executor::execute_effect(
                        game,
                        &crate::effect::Effect::new(suspend_haste),
                        &mut crate::executor::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
                }
                if cast_with_warp {
                    let exile_then_grant = crate::effects::ScheduleDelayedTriggerEffect::new(
                        Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
                        vec![crate::effect::Effect::new(
                            crate::effects::ExileThenGrantPlayEffect::new(
                                crate::target::ChooseSpec::SpecificObject(result.new_id),
                                crate::target::PlayerFilter::Specific(entry.controller),
                                crate::grant::GrantDuration::Forever,
                            ),
                        )],
                        true,
                        vec![result.new_id],
                        crate::target::PlayerFilter::Specific(entry.controller),
                    );
                    let _ = crate::executor::execute_effect(
                        game,
                        &crate::effect::Effect::new(exile_then_grant),
                        &mut crate::executor::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
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
                CastingMethod::FaceDown => false,
                CastingMethod::SplitOtherHalf | CastingMethod::Fuse => false,
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
                        crate::events::cause::EventCause::from_effect(
                            entry.object_id,
                            entry.controller,
                        ),
                        &mut *decision_maker,
                    )
                    && result.final_zone == Zone::Exile
                    && let Some(exiled_id) = result.new_object_id
                {
                    game.delayed_triggers.push(crate::triggers::DelayedTrigger {
                        trigger: crate::triggers::Trigger::beginning_of_upkeep(
                            crate::target::PlayerFilter::Specific(entry.controller),
                        ),
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::may_single(Effect::new(
                                crate::effects::CastSourceEffect::new()
                                    .without_paying_mana_cost()
                                    .require_exile(),
                            )),
                        ]),
                        one_shot: true,
                        x_value: entry.x_value,
                        not_before_turn: None,
                        expires_at_turn: None,
                        target_objects: vec![exiled_id],
                        ability_source: None,
                        ability_source_stable_id: None,
                        ability_source_name: None,
                        ability_source_snapshot: None,
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
                    crate::events::cause::EventCause::from_effect(
                        entry.object_id,
                        entry.controller,
                    ),
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
                    crate::events::cause::EventCause::from_effect(
                        entry.object_id,
                        entry.controller,
                    ),
                    &mut *decision_maker,
                );
            }
        }
        // Abilities just disappear from the stack
    }

    Ok(())
}

/// Get effects for a stack entry.
pub(super) fn get_effects_for_stack_entry(
    _game: &GameState,
    entry: &StackEntry,
    obj: &crate::object::Object,
) -> crate::resolution::ResolutionProgram {
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
        return crate::resolution::ResolutionProgram::default();
    }

    crate::resolution::ResolutionProgram::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::types::CardType;

    #[derive(Default)]
    struct MatchingOptionDecisionMaker {
        needle: Option<String>,
    }

    impl MatchingOptionDecisionMaker {
        fn new(needle: &str) -> Self {
            Self {
                needle: Some(needle.to_ascii_lowercase()),
            }
        }
    }

    impl crate::decision::DecisionMaker for MatchingOptionDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(needle) = self.needle.take()
                && let Some(option) = ctx.options.iter().find(|option| {
                    option.legal && option.description.to_ascii_lowercase().contains(&needle)
                })
            {
                return vec![option.index];
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    #[derive(Default)]
    struct AnswerThenCaptureDecisionMaker {
        answers_remaining: usize,
        captured: Option<Vec<String>>,
    }

    impl AnswerThenCaptureDecisionMaker {
        fn new(answers_remaining: usize) -> Self {
            Self {
                answers_remaining,
                captured: None,
            }
        }
    }

    impl crate::decision::DecisionMaker for AnswerThenCaptureDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.captured.is_some()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if self.answers_remaining > 0 {
                self.answers_remaining -= 1;
                return ctx
                    .options
                    .iter()
                    .filter(|option| option.legal)
                    .map(|option| option.index)
                    .take(ctx.min)
                    .collect();
            }

            if self.captured.is_none() {
                self.captured = Some(
                    ctx.options
                        .iter()
                        .filter(|option| option.legal)
                        .map(|option| option.description.clone())
                        .collect(),
                );
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    fn parse_spell_definition(
        name: &str,
        card_types: Vec<CardType>,
        oracle_text: &str,
    ) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .parse_text(oracle_text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"))
    }

    fn parse_sorcery_definition(name: &str, oracle_text: &str) -> crate::cards::CardDefinition {
        parse_spell_definition(name, vec![CardType::Sorcery], oracle_text)
    }

    fn parse_instant_definition(name: &str, oracle_text: &str) -> crate::cards::CardDefinition {
        parse_spell_definition(name, vec![CardType::Instant], oracle_text)
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn register_spell_cast_this_turn_for_test(
        game: &mut GameState,
        spell_id: ObjectId,
        caster: PlayerId,
    ) {
        let event = TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell_id, caster, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    #[test]
    fn stack_resolution_tracks_creature_damage_for_backdraft_history() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        create_creature(&mut game, "Goblin 1", bob, 2, 2);
        create_creature(&mut game, "Goblin 2", bob, 2, 2);

        let blasphemous_act = parse_sorcery_definition(
            "Blasphemous Act",
            "This spell deals 13 damage to each creature.",
        );
        let backdraft = parse_sorcery_definition(
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        );

        let blasphemous_act_id =
            game.create_object_from_definition(&blasphemous_act, bob, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, blasphemous_act_id, bob);
        game.push_to_stack(StackEntry::new(blasphemous_act_id, bob));

        let mut trigger_queue = TriggerQueue::new();
        let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut auto_dm, &mut trigger_queue)
            .expect("Blasphemous Act should resolve");

        assert_eq!(
            game.turn_history
                .damage_dealt_by_spell_this_turn(&game.provenance_graph, blasphemous_act_id),
            26,
            "stack-resolved creature damage should be queryable from turn history"
        );

        let bob_life_before = game.player(bob).expect("bob exists").life;
        let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
        game.push_to_stack(StackEntry::new(backdraft_id, alice));

        let mut dm = MatchingOptionDecisionMaker::new("Bob");
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Backdraft should resolve");

        assert_eq!(
            game.player(bob).expect("bob exists").life,
            bob_life_before - 13,
            "Backdraft should use the creature damage dealt by Blasphemous Act"
        );
    }

    #[test]
    fn backdraft_prompts_for_spell_history_when_the_same_player_cast_both_spells() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        create_creature(&mut game, "Ornithopter 1", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 2", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 3", alice, 0, 2);

        let blasphemous_act = parse_sorcery_definition(
            "Blasphemous Act",
            "This spell deals 13 damage to each creature.",
        );
        let backdraft = parse_sorcery_definition(
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        );

        let blasphemous_act_id =
            game.create_object_from_definition(&blasphemous_act, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, blasphemous_act_id, alice);
        game.push_to_stack(StackEntry::new(blasphemous_act_id, alice));

        let mut trigger_queue = TriggerQueue::new();
        let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut auto_dm, &mut trigger_queue)
            .expect("Blasphemous Act should resolve");

        assert_eq!(
            game.turn_history
                .damage_dealt_by_spell_this_turn(&game.provenance_graph, blasphemous_act_id),
            39,
            "Blasphemous Act should record the 39 damage dealt to the three Ornithopters"
        );

        let history_names = game
            .turn_history
            .spell_cast_snapshot_history()
            .into_iter()
            .map(|snapshot| snapshot.name)
            .collect::<Vec<_>>();
        assert!(
            history_names.iter().any(|name| name == "Blasphemous Act"),
            "resolved Blasphemous Act should remain in spell-cast history, got {history_names:?}"
        );

        let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
        game.push_to_stack(StackEntry::new(backdraft_id, alice));

        let mut dm = AnswerThenCaptureDecisionMaker::new(1);
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Backdraft should resolve far enough to ask for a spell-history choice");

        let captured = dm.captured.expect(
            "Backdraft should prompt for one of Alice's sorcery spells after choosing Alice",
        );
        assert!(
            captured
                .iter()
                .any(|option| option.contains("Blasphemous Act")),
            "expected Blasphemous Act to be a legal Backdraft history option, got {captured:?}"
        );
        assert!(
            captured.iter().any(|option| option.contains("Backdraft")),
            "expected Backdraft to also remain a legal history option, got {captured:?}"
        );
    }

    #[test]
    fn run_priority_loop_finishes_backdraft_after_the_single_player_prompt() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;

        create_creature(&mut game, "Ornithopter 1", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 2", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 3", alice, 0, 2);

        let blasphemous_act = parse_sorcery_definition(
            "Blasphemous Act",
            "This spell deals 13 damage to each creature.",
        );
        let backdraft = parse_instant_definition(
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        );

        let blasphemous_act_id =
            game.create_object_from_definition(&blasphemous_act, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, blasphemous_act_id, alice);
        game.push_to_stack(StackEntry::new(blasphemous_act_id, alice));

        let mut trigger_queue = TriggerQueue::new();
        let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut auto_dm, &mut trigger_queue)
            .expect("Blasphemous Act should resolve");

        assert_eq!(
            game.turn_history
                .damage_dealt_by_spell_this_turn(&game.provenance_graph, blasphemous_act_id),
            39,
            "Blasphemous Act should record the 39 damage dealt to the three Ornithopters"
        );

        let alice_life_before = game.player(alice).expect("alice exists").life;
        let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
        game.push_to_stack(StackEntry::new(backdraft_id, alice));

        let mut dm = MatchingOptionDecisionMaker::new("Alice");
        let result =
            crate::game_loop::run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
                .expect("priority loop should resolve Backdraft after choosing Alice");

        assert!(
            matches!(result, crate::decision::GameProgress::Continue),
            "priority loop should finish cleanly after resolving Backdraft, got {result:?}"
        );
        assert_eq!(
            game.player(alice).expect("alice exists").life,
            alice_life_before - 19,
            "Backdraft should still deal half of Blasphemous Act's 39 damage after the player choice"
        );
    }
}
