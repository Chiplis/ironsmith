use super::*;

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
        apply_legend_rule_choice, apply_state_based_actions_from_actions_with,
        check_state_based_actions_with_view, legend_rule_specs_from_actions,
    };

    // Refresh continuous state (static ability effects and "can't" effect tracking)
    // before checking SBAs. This ensures the layer system is up to date.
    game.refresh_continuous_state();

    loop {
        let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
        let all_effects = view.effects().to_vec();
        let actions = check_state_based_actions_with_view(game, &view);
        drop(view);
        if actions.is_empty() {
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
            let post_legend_actions = check_state_based_actions_with_view(game, &post_legend_view);
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
        // SBA moves queue primitive ZoneChangeEvent via move_object; consume them now.
        drain_pending_trigger_events(game, trigger_queue);
        if !applied && !had_legend_decisions {
            break;
        }
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

    for controller in controller_order {
        let Some(triggers) = grouped.remove(&controller) else {
            continue;
        };
        let ordered = order_triggers_for_controller(game, decision_maker, triggers);
        for trigger in ordered.into_iter().rev() {
            if !can_stack_trigger_this_turn(game, &trigger) {
                continue;
            }
            if let Some(entry) =
                create_triggered_stack_entry_with_targets(game, &trigger, decision_maker)
            {
                game.record_trigger_fired(trigger.source, trigger.trigger_identity);
                game.push_to_stack(entry);
            }
        }
    }

    Ok(())
}

fn players_in_apnap_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_order.len())
        .filter_map(|offset| {
            let player_id = game.turn_order[(start + offset) % game.turn_order.len()];
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
        .map(|(index, label)| (ObjectId::from_raw(u64::MAX - index as u64), label))
        .collect();
    let ctx = crate::decisions::context::enrich_display_hints(
        game,
        crate::decisions::context::DecisionContext::Order(
            crate::decisions::context::OrderContext::new(controller, None, description, items),
        ),
    )
    .into_order();
    let response = decision_maker.decide_order(game, &ctx);

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
    if !trigger.ability.choices.is_empty() {
        return false;
    }

    let Some(activated_event) = trigger
        .triggering_event
        .downcast::<crate::events::spells::AbilityActivatedEvent>()
    else {
        return false;
    };
    if !activated_event.is_mana_ability {
        return false;
    }

    effects_could_add_mana(
        game,
        trigger.source,
        trigger.controller,
        &trigger.ability.effects,
    )
}

pub(super) fn effects_could_add_mana(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    effects: &[crate::effect::Effect],
) -> bool {
    effects
        .iter()
        .any(|effect| effect_could_add_mana(game, source, controller, effect))
}

pub(super) fn effect_could_add_mana(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    effect: &crate::effect::Effect,
) -> bool {
    if effect
        .producible_mana_symbols(game, source, controller)
        .is_some_and(|symbols| !symbols.is_empty())
    {
        return true;
    }

    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return effects_could_add_mana(game, source, controller, &sequence.effects);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return effects_could_add_mana(game, source, controller, &may.effects);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return effects_could_add_mana(game, source, controller, &conditional.if_true)
            || effects_could_add_mana(game, source, controller, &conditional.if_false);
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return effects_could_add_mana(game, source, controller, &if_effect.then)
            || effects_could_add_mana(game, source, controller, &if_effect.else_);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_could_add_mana(game, source, controller, &with_id.effect);
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        return choose_mode
            .modes
            .iter()
            .any(|mode| effects_could_add_mana(game, source, controller, &mode.effects));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_could_add_mana(game, source, controller, &tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return effect_could_add_mana(game, source, controller, &tag_all.effect);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return effects_could_add_mana(game, source, controller, &for_each.effects);
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        return effects_could_add_mana(game, source, controller, &for_players.effects);
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return effects_could_add_mana(game, source, controller, &for_each_tagged.effects);
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return effects_could_add_mana(game, source, controller, &for_each_controller.effects);
    }
    if let Some(for_each_tagged_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return effects_could_add_mana(game, source, controller, &for_each_tagged_player.effects);
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        return effects_could_add_mana(game, source, controller, &unless_action.effects)
            || effects_could_add_mana(game, source, controller, &unless_action.alternative);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        return effects_could_add_mana(game, source, controller, &unless_pays.effects);
    }

    false
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
    if let Some(triggering_event) = entry.triggering_event.clone() {
        ctx = ctx.with_triggering_event(triggering_event);
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
        Vec::new()
    };

    let mut all_events = Vec::new();
    let mut consumed_modal_selection = false;
    let mut assignment_cursor = 0usize;
    for effect in &effects {
        let effect_target_assignments =
            super::stack_resolution::active_target_assignments_for_effect(
                game,
                effect,
                entry.controller,
                entry.object_id,
                entry.chosen_modes.as_deref(),
                &mut consumed_modal_selection,
                &valid_target_assignments,
                &mut assignment_cursor,
            );
        let outcome = ctx.with_temp_target_assignments(effect_target_assignments, |ctx| {
            execute_effect(game, effect, ctx)
        });
        if let Ok(outcome) = outcome {
            all_events.extend(outcome.events);
        }
    }

    for event in all_events {
        let event = game.ensure_trigger_event_provenance(event);
        let triggers = check_triggers(game, &event);
        for trigger in triggers {
            trigger_queue.add(trigger);
        }
    }
    drain_pending_trigger_events(game, trigger_queue);

    if let Some(saga_id) = entry.saga_final_chapter_source {
        mark_saga_final_chapter_resolved(game, saga_id);
    }
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
        crate::ConditionExpr::FirstTimeThisTurn | crate::ConditionExpr::MaxTimesEachTurn(_) => {
            verify_intervening_if(
                game,
                condition,
                trigger.controller,
                &trigger.triggering_event,
                trigger.source,
                Some(trigger.trigger_identity),
            )
        }
        _ => true,
    }
}

/// Create a stack entry for a triggered ability, handling target selection.
///
/// Returns None if the trigger has mandatory targets but no legal targets exist.
pub(super) fn create_triggered_stack_entry_with_targets(
    game: &mut GameState,
    trigger: &TriggeredAbilityEntry,
    decision_maker: &mut dyn DecisionMaker,
) -> Option<StackEntry> {
    let mut entry = triggered_to_stack_entry(game, trigger);
    if let Some(triggering_event) = entry.triggering_event.take() {
        let matched_node = game.provenance_graph.alloc_child(
            triggering_event.provenance(),
            crate::provenance::ProvenanceNodeKind::TriggerMatched {
                source: trigger.source,
                controller: trigger.controller,
            },
        );
        entry.triggering_event = Some(triggering_event.with_provenance(matched_node));
    }

    // Check if this trigger has targets that need to be selected
    if trigger.ability.choices.is_empty() {
        // No targets needed
        return Some(entry);
    }

    // Build tagged-object context for target selection when filters reference
    // saddle/crew contributors (e.g., "target creature that saddled it this turn").
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

    // Select targets for each target spec
    let mut chosen_targets = Vec::new();
    let mut target_assignments = Vec::new();
    for target_spec in &trigger.ability.choices {
        let count = target_spec.count();

        // Compute legal targets for this spec
        let legal_targets = compute_legal_targets_with_tagged_objects(
            game,
            target_spec,
            trigger.controller,
            Some(trigger.source),
            tagged_objects_ref,
        );

        if legal_targets.len() < count.min {
            // Mandatory targets are missing, so the trigger can't go on the stack.
            return None;
        }

        // Create a context for target selection
        let ctx = crate::decisions::context::TargetsContext::new(
            trigger.controller,
            trigger.source,
            format!("{}'s triggered ability", trigger.source_name),
            vec![crate::decisions::context::TargetRequirementContext {
                description: format!("target for {}", trigger.source_name),
                legal_targets: legal_targets.clone(),
                min_targets: count.min,
                max_targets: count.max,
            }],
        );

        // Get the choice from the decision maker
        let mut selected_targets = Vec::new();
        for target in decision_maker.decide_targets(game, &ctx) {
            if !legal_targets.contains(&target) || selected_targets.contains(&target) {
                continue;
            }
            selected_targets.push(target);
            if let Some(max_targets) = count.max
                && selected_targets.len() >= max_targets
            {
                break;
            }
        }

        if selected_targets.len() < count.min {
            for legal_target in &legal_targets {
                if selected_targets.len() >= count.min {
                    break;
                }
                if !selected_targets.contains(legal_target) {
                    selected_targets.push(*legal_target);
                }
            }
        }

        if selected_targets.len() < count.min {
            return None;
        }

        let start = chosen_targets.len();
        chosen_targets.extend(selected_targets);
        let end = chosen_targets.len();
        target_assignments.push(crate::game_state::TargetAssignment {
            spec: target_spec.clone(),
            range: start..end,
        });
    }

    // Add the chosen targets to the stack entry
    entry.targets = chosen_targets;
    entry.target_assignments = target_assignments;

    Some(entry)
}

/// Convert a triggered ability entry to a stack entry.
pub(super) fn triggered_to_stack_entry(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
) -> StackEntry {
    use crate::events::EventKind;
    use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
    use crate::events::zones::ZoneChangeEvent;
    use crate::triggers::AttackEventTarget;

    // Capture source LKI at trigger-to-stack time. If the source no longer exists,
    // fall back to snapshot data from the triggering event (e.g. dies triggers).
    let source_snapshot = game
        .object(trigger.source)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
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
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        });

    // Create an ability stack entry with the effects from the triggered ability
    let mut entry = StackEntry::ability(
        trigger.source,
        trigger.controller,
        trigger.ability.effects.clone(),
    )
    .with_source_info(trigger.source_stable_id, trigger.source_name.clone())
    .with_triggering_event(trigger.triggering_event.clone());
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

    if let Some(crewers) = game.crewed_this_turn.get(&trigger.source)
        && !crewers.is_empty()
    {
        entry.crew_contributors = crewers.clone();
    }

    if let Some(saddlers) = game.saddled_this_turn.get(&trigger.source)
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
                    entry = entry.with_defending_player(planeswalker.controller);
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
                    entry = entry.with_defending_player(planeswalker.controller);
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
                    entry = entry.with_defending_player(planeswalker.controller);
                }
            }
        }
    }

    // Check if this is a saga's final chapter ability.
    // Use trigger metadata directly instead of parsing display strings.
    if let Some(chapters) = trigger.ability.trigger.saga_chapters()
        && let Some(saga_obj) = game.object(trigger.source)
    {
        let max_chapter = saga_obj.max_saga_chapter.unwrap_or(0);
        if chapters.iter().any(|&ch| ch >= max_chapter) {
            entry = entry.with_saga_final_chapter(trigger.source);
        }
    }

    entry
}
