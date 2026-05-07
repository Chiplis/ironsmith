use super::*;

const JS_SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;

// ============================================================================
// State-Based Actions Integration
// ============================================================================

/// Check and apply all state-based actions, generating trigger events.
///
/// This runs repeatedly until no more SBAs need to be applied.
/// Note: This version auto-keeps the first legend for legend rule violations.
/// Use `check_and_apply_sbas_with` to handle legend rule interactively.
pub fn check_and_apply_sbas(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    check_and_apply_sbas_with(game, trigger_queue, &mut dm)
}

/// Check and apply all state-based actions, generating trigger events.
///
/// This runs repeatedly until no more SBAs need to be applied.
/// Legend rule violations will prompt the decision maker for which legend to keep.
pub fn check_and_apply_sbas_with(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::decisions::make_decision;
    use crate::rules::state_based::{
        StateBasedActionContext, apply_legend_rule_choice,
        apply_state_based_actions_from_actions_with, check_state_based_actions_with_context,
        legend_rule_specs_from_actions,
    };

    // Refresh continuous state (static ability effects and "can't" effect tracking)
    // before checking SBAs. This ensures the layer system is up to date.
    game.refresh_continuous_state();

    loop {
        let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
        let all_effects = view.effects().to_vec();
        let context = StateBasedActionContext::from_trigger_queue(trigger_queue);
        let actions = check_state_based_actions_with_context(game, &view, &context);
        drop(view);
        if actions.is_empty() {
            game.clear_deathtouch_damage_since_sba();
            break;
        }

        // Handle legend rule decisions first
        let legend_specs = legend_rule_specs_from_actions(&actions);
        let had_legend_decisions = !legend_specs.is_empty();
        for (player, spec) in legend_specs {
            let keep_id: ObjectId = make_decision(game, decision_maker, player, None, spec);
            apply_legend_rule_choice(game, keep_id);
        }

        // Apply the SBAs (legend rule already handled above)
        // Use the decision maker version to allow interactive replacement effect choices
        let applied = if had_legend_decisions {
            let post_legend_view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
            let post_legend_effects = post_legend_view.effects().to_vec();
            let post_legend_context = StateBasedActionContext::from_trigger_queue(trigger_queue);
            let post_legend_actions = check_state_based_actions_with_context(
                game,
                &post_legend_view,
                &post_legend_context,
            );
            drop(post_legend_view);
            apply_state_based_actions_from_actions_with(
                game,
                post_legend_actions,
                &post_legend_effects,
                decision_maker,
            )
        } else {
            apply_state_based_actions_from_actions_with(game, actions, &all_effects, decision_maker)
        };
        game.clear_deathtouch_damage_since_sba();
        // SBA moves queue primitive ZoneChangeEvent via move_object; consume them now.
        drain_pending_trigger_events(game, trigger_queue);
        if !applied && !had_legend_decisions {
            break;
        }
    }

    let (state_triggers, active_state_triggers) = crate::triggers::check_state_triggers(game);
    game.effect_store.active_state_trigger_conditions = active_state_triggers;
    for trigger in state_triggers {
        trigger_queue.add(trigger);
    }

    Ok(())
}

/// Put triggered abilities from the queue onto the stack.
pub fn put_triggers_on_stack(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    put_triggers_on_stack_with_dm(game, trigger_queue, &mut dm)
}

/// Put triggered abilities from the queue onto the stack with target selection.
///
/// This handles the full flow of putting triggers on the stack:
/// 1. Group triggers by controller (APNAP order)
/// 2. For each trigger, handle target selection if needed
/// 3. Push the trigger onto the stack with targets
pub fn put_triggers_on_stack_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    game.refresh_continuous_state();

    // Triggered mana abilities resolve immediately and never use the stack.
    // Flush them first so only non-mana triggers remain to be stacked.
    resolve_triggered_mana_abilities_with_dm(game, trigger_queue, decision_maker);

    // Group triggers by controller, then let each controller order their own
    // simultaneous triggers before applying APNAP stack placement.
    let mut grouped: std::collections::HashMap<PlayerId, Vec<TriggeredAbilityEntry>> =
        std::collections::HashMap::new();

    for trigger in trigger_queue.take_all() {
        grouped.entry(trigger.controller).or_default().push(trigger);
    }

    let mut controller_order = players_in_apnap_order(game);
    for controller in grouped.keys().copied() {
        if !controller_order.contains(&controller) {
            controller_order.push(controller);
        }
    }

    for (controller_index, controller) in controller_order.iter().copied().enumerate() {
        let Some(triggers) = grouped.remove(&controller) else {
            continue;
        };
        let ordered = order_triggers_for_controller(game, decision_maker, triggers);
        if decision_maker.awaiting_choice() {
            for trigger in ordered {
                trigger_queue.add(trigger);
            }
            for remaining_controller in controller_order.iter().copied().skip(controller_index + 1)
            {
                if let Some(remaining_triggers) = grouped.remove(&remaining_controller) {
                    for trigger in remaining_triggers {
                        trigger_queue.add(trigger);
                    }
                }
            }
            return Ok(());
        }
        let ordered_for_stacking: Vec<_> = ordered.into_iter().rev().collect();
        for (index, trigger) in ordered_for_stacking.iter().enumerate() {
            if !can_stack_trigger_this_turn(game, &trigger) {
                continue;
            }
            if let Some(entry) =
                create_triggered_stack_entry_with_targets(game, trigger, decision_maker)
            {
                if decision_maker.awaiting_choice() {
                    for deferred in ordered_for_stacking.iter().skip(index).rev() {
                        trigger_queue.add(deferred.clone());
                    }
                    for remaining_controller in
                        controller_order.iter().copied().skip(controller_index + 1)
                    {
                        if let Some(remaining_triggers) = grouped.remove(&remaining_controller) {
                            for trigger in remaining_triggers {
                                trigger_queue.add(trigger);
                            }
                        }
                    }
                    return Ok(());
                }
                game.record_trigger_fired(trigger.source, trigger.trigger_identity);
                game.push_to_stack(entry);
            } else if decision_maker.awaiting_choice() {
                for deferred in ordered_for_stacking.iter().skip(index).rev() {
                    trigger_queue.add(deferred.clone());
                }
                for remaining_controller in
                    controller_order.iter().copied().skip(controller_index + 1)
                {
                    if let Some(remaining_triggers) = grouped.remove(&remaining_controller) {
                        for trigger in remaining_triggers {
                            trigger_queue.add(trigger);
                        }
                    }
                }
                return Ok(());
            }
        }
    }

    Ok(())
}

fn players_in_apnap_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_store.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_store
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_store.turn_order.len())
        .filter_map(|offset| {
            let player_id =
                game.turn_store.turn_order[(start + offset) % game.turn_store.turn_order.len()];
            game.player(player_id)
                .filter(|player| player.is_in_game())
                .map(|_| player_id)
        })
        .collect()
}

fn describe_trigger_for_ordering(trigger: &TriggeredAbilityEntry) -> String {
    let trigger_text = trigger.ability.trigger.display();
    let effect_text = crate::compiled_text::compile_effect_list(&trigger.ability.effects);
    let detail = if !effect_text.trim().is_empty() {
        effect_text
    } else if !trigger_text.trim().is_empty() {
        trigger_text
    } else {
        "Triggered ability".to_string()
    };

    format!("{}\n{}", trigger.source_name, detail)
}

fn uniquify_trigger_labels(labels: &mut [String]) {
    let mut totals = std::collections::HashMap::<String, usize>::new();
    for label in labels.iter() {
        *totals.entry(label.clone()).or_insert(0) += 1;
    }

    let mut seen = std::collections::HashMap::<String, usize>::new();
    for label in labels.iter_mut() {
        let total = totals.get(label).copied().unwrap_or(0);
        if total <= 1 {
            continue;
        }
        let ordinal = seen.entry(label.clone()).or_insert(0);
        *ordinal += 1;
        label.push_str(&format!("\nTrigger {}", *ordinal));
    }
}

fn order_triggers_for_controller(
    game: &GameState,
    decision_maker: &mut dyn DecisionMaker,
    triggers: Vec<TriggeredAbilityEntry>,
) -> Vec<TriggeredAbilityEntry> {
    if triggers.len() <= 1 {
        return triggers;
    }

    let controller = triggers[0].controller;
    let description = "Order triggered abilities. The leftmost item becomes the top of your stack.";
    let mut labels: Vec<String> = triggers.iter().map(describe_trigger_for_ordering).collect();
    uniquify_trigger_labels(&mut labels);

    let items: Vec<(ObjectId, String)> = labels
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            (
                ObjectId::from_raw(JS_SAFE_INTEGER_MAX.saturating_sub(index as u64)),
                label,
            )
        })
        .collect();
    let ctx = crate::decisions::context::enrich_display_hints(
        game,
        crate::decisions::context::DecisionContext::Order(
            crate::decisions::context::OrderContext::new(controller, None, description, items),
        ),
    )
    .into_order();
    let response = decision_maker.decide_order(game, &ctx);
    if decision_maker.awaiting_choice() {
        return triggers;
    }

    let mut remaining: Vec<(ObjectId, TriggeredAbilityEntry)> =
        ctx.items.iter().map(|(id, _)| *id).zip(triggers).collect();
    let mut ordered = Vec::with_capacity(remaining.len());

    for id in response {
        if let Some(position) = remaining.iter().position(|(item_id, _)| *item_id == id) {
            ordered.push(remaining.remove(position).1);
        }
    }

    ordered.extend(remaining.into_iter().map(|(_, trigger)| trigger));
    ordered
}

pub(super) fn is_triggered_mana_ability(game: &GameState, trigger: &TriggeredAbilityEntry) -> bool {
    if trigger.ability.choices.iter().any(ChooseSpec::is_target) {
        return false;
    }

    if !triggered_by_mana_event(trigger) {
        return false;
    }

    crate::ability::effects_could_add_mana(
        game,
        trigger.source,
        trigger.controller,
        &trigger.ability.effects,
    )
}

fn triggered_by_mana_event(trigger: &TriggeredAbilityEntry) -> bool {
    if let Some(activated_event) = trigger
        .triggering_event
        .downcast::<crate::events::spells::AbilityActivatedEvent>()
    {
        return activated_event.is_mana_ability;
    }

    trigger
        .triggering_event
        .downcast::<crate::events::ManaAddedEvent>()
        .is_some_and(|event| !event.mana.is_empty())
}

pub(super) fn resolve_triggered_stack_entry_immediately(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
    entry: StackEntry,
) {
    // Mirror stack-resolution context as closely as possible, but without using the stack.
    let mut ctx = ExecutionContext::new(entry.object_id, entry.controller, decision_maker)
        .with_optional_costs_paid(entry.optional_costs_paid.clone())
        .with_cause(EventCause::from_effect(entry.object_id, entry.controller));
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
    if let Some(event_value_amount) = entry.event_value_amount {
        ctx = ctx.with_event_value_amount(event_value_amount);
    }
    if let Some(trigger_identity) = entry.trigger_identity {
        ctx = ctx.with_trigger_identity(trigger_identity);
    }
    if let Some(source_snapshot) = entry.source_snapshot.clone() {
        ctx = ctx.with_source_snapshot(source_snapshot);
    }
    if !entry.tagged_objects.is_empty() {
        ctx = ctx.with_tagged_objects(entry.tagged_objects.clone());
    }
    if let Some(ref modes) = entry.chosen_modes {
        ctx = ctx.with_chosen_modes(Some(modes.clone()));
    }
    apply_keyword_payment_tags_for_resolution(game, &entry, &mut ctx);

    let (valid_targets, valid_target_assignments, all_targets_invalid) =
        validate_stack_entry_targets(game, &entry);
    if !entry.targets.is_empty() && all_targets_invalid {
        return;
    }

    if let Some(trigger_identity) = entry.trigger_identity {
        game.record_triggered_ability_resolved(entry.object_id, trigger_identity);
    }

    if let Some(ref condition) = entry.intervening_if
        && let Some(ref triggering_event) = entry.triggering_event
        && !verify_intervening_if(
            game,
            condition,
            entry.controller,
            triggering_event,
            entry.object_id,
            None,
        )
    {
        return;
    }

    ctx = ctx
        .with_targets(valid_targets)
        .with_target_assignments(valid_target_assignments.clone());
    ctx.snapshot_targets(game);

    let effects = if let Some(ref ability_effects) = entry.ability_effects {
        ability_effects.clone()
    } else if let Some(obj) = game.object(entry.object_id) {
        get_effects_for_stack_entry(game, &entry, obj)
    } else {
        crate::resolution::ResolutionProgram::default()
    };

    let all_events = match super::stack_resolution::execute_resolution_program(
        game,
        &mut ctx,
        entry.controller,
        entry.object_id,
        &effects,
        entry.chosen_modes.as_deref(),
        &valid_target_assignments,
    ) {
        Ok(events) => events,
        Err(_) => return,
    };

    for event in all_events {
        queue_triggers_from_event(game, trigger_queue, event, false);
    }
    drain_pending_trigger_events(game, trigger_queue);
}

pub(super) fn resolve_triggered_mana_abilities_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) {
    loop {
        let mut pending = trigger_queue.take_all();
        if pending.is_empty() {
            break;
        }

        let active_player = game.turn.active_player;
        let mut active_mana_triggers = Vec::new();
        let mut other_mana_triggers = Vec::new();
        let mut remaining_triggers = Vec::new();

        for trigger in pending.drain(..) {
            if is_triggered_mana_ability(game, &trigger) {
                if trigger.controller == active_player {
                    active_mana_triggers.push(trigger);
                } else {
                    other_mana_triggers.push(trigger);
                }
            } else {
                remaining_triggers.push(trigger);
            }
        }

        if active_mana_triggers.is_empty() && other_mana_triggers.is_empty() {
            trigger_queue.entries.extend(remaining_triggers);
            break;
        }

        for trigger in active_mana_triggers
            .into_iter()
            .chain(other_mana_triggers.into_iter())
        {
            if !can_stack_trigger_this_turn(game, &trigger) {
                continue;
            }

            if let Some(entry) =
                create_triggered_stack_entry_with_targets(game, &trigger, decision_maker)
            {
                game.record_trigger_fired(trigger.source, trigger.trigger_identity);
                resolve_triggered_stack_entry_immediately(
                    game,
                    trigger_queue,
                    decision_maker,
                    entry,
                );
            }
        }

        // Preserve non-mana triggers while appending any triggers emitted during
        // immediate mana-trigger resolution.
        remaining_triggers.extend(trigger_queue.take_all());
        trigger_queue.entries.extend(remaining_triggers);
    }
}

pub(super) fn can_stack_trigger_this_turn(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
) -> bool {
    let Some(ref condition) = trigger.ability.intervening_if else {
        return true;
    };

    match condition {
        crate::ConditionExpr::FirstTimeThisTurn
        | crate::ConditionExpr::MaxTimesEachTurn(_)
        | crate::ConditionExpr::DoThisMaxTimesEachTurn(_) => verify_intervening_if(
            game,
            condition,
            trigger.controller,
            &trigger.triggering_event,
            trigger.source,
            Some(trigger.trigger_identity),
        ),
        _ => true,
    }
}

fn resolve_trigger_modal_count(
    value: &crate::effect::Value,
    x_value: Option<u32>,
    fallback: usize,
) -> usize {
    match value {
        crate::effect::Value::Fixed(n) => (*n).max(0) as usize,
        crate::effect::Value::X => x_value.map(|x| x as usize).unwrap_or(fallback),
        crate::effect::Value::XTimes(multiplier) => x_value
            .map(|x| ((x as i32) * *multiplier).max(0) as usize)
            .unwrap_or(fallback),
        _ => fallback,
    }
}

fn triggered_modal_spec(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
) -> Option<crate::effects::ModalSpec> {
    for effect in trigger.ability.effects.all_effects() {
        if let Some(spec) =
            effect
                .0
                .get_modal_spec_with_context(game, trigger.controller, trigger.source)
        {
            return Some(spec);
        }
    }

    None
}

fn selected_trigger_effects(trigger: &TriggeredAbilityEntry) -> Vec<Effect> {
    trigger.ability.effects.all_effects_owned()
}

fn mode_is_legal_for_trigger(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    effects: &[Effect],
    mode_idx: usize,
) -> bool {
    spell_has_legal_targets_with_mode_preview(
        game,
        effects,
        trigger.controller,
        Some(trigger.source),
        &[mode_idx],
    )
}

fn choose_trigger_modes(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    decision_maker: &mut dyn DecisionMaker,
) -> Option<Option<Vec<usize>>> {
    let Some(modal_spec) = triggered_modal_spec(game, trigger) else {
        return Some(None);
    };

    let effects = selected_trigger_effects(trigger);
    let max_modes = resolve_trigger_modal_count(
        &modal_spec.max_modes,
        trigger.x_value,
        modal_spec.mode_descriptions.len().max(1),
    );
    let min_modes = resolve_trigger_modal_count(&modal_spec.min_modes, trigger.x_value, max_modes);

    if max_modes == 0 || modal_spec.mode_descriptions.is_empty() {
        return Some(Some(Vec::new()));
    }

    let mode_options: Vec<crate::decisions::specs::ModeOption> = modal_spec
        .mode_descriptions
        .iter()
        .enumerate()
        .map(|(i, desc)| {
            crate::decisions::specs::ModeOption::with_legality(
                i,
                desc.clone(),
                mode_is_legal_for_trigger(game, trigger, &effects, i),
            )
        })
        .collect();

    if mode_options.iter().filter(|mode| mode.legal).count() < min_modes {
        return None;
    }

    let spec = crate::decisions::ModesSpec::new(
        trigger.source,
        mode_options,
        min_modes,
        max_modes,
        modal_spec.allow_repeated_modes,
    );
    let chosen: Vec<usize> = crate::decisions::make_decision(
        game,
        decision_maker,
        trigger.controller,
        Some(trigger.source),
        spec,
    );
    if decision_maker.awaiting_choice() {
        return None;
    }

    let mut valid = Vec::new();
    for idx in chosen {
        if !mode_is_legal_for_trigger(game, trigger, &effects, idx) {
            continue;
        }
        if !modal_spec.allow_repeated_modes && valid.contains(&idx) {
            continue;
        }
        valid.push(idx);
        if valid.len() >= max_modes {
            break;
        }
    }

    if valid.len() < min_modes {
        return None;
    }

    Some(Some(valid))
}

fn trigger_target_requirement_contexts(
    requirements: &[TargetRequirement],
) -> Vec<crate::decisions::context::TargetRequirementContext> {
    requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
            },
        )
        .collect()
}

fn target_requirements_from_explicit_choices(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    entry: &StackEntry,
) -> Vec<TargetRequirement> {
    let target_choices = trigger
        .ability
        .choices
        .iter()
        .filter(|choice| choice.is_target())
        .collect::<Vec<_>>();
    if target_choices.is_empty() {
        return Vec::new();
    }

    let mut tagged_objects: std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    > = std::collections::HashMap::new();
    if !entry.crew_contributors.is_empty() {
        let snapshots = entry
            .crew_contributors
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            tagged_objects.insert(crate::tag::TagKey::from("crewed_it_this_turn"), snapshots);
        }
    }
    if !entry.saddle_contributors.is_empty() {
        let snapshots = entry
            .saddle_contributors
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect::<Vec<_>>();
        if !snapshots.is_empty() {
            tagged_objects.insert(crate::tag::TagKey::from("saddled_it_this_turn"), snapshots);
        }
    }
    let tagged_objects_ref = if tagged_objects.is_empty() {
        None
    } else {
        Some(&tagged_objects)
    };
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);

    target_choices
        .into_iter()
        .map(|target_spec| {
            let count = target_spec.count();
            let legal_targets = compute_legal_targets_with_tagged_objects_and_view(
                game,
                target_spec,
                trigger.controller,
                Some(trigger.source),
                tagged_objects_ref,
                &view,
            );

            TargetRequirement {
                spec: (*target_spec).clone(),
                legal_targets,
                description: format!("target for {}", trigger.source_name),
                min_targets: count.min,
                max_targets: count.max,
            }
        })
        .collect()
}

fn choose_trigger_targets(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    requirements: &[TargetRequirement],
    decision_maker: &mut dyn DecisionMaker,
) -> Option<(Vec<Target>, Vec<crate::game_state::TargetAssignment>)> {
    if requirements.is_empty() {
        return Some((Vec::new(), Vec::new()));
    }

    if requirements
        .iter()
        .any(|requirement| requirement.legal_targets.len() < requirement.min_targets)
    {
        return None;
    }

    let requirement_contexts = trigger_target_requirement_contexts(requirements);
    let ctx = crate::decisions::context::TargetsContext::new(
        trigger.controller,
        trigger.source,
        format!("{}'s triggered ability", trigger.source_name),
        requirement_contexts.clone(),
    );

    let proposed_targets = decision_maker.decide_targets(game, &ctx);
    if decision_maker.awaiting_choice() {
        return None;
    }

    let selected_targets = crate::targeting::normalize_targets_for_requirements(
        &requirement_contexts,
        proposed_targets,
    )?;
    let ranges =
        crate::targeting::assigned_target_ranges(&requirement_contexts, &selected_targets)?;
    let assignments = requirements
        .iter()
        .zip(ranges)
        .map(|(requirement, range)| crate::game_state::TargetAssignment {
            spec: requirement.spec.clone(),
            range,
        })
        .collect();

    Some((selected_targets, assignments))
}

/// Create a stack entry for a triggered ability, handling target selection.
///
/// Returns None if the trigger has mandatory targets but no legal targets exist.
pub(super) fn create_triggered_stack_entry_with_targets(
    game: &mut GameState,
    trigger: &TriggeredAbilityEntry,
    decision_maker: &mut dyn DecisionMaker,
) -> Option<StackEntry> {
    let effects = game.cached_continuous_effects_snapshot();
    let mut entry = triggered_to_stack_entry_with_effects(game, trigger, &effects);
    if let Some(triggering_event) = entry.triggering_event.take() {
        let matched_node = game.provenance_graph_mut().alloc_child(
            triggering_event.provenance(),
            crate::provenance::ProvenanceNodeKind::TriggerMatched {
                source: trigger.source,
                controller: trigger.controller,
            },
        );
        entry.triggering_event = Some(triggering_event.with_provenance(matched_node));
    }

    if let Some(chosen_modes) = choose_trigger_modes(game, trigger, decision_maker)? {
        entry = entry.with_chosen_modes(Some(chosen_modes));
    }
    if decision_maker.awaiting_choice() {
        return None;
    }

    let mut requirements = target_requirements_from_explicit_choices(game, trigger, &entry);
    if let Some(ref chosen_modes) = entry.chosen_modes {
        requirements.extend(extract_target_requirements_from_program_with_modes(
            game,
            &trigger.ability.effects,
            trigger.controller,
            Some(trigger.source),
            Some(chosen_modes),
        ));
    }

    let (chosen_targets, target_assignments) =
        choose_trigger_targets(game, trigger, &requirements, decision_maker)?;
    if decision_maker.awaiting_choice() {
        return None;
    }
    entry.targets = chosen_targets;
    entry.target_assignments = target_assignments;

    Some(entry)
}

/// Convert a triggered ability entry to a stack entry.
pub(super) fn triggered_to_stack_entry_with_effects(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    effects: &[crate::continuous::ContinuousEffect],
) -> StackEntry {
    use crate::events::EventKind;
    use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
    use crate::events::zones::ZoneChangeEvent;
    use crate::triggers::AttackEventTarget;

    // Capture source LKI at trigger-to-stack time. If the source no longer exists,
    // fall back to snapshot data from the triggering event (e.g. dies triggers).
    let source_snapshot = game
        .object(trigger.source)
        .map(|obj| {
            ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                obj, game, effects,
            )
        })
        .or_else(|| trigger.source_snapshot.clone())
        .or_else(|| {
            trigger
                .triggering_event
                .downcast::<ZoneChangeEvent>()
                .and_then(|zc| zc.snapshot.clone())
                .filter(|snapshot| snapshot.object_id == trigger.source)
        })
        .or_else(|| {
            game.find_object_by_stable_id(trigger.source_stable_id)
                .and_then(|id| game.object(id))
                .map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                        obj, game, effects,
                    )
                })
        });

    // Create an ability stack entry with the effects from the triggered ability
    let mut entry = StackEntry::ability(
        trigger.source,
        trigger.controller,
        trigger.ability.effects.clone(),
    )
    .with_provenance(trigger.triggering_event.provenance())
    .with_source_info(trigger.source_stable_id, trigger.source_name.clone())
    .with_triggering_event(trigger.triggering_event.clone())
    .with_trigger_identity(trigger.trigger_identity);
    if let Some(event_value_amount) = trigger.event_value_amount {
        entry = entry.with_event_value_amount(event_value_amount);
    }
    if let Some(source_obj) = game.object(trigger.source) {
        entry = entry.with_optional_costs_paid(source_obj.optional_costs_paid.clone());
    }
    entry = entry.with_chosen_player(game.chosen_player(trigger.source));
    if !trigger.tagged_objects.is_empty() {
        entry = entry.with_tagged_objects(trigger.tagged_objects.clone());
    }
    if let Some(snapshot) = source_snapshot {
        entry = entry.with_source_snapshot(snapshot);
    }
    // If the source was cast with X, propagate that value to the triggered ability.
    if let Some(x) = trigger.x_value {
        entry = entry.with_x(x);
    } else if let Some(obj) = game.object(trigger.source)
        && let Some(x) = obj.x_value
    {
        entry = entry.with_x(x);
    } else if let Some(ref snapshot) = entry.source_snapshot
        && let Some(x) = snapshot.x_value
    {
        entry = entry.with_x(x);
    }
    // Propagate keyword payment contributions from the source permanent's cast,
    // so triggered abilities can reference "each creature that convoked it", etc.
    if let Some(obj) = game.object(trigger.source)
        && !obj.keyword_payment_contributions_to_cast.is_empty()
    {
        entry.keyword_payment_contributions = obj.keyword_payment_contributions_to_cast.clone();
    }

    if let Some(crewers) = game
        .turn_store
        .turn_history
        .crewed_this_turn
        .get(&trigger.source)
        && !crewers.is_empty()
    {
        entry.crew_contributors = crewers.clone();
    }

    if let Some(saddlers) = game
        .turn_store
        .turn_history
        .saddled_this_turn
        .get(&trigger.source)
        && !saddlers.is_empty()
    {
        entry.saddle_contributors = saddlers.clone();
    }

    // Copy intervening-if condition if present (must be rechecked at resolution time)
    if let Some(ref condition) = trigger.ability.intervening_if {
        entry = entry.with_intervening_if(condition.clone());
    }

    // Extract defending player from combat triggers
    if trigger.triggering_event.kind() == EventKind::CreatureAttacked
        && let Some(attacked) = trigger.triggering_event.downcast::<CreatureAttackedEvent>()
    {
        match attacked.target {
            AttackEventTarget::Player(player_id) => {
                entry = entry.with_defending_player(player_id);
            }
            AttackEventTarget::Planeswalker(planeswalker_id) => {
                if let Some(planeswalker) = game.object(planeswalker_id) {
                    entry = entry.with_defending_player(game.controller_of(planeswalker));
                }
            }
        }
    }
    if trigger.triggering_event.kind() == EventKind::CreatureAttackedAndUnblocked
        && let Some(attacked) = trigger
            .triggering_event
            .downcast::<CreatureAttackedAndUnblockedEvent>()
    {
        match attacked.target {
            AttackEventTarget::Player(player_id) => {
                entry = entry.with_defending_player(player_id);
            }
            AttackEventTarget::Planeswalker(planeswalker_id) => {
                if let Some(planeswalker) = game.object(planeswalker_id) {
                    entry = entry.with_defending_player(game.controller_of(planeswalker));
                }
            }
        }
    }
    if trigger.triggering_event.kind() == EventKind::CreatureBecameBlocked
        && let Some(blocked) = trigger
            .triggering_event
            .downcast::<CreatureBecameBlockedEvent>()
        && let Some(target) = blocked.attack_target
    {
        match target {
            AttackEventTarget::Player(player_id) => {
                entry = entry.with_defending_player(player_id);
            }
            AttackEventTarget::Planeswalker(planeswalker_id) => {
                if let Some(planeswalker) = game.object(planeswalker_id) {
                    entry = entry.with_defending_player(game.controller_of(planeswalker));
                }
            }
        }
    }

    if trigger.ability.trigger.saga_chapters().is_some() {
        entry = entry.with_chapter_ability_source(trigger.source);
    }

    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ChooseExileModeTarget {
        target: ObjectId,
        mode_prompts: usize,
        target_prompts: usize,
    }

    impl DecisionMaker for ChooseExileModeTarget {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.mode_prompts += 1;
            vec![1]
        }

        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.target_prompts += 1;
            assert!(
                ctx.requirements.iter().any(|requirement| requirement
                    .legal_targets
                    .contains(&Target::Object(self.target))),
                "graveyard card should be a legal modal trigger target"
            );
            vec![Target::Object(self.target)]
        }
    }

    #[test]
    fn modal_trigger_chooses_mode_then_targets_before_stacking() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let graveyard_target = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_100),
            "Graveyard Target",
        )
        .card_types(vec![CardType::Instant])
        .build();
        let graveyard_target_id =
            game.create_object_from_definition(&graveyard_target, bob, Zone::Graveyard);
        let graveyard_target_stable_id = game
            .object(graveyard_target_id)
            .expect("graveyard target should exist")
            .stable_id;

        let modal_effect = Effect::choose_one(vec![
            crate::effect::EffectMode::new(
                "Discard a card. If you do, draw a card.",
                vec![Effect::discard(1), Effect::draw(1)],
            ),
            crate::effect::EffectMode::new(
                "Exile up to one target card from a graveyard.",
                vec![Effect::exile(
                    ChooseSpec::target(ChooseSpec::card_in_zone(Zone::Graveyard))
                        .with_count(crate::effect::ChoiceCount::up_to(1)),
                )],
            ),
        ]);
        let kavu = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_101),
            "Territorial Kavu",
        )
        .card_types(vec![CardType::Creature])
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_attacks(),
            vec![modal_effect],
        ))
        .build();
        let kavu_id = game.create_object_from_definition(&kavu, alice, Zone::Battlefield);

        let attack_event = TriggerEvent::new_with_provenance(
            crate::events::combat::CreatureAttackedEvent::new(
                kavu_id,
                crate::triggers::AttackEventTarget::Player(bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &attack_event) {
            trigger_queue.add(trigger);
        }
        assert_eq!(trigger_queue.entries.len(), 1);

        let mut dm = ChooseExileModeTarget {
            target: graveyard_target_id,
            mode_prompts: 0,
            target_prompts: 0,
        };
        put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
            .expect("Kavu trigger should be put on the stack");

        assert_eq!(dm.mode_prompts, 1);
        assert_eq!(dm.target_prompts, 1);
        assert!(trigger_queue.is_empty());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].chosen_modes.as_deref(), Some(&[1][..]));
        assert_eq!(
            game.stack[0].targets,
            vec![Target::Object(graveyard_target_id)]
        );
        assert_eq!(game.stack[0].target_assignments.len(), 1);

        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Kavu trigger should resolve");
        let exiled_id = game
            .find_object_by_stable_id(graveyard_target_stable_id)
            .expect("target should still exist after moving zones");
        assert_eq!(
            game.object(exiled_id).map(|object| object.zone),
            Some(Zone::Exile)
        );
    }
}
