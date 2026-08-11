use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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
        StateBasedAction, StateBasedActionContext, apply_legend_rule_choice_from_group,
        apply_sector_designation_choices_from_group, apply_state_based_actions_from_actions_with,
        check_state_based_actions_with_context, legend_rule_specs_from_actions,
    };

    // Refresh continuous state (static ability effects and "can't" effect tracking)
    // before checking SBAs. This ensures the layer system is up to date.
    game.refresh_continuous_state();
    let mut seen_mandatory_states = std::collections::HashSet::new();

    loop {
        if restore_unattached_bestow_creatures(game) {
            game.refresh_continuous_state();
        }
        game.refresh_continuous_state();
        let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
        let context = StateBasedActionContext::from_trigger_queue(trigger_queue);
        let actions = check_state_based_actions_with_context(game, &view, &context);
        let all_effects = view.effects_arc();
        drop(view);
        if actions.is_empty() {
            game.clear_pending_sector_designations();
            game.clear_empty_library_draw_attempts_since_sba();
            game.clear_deathtouch_damage_since_sba();
            break;
        }

        // CR 704.5u requires every sector choice to be made against the same
        // pre-commit game state. Keep partial asynchronous answers in GameState
        // so the native priority loop and external/WASM driver resume the same
        // immutable proposal instead of replaying or partially committing it.
        let sector_action = actions.iter().find_map(|action| match action {
            StateBasedAction::SectorDesignationChoices { source, creatures } => {
                Some((*source, creatures.clone()))
            }
            _ => None,
        });
        if let Some((source, creatures)) = sector_action {
            seen_mandatory_states.clear();
            let mut pending = match game.take_pending_sector_designations() {
                Some(pending) if pending.source == source && pending.creatures == creatures => {
                    pending
                }
                _ => crate::game_state::PendingSectorDesignationState {
                    source,
                    creatures,
                    choices: Vec::new(),
                },
            };
            let options = crate::marker::SectorDesignation::ALL
                .into_iter()
                .enumerate()
                .map(|(index, sector)| {
                    crate::decisions::context::SelectableOption::new(index, sector.description())
                })
                .collect::<Vec<_>>();
            while pending.choices.len() < pending.creatures.len() {
                let (player, creature) = pending.creatures[pending.choices.len()];
                let name = game
                    .object(creature)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "this creature".to_string());
                let context = crate::decisions::context::SelectOptionsContext::new(
                    player,
                    Some(pending.source),
                    format!("Choose a sector for {name}"),
                    options.clone(),
                    1,
                    1,
                );
                let index = decision_maker
                    .decide_options(game, &context)
                    .first()
                    .copied()
                    .unwrap_or(0);
                if decision_maker.awaiting_choice() {
                    game.set_pending_sector_designations(pending);
                    return Ok(());
                }
                pending.choices.push(
                    crate::marker::SectorDesignation::from_option_index(index)
                        .unwrap_or(crate::marker::SectorDesignation::Alpha),
                );
            }
            apply_sector_designation_choices_from_group(
                game,
                pending.source,
                &pending.creatures,
                &pending.choices,
            );
            continue;
        }
        game.clear_pending_sector_designations();

        // Handle legend rule decisions first
        let legend_specs = legend_rule_specs_from_actions(&actions);
        let had_legend_decisions = !legend_specs.is_empty();
        if had_legend_decisions {
            // A player choice participates in this procedure, so it cannot prove
            // a mandatory-action draw. Start a fresh candidate after the choice.
            seen_mandatory_states.clear();
        } else {
            let fingerprint = sba_control_fingerprint(game, &actions);
            if !seen_mandatory_states.insert(fingerprint) {
                return Err(GameLoopError::MandatoryLoopDraw);
            }
        }
        for (player, spec) in legend_specs {
            let legend_group = spec.legends.clone();
            let keep_id: ObjectId = make_decision(game, decision_maker, player, None, spec);
            if decision_maker.awaiting_choice() {
                // The prompt was only surfaced; committing the fallback here would
                // advance local state past a choice the replay log doesn't contain yet.
                return Ok(());
            }
            apply_legend_rule_choice_from_group(game, keep_id, &legend_group);
        }

        // Apply the SBAs (legend rule already handled above)
        // Use the decision maker version to allow interactive replacement effect choices
        let applied = if had_legend_decisions {
            game.refresh_continuous_state();
            let post_legend_view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
            let post_legend_context = StateBasedActionContext::from_trigger_queue(trigger_queue);
            let post_legend_actions = check_state_based_actions_with_context(
                game,
                &post_legend_view,
                &post_legend_context,
            );
            let post_legend_effects = post_legend_view.effects_arc();
            drop(post_legend_view);
            apply_state_based_actions_from_actions_with(
                game,
                post_legend_actions,
                post_legend_effects.as_slice(),
                decision_maker,
            )
        } else {
            apply_state_based_actions_from_actions_with(
                game,
                actions,
                all_effects.as_slice(),
                decision_maker,
            )
        };
        if decision_maker.awaiting_choice() {
            return Ok(());
        }
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

fn sba_control_fingerprint(
    game: &GameState,
    actions: &[crate::rules::state_based::StateBasedAction],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{actions:?}").hash(&mut hasher);
    format!("{:?}", game.players).hash(&mut hasher);
    format!("{:?}", game.turn).hash(&mut hasher);
    for object_id in game.object_ids_in_deterministic_order() {
        if let Some(object) = game.object(object_id) {
            format!("{object:?}").hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn restore_unattached_bestow_creatures(game: &mut GameState) -> bool {
    let candidates = game
        .battlefield
        .iter()
        .copied()
        .filter(|&object_id| {
            let Some(object) = game.object(object_id) else {
                return false;
            };
            if !object.is_bestow_overlay_active() {
                return false;
            }
            match object.attached_to {
                None => true,
                Some(target) => {
                    !crate::effects::permanents::attachment_can_attach_to_target(
                        game, object_id, target,
                    ) || matches!(
                        target,
                        crate::object::AttachmentTarget::Object(attached_id)
                            if crate::targeting::has_protection_from_source(
                                game,
                                attached_id,
                                object_id,
                            )
                    )
                }
            }
        })
        .collect::<Vec<_>>();

    for object_id in &candidates {
        game.detach_object_from_current_target(*object_id);
        if let Some(object) = game.object_mut(*object_id) {
            object.end_bestow_cast_overlay();
        }
    }

    !candidates.is_empty()
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
/// This handles the full CR 603.3b flow, including the separate APNAP pass for
/// abilities whose trigger condition is another ability triggering.
pub fn put_triggers_on_stack_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    game.refresh_continuous_state();
    let mut announced_counts = std::collections::HashMap::<
        (crate::ids::ObjectId, crate::triggers::TriggerIdentity),
        u32,
    >::new();

    loop {
        drain_pending_trigger_events(game, trigger_queue);
        drain_ability_triggered_events(game, trigger_queue, &mut announced_counts)?;

        // Triggered mana abilities resolve immediately and never use the stack.
        resolve_triggered_mana_abilities_with_dm(game, trigger_queue, decision_maker)?;
        announced_counts.retain(|(source, trigger_identity), _| {
            trigger_queue
                .entries
                .iter()
                .any(|entry| entry.source == *source && entry.trigger_identity == *trigger_identity)
        });
        if decision_maker.awaiting_choice() {
            return Ok(());
        }

        // Immediate mana-trigger resolution can itself create events/triggers.
        if !game.effect_store.pending_trigger_events.is_empty()
            || trigger_queue.has_ability_triggered_events()
        {
            continue;
        }

        let mut ordinary = Vec::new();
        let mut triggered_by_ability = Vec::new();
        for trigger in trigger_queue.take_all() {
            if trigger.triggering_event.kind() == crate::events::EventKind::AbilityTriggered {
                triggered_by_ability.push(trigger);
            } else {
                ordinary.push(trigger);
            }
        }

        if ordinary.is_empty() && triggered_by_ability.is_empty() {
            return Ok(());
        }

        // First complete APNAP pass: trigger conditions other than another
        // ability triggering.
        if stack_trigger_pass(game, trigger_queue, decision_maker, ordinary) {
            for trigger in triggered_by_ability {
                trigger_queue.requeue(trigger);
            }
            return Ok(());
        }

        // Second complete APNAP pass: all remaining trigger-on-trigger entries.
        if stack_trigger_pass(game, trigger_queue, decision_maker, triggered_by_ability) {
            return Ok(());
        }
        announced_counts.clear();

        // Putting abilities on the stack (including target selection) can cause
        // new triggers. CR 603.3b requires another SBA/stacking cycle before
        // priority is granted.
        check_and_apply_sbas_with(game, trigger_queue, decision_maker)?;
        if decision_maker.awaiting_choice() {
            return Ok(());
        }
    }
}

fn drain_ability_triggered_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    announced_counts: &mut std::collections::HashMap<
        (crate::ids::ObjectId, crate::triggers::TriggerIdentity),
        u32,
    >,
) -> Result<(), GameLoopError> {
    let mut seen_generations = std::collections::HashSet::new();
    loop {
        let pending = trigger_queue.take_ability_triggered_events();
        if pending.is_empty() {
            return Ok(());
        }

        let pending = pending
            .into_iter()
            .filter_map(|(event, trigger_limit)| {
                let ability_event = event.downcast::<crate::events::AbilityTriggeredEvent>()?;
                if let Some(trigger_limit) = trigger_limit {
                    let key = (ability_event.source, ability_event.trigger_identity);
                    let already_fired = game.trigger_fire_count_this_turn(
                        ability_event.source,
                        ability_event.trigger_identity,
                    );
                    let announced = announced_counts.get(&key).copied().unwrap_or(0);
                    if already_fired.saturating_add(announced) >= trigger_limit {
                        return None;
                    }
                    *announced_counts.entry(key).or_default() += 1;
                }
                Some(event)
            })
            .collect::<Vec<_>>();

        let generation = pending
            .iter()
            .filter_map(|event| event.downcast::<crate::events::AbilityTriggeredEvent>())
            .map(|event| (event.source_stable_id, event.trigger_identity))
            .collect::<std::collections::HashSet<_>>();
        if generation
            .iter()
            .any(|identity| seen_generations.contains(identity))
        {
            return Err(GameLoopError::MandatoryLoopDraw);
        }
        seen_generations.extend(generation);

        for event in pending {
            let parent = game.ensure_trigger_event_provenance(event);
            let provenance = game.alloc_child_event_provenance(
                parent.provenance(),
                crate::events::EventKind::AbilityTriggered,
            );
            queue_triggers_from_event(
                game,
                trigger_queue,
                parent.with_provenance(provenance),
                true,
            );
        }
    }
}

/// Stack one CR 603.3b APNAP pass. Returns true when an interactive decision
/// paused the pass and all unstacked entries were restored without re-emitting
/// their AbilityTriggered events.
fn stack_trigger_pass(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
    triggers: Vec<TriggeredAbilityEntry>,
) -> bool {
    if triggers.is_empty() {
        return false;
    }

    // Group triggers by controller, then let each controller order their own
    // simultaneous triggers before applying APNAP stack placement.
    let mut grouped: std::collections::HashMap<PlayerId, Vec<TriggeredAbilityEntry>> =
        std::collections::HashMap::new();

    for trigger in triggers {
        // CR 800.4d: triggered abilities controlled by a player who left the
        // game are never put onto the stack.
        if game
            .player(trigger.controller)
            .is_some_and(|player| player.is_in_game())
        {
            let decision_player = game
                .primary_player_for(trigger.controller)
                .unwrap_or(trigger.controller);
            grouped.entry(decision_player).or_default().push(trigger);
        }
    }

    let controller_order = players_in_apnap_order(game);

    for (controller_index, controller) in controller_order.iter().copied().enumerate() {
        let Some(triggers) = grouped.remove(&controller) else {
            continue;
        };
        let ordered = order_triggers_for_controller(game, decision_maker, controller, triggers);
        if decision_maker.awaiting_choice() {
            for trigger in ordered {
                trigger_queue.requeue(trigger);
            }
            for remaining_controller in controller_order.iter().copied().skip(controller_index + 1)
            {
                if let Some(remaining_triggers) = grouped.remove(&remaining_controller) {
                    for trigger in remaining_triggers {
                        trigger_queue.requeue(trigger);
                    }
                }
            }
            return true;
        }
        let ordered_for_stacking: Vec<_> = ordered.into_iter().rev().collect();
        for (index, trigger) in ordered_for_stacking.iter().enumerate() {
            if !can_stack_trigger_this_turn(game, &trigger) {
                continue;
            }

            let origin_marker = game.grand_melee().map(|state| state.focused_marker());
            let destination_marker =
                match grand_melee_trigger_stack_destination(game, decision_maker, trigger) {
                    TriggerStackDestination::Selected(marker) => marker,
                    TriggerStackDestination::AwaitingChoice => {
                        for deferred in ordered_for_stacking.iter().skip(index).rev() {
                            trigger_queue.requeue(deferred.clone());
                        }
                        for remaining_controller in
                            controller_order.iter().copied().skip(controller_index + 1)
                        {
                            if let Some(remaining_triggers) = grouped.remove(&remaining_controller)
                            {
                                for trigger in remaining_triggers {
                                    trigger_queue.requeue(trigger);
                                }
                            }
                        }
                        return true;
                    }
                };
            if destination_marker != origin_marker {
                game.select_grand_melee_turn_marker(
                    destination_marker.expect("Grand Melee destination marker"),
                )
                .expect("eligible Grand Melee trigger destination remains active");
            }
            if let Some(entry) = create_triggered_stack_entry_with_targets(
                game,
                trigger,
                decision_maker,
                trigger_queue,
            ) {
                if decision_maker.awaiting_choice() {
                    if destination_marker != origin_marker
                        && let Some(origin_marker) = origin_marker
                    {
                        game.select_grand_melee_turn_marker(origin_marker)
                            .expect("original Grand Melee trigger lane remains active");
                    }
                    for deferred in ordered_for_stacking.iter().skip(index).rev() {
                        trigger_queue.requeue(deferred.clone());
                    }
                    for remaining_controller in
                        controller_order.iter().copied().skip(controller_index + 1)
                    {
                        if let Some(remaining_triggers) = grouped.remove(&remaining_controller) {
                            for trigger in remaining_triggers {
                                trigger_queue.requeue(trigger);
                            }
                        }
                    }
                    return true;
                }
                game.record_trigger_fired(trigger.source, trigger.trigger_identity);
                let targets = entry.targets.clone();
                let object_id = entry.object_id;
                let controller = entry.controller;
                let provenance = entry.provenance;
                game.push_to_stack(entry);
                queue_becomes_targeted_events(
                    game,
                    trigger_queue,
                    &targets,
                    object_id,
                    controller,
                    true,
                    provenance,
                );
                if destination_marker != origin_marker
                    && let Some(origin_marker) = origin_marker
                {
                    game.select_grand_melee_turn_marker(origin_marker)
                        .expect("original Grand Melee trigger lane remains active");
                }
            } else if decision_maker.awaiting_choice() {
                if destination_marker != origin_marker
                    && let Some(origin_marker) = origin_marker
                {
                    game.select_grand_melee_turn_marker(origin_marker)
                        .expect("original Grand Melee trigger lane remains active");
                }
                for deferred in ordered_for_stacking.iter().skip(index).rev() {
                    trigger_queue.requeue(deferred.clone());
                }
                for remaining_controller in
                    controller_order.iter().copied().skip(controller_index + 1)
                {
                    if let Some(remaining_triggers) = grouped.remove(&remaining_controller) {
                        for trigger in remaining_triggers {
                            trigger_queue.requeue(trigger);
                        }
                    }
                }
                return true;
            } else if destination_marker != origin_marker
                && let Some(origin_marker) = origin_marker
            {
                game.select_grand_melee_turn_marker(origin_marker)
                    .expect("original Grand Melee trigger lane remains active");
            }
        }
    }

    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TriggerStackDestination {
    Selected(Option<u32>),
    AwaitingChoice,
}

/// Apply CR 807.5b before targets are chosen so the selected marker's range
/// and stack objects define the legal target set. A causal stack object always
/// binds the trigger to its own lane; otherwise a controller with priority for
/// multiple marker stacks explicitly chooses one.
fn grand_melee_trigger_stack_destination(
    game: &GameState,
    decision_maker: &mut dyn DecisionMaker,
    trigger: &TriggeredAbilityEntry,
) -> TriggerStackDestination {
    let Some(state) = game.grand_melee() else {
        return TriggerStackDestination::Selected(None);
    };

    if let Some(marker) = game.grand_melee_stack_marker_for_cause(
        trigger.triggering_event.source_object(),
        trigger.triggering_event.provenance(),
    ) {
        return TriggerStackDestination::Selected(Some(marker));
    }

    let markers = game.grand_melee_priority_markers_for(trigger.controller);
    match markers.as_slice() {
        [] => TriggerStackDestination::Selected(Some(state.focused_marker())),
        [marker] => TriggerStackDestination::Selected(Some(*marker)),
        _ => {
            let options = markers
                .iter()
                .enumerate()
                .map(|(index, marker)| {
                    crate::decisions::context::SelectableOption::new(
                        index,
                        format!("Turn marker {marker} stack"),
                    )
                })
                .collect();
            let context = crate::decisions::context::SelectOptionsContext::new(
                trigger.controller,
                Some(trigger.source),
                format!(
                    "Choose the Grand Melee stack for {}'s triggered ability",
                    trigger.source_name
                ),
                options,
                1,
                1,
            );
            let selected = decision_maker
                .decide_options(game, &context)
                .first()
                .copied()
                .unwrap_or(0);
            if decision_maker.awaiting_choice() {
                TriggerStackDestination::AwaitingChoice
            } else {
                TriggerStackDestination::Selected(Some(
                    markers.get(selected).copied().unwrap_or(markers[0]),
                ))
            }
        }
    }
}

fn players_in_apnap_order(game: &GameState) -> Vec<PlayerId> {
    if game.shared_team_turns_enabled() {
        return game
            .team_apnap_player_order()
            .into_iter()
            .filter_map(|player| {
                let team = game.team_index_for(player)?;
                game.primary_player_for_team(team)
            })
            .fold(Vec::new(), |mut players, player| {
                if !players.contains(&player) {
                    players.push(player);
                }
                players
            });
    }
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
    let detail = if !trigger_text.trim().is_empty() && !effect_text.trim().is_empty() {
        format!("{trigger_text}\n{effect_text}")
    } else if !effect_text.trim().is_empty() {
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
    decision_player: PlayerId,
    triggers: Vec<TriggeredAbilityEntry>,
) -> Vec<TriggeredAbilityEntry> {
    if triggers.len() <= 1 {
        return triggers;
    }

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
            crate::decisions::context::OrderContext::new(decision_player, None, description, items),
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
    if let Some(triggering_event) = entry.triggering_event.clone() {
        if let Some(attacked) =
            triggering_event.downcast::<crate::events::combat::CreatureAttackedEvent>()
        {
            if let Some(attacker) = game.object(attacked.attacker) {
                ctx = ctx.with_attacking_player(game.controller_of(attacker));
            }
        } else if let Some(attacked) =
            triggering_event.downcast::<crate::events::combat::CreatureAttackedAndUnblockedEvent>()
            && let Some(attacker) = game.object(attacked.attacker)
        {
            ctx = ctx.with_attacking_player(game.controller_of(attacker));
        }
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
            Some(&entry.optional_costs_paid),
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
) -> Result<(), GameLoopError> {
    let mut mandatory_loop = super::mandatory_loop::MandatoryLoopTracker::default();
    loop {
        let mut pending = trigger_queue.take_all();
        if pending.is_empty() {
            break;
        }

        let mut active_mana_triggers = Vec::new();
        let mut other_mana_triggers = Vec::new();
        let mut remaining_triggers = Vec::new();

        for trigger in pending.drain(..) {
            if is_triggered_mana_ability(game, &trigger) {
                if game.is_active_player(trigger.controller) {
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

            if let Some(entry) = create_triggered_stack_entry_with_targets(
                game,
                &trigger,
                decision_maker,
                trigger_queue,
            ) {
                let resolved =
                    super::mandatory_loop::MandatoryProcedureObservation::from_stack_entry(
                        game, &entry,
                    );
                let queued_before_resolution = trigger_queue.entries.len();
                game.record_trigger_fired(trigger.source, trigger.trigger_identity);
                resolve_triggered_stack_entry_immediately(
                    game,
                    trigger_queue,
                    decision_maker,
                    entry,
                );
                let queued = trigger_queue
                    .entries
                    .iter()
                    .skip(queued_before_resolution)
                    .map(|entry| {
                        super::mandatory_loop::MandatoryProcedureObservation::from_trigger_entry(
                            game, entry,
                        )
                    })
                    .collect::<Vec<_>>();
                if let Some(controllers) = mandatory_loop.observe_resolution(resolved, queued) {
                    game.mark_mandatory_loop_draw_for(controllers);
                    return Err(GameLoopError::MandatoryLoopDraw);
                }
            }
        }

        // Preserve non-mana triggers while appending any triggers emitted during
        // immediate mana-trigger resolution.
        remaining_triggers.extend(trigger_queue.take_all());
        trigger_queue.entries.extend(remaining_triggers);
    }

    Ok(())
}

pub(super) fn can_stack_trigger_this_turn(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
) -> bool {
    let Some(ref condition) = trigger.ability.intervening_if else {
        return true;
    };

    verify_intervening_if(
        game,
        condition,
        trigger.controller,
        &trigger.triggering_event,
        trigger.source,
        Some(trigger.trigger_identity),
        None,
    )
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
        modal_spec.mode_point_costs.clone(),
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
    let mut selected_point_total = 0usize;
    for idx in chosen {
        if !mode_is_legal_for_trigger(game, trigger, &effects, idx) {
            continue;
        }
        if !modal_spec.allow_repeated_modes && valid.contains(&idx) {
            continue;
        }
        let point_cost = modal_spec
            .mode_point_costs
            .get(idx)
            .copied()
            .unwrap_or(1)
            .max(1) as usize;
        if selected_point_total.saturating_add(point_cost) > max_modes {
            continue;
        }
        valid.push(idx);
        selected_point_total += point_cost;
        if selected_point_total >= max_modes {
            break;
        }
    }

    if selected_point_total < min_modes {
        return None;
    }
    if !spell_has_legal_targets_with_modes(
        game,
        &effects,
        trigger.controller,
        Some(trigger.source),
        Some(&valid),
    ) {
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
                legal_target_sets: requirement.legal_target_sets.clone(),
                aggregate_constraint: requirement.aggregate_constraint.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
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
    tagged_objects.extend(trigger.tagged_objects.clone());
    add_triggering_object_tag(game, &trigger.triggering_event, &mut tagged_objects);
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
    let attacking_player = entry
        .triggering_event
        .as_ref()
        .and_then(|event| event.object_id())
        .and_then(|attacker| game.object(attacker))
        .map(|attacker| game.controller_of(attacker));

    target_choices
        .into_iter()
        .map(|target_spec| {
            let count = target_spec.count();
            let resolved_target_spec = super::targeting::choose_spec_with_damaged_player_from_event(
                target_spec,
                entry.triggering_event.as_ref(),
            );
            let legal_targets = compute_legal_targets_with_tagged_objects_combat_context_and_view(
                game,
                &resolved_target_spec,
                trigger.controller,
                Some(trigger.source),
                entry.source_snapshot.as_ref(),
                tagged_objects_ref,
                entry.defending_player,
                attacking_player,
                &view,
            );

            let legal_target_sets = crate::targeting::legal_target_sets_for_spec(
                game,
                &resolved_target_spec,
                &legal_targets,
            );
            let aggregate_constraint = crate::targeting::resolved_target_aggregate_constraint(
                game,
                &resolved_target_spec,
                trigger.controller,
                Some(trigger.source),
                &legal_targets,
            );

            TargetRequirement {
                spec: resolved_target_spec,
                chooser: None,
                legal_targets,
                legal_target_sets,
                aggregate_constraint,
                description: format!("target for {}", trigger.source_name),
                min_targets: count.min,
                max_targets: count.max,
                distinct_player_group: None,
                distribution_value: None,
                distribution_min_per_target: 1,
            }
        })
        .collect()
}

fn target_requirements_overlap(left: &TargetRequirement, right: &TargetRequirement) -> bool {
    target_requirement_reuses_existing(left, std::slice::from_ref(right))
        || target_requirement_reuses_existing(right, std::slice::from_ref(left))
}

fn target_requirements_cover_existing(
    candidates: &[TargetRequirement],
    required: &[TargetRequirement],
) -> bool {
    let mut used = vec![false; candidates.len()];
    for requirement in required {
        let Some(index) = candidates
            .iter()
            .enumerate()
            .find_map(|(index, candidate)| {
                (!used[index] && target_requirements_overlap(candidate, requirement))
                    .then_some(index)
            })
        else {
            return false;
        };
        used[index] = true;
    }
    true
}

fn refresh_trigger_program_target_requirements(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    entry: &StackEntry,
    requirements: &mut [TargetRequirement],
) {
    if requirements.is_empty() {
        return;
    }

    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    let tagged_objects = (!entry.tagged_objects.is_empty()).then_some(&entry.tagged_objects);
    let attacking_player = entry
        .triggering_event
        .as_ref()
        .and_then(|event| event.object_id())
        .and_then(|attacker| game.object(attacker))
        .map(|attacker| game.controller_of(attacker));

    for requirement in requirements {
        let spec = super::targeting::choose_spec_with_damaged_player_from_event(
            &requirement.spec,
            entry.triggering_event.as_ref(),
        );
        requirement.legal_targets =
            compute_legal_targets_with_tagged_objects_combat_context_and_view(
                game,
                &spec,
                trigger.controller,
                Some(trigger.source),
                entry.source_snapshot.as_ref(),
                tagged_objects,
                entry.defending_player,
                attacking_player,
                &view,
            );
        requirement.legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, &spec, &requirement.legal_targets);
        requirement.aggregate_constraint = crate::targeting::resolved_target_aggregate_constraint(
            game,
            &spec,
            trigger.controller,
            Some(trigger.source),
            &requirement.legal_targets,
        );
        requirement.spec = spec;
    }
}

fn add_triggering_object_tag(
    game: &GameState,
    triggering_event: &TriggerEvent,
    tagged_objects: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
) {
    if let Some(object_id) = triggering_event.object_id()
        && let Some(snapshot) = triggering_event.snapshot().cloned().or_else(|| {
            game.object(object_id)
                .map(|obj| ObjectSnapshot::from_object(obj, game))
        })
    {
        tagged_objects
            .entry(crate::tag::TagKey::from("__it__"))
            .or_default()
            .push(snapshot.clone());
        tagged_objects
            .entry(crate::tag::TagKey::from("triggering"))
            .or_default()
            .push(snapshot);
    }
}

fn resolve_trigger_target_chooser(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    entry: &StackEntry,
    chooser: Option<&crate::target::PlayerFilter>,
) -> Option<PlayerId> {
    let Some(chooser) = chooser else {
        return Some(trigger.controller);
    };

    let mut filter_ctx = game
        .filter_context_for(trigger.controller, Some(trigger.source))
        .with_active_player(game.turn.active_player)
        .with_tagged_objects(&entry.tagged_objects);
    filter_ctx.defending_player = entry.defending_player;
    filter_ctx.chosen_player = entry.chosen_player;
    filter_ctx.attacking_player = entry
        .triggering_event
        .as_ref()
        .and_then(|event| event.object_id())
        .and_then(|object_id| game.object(object_id))
        .map(|object| game.controller_of(object));

    let matches = game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .filter_map(|player| {
            crate::filter::player_filter_matches_game(chooser, player.id, game, &filter_ctx)
                .then_some(player.id)
        })
        .collect::<Vec<_>>();
    if let [resolved] = matches.as_slice() {
        return Some(*resolved);
    }
    if matches.is_empty() && game.limited_range_of_influence().is_some() {
        filter_ctx.players_in_range = None;
        return game.closest_in_game_player_to_left_matching(trigger.controller, |candidate| {
            crate::filter::player_filter_matches_game(chooser, candidate, game, &filter_ctx)
        });
    }
    None
}

fn choose_trigger_targets_with_one_chooser(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    chooser: PlayerId,
    requirements: &[TargetRequirement],
    decision_maker: &mut dyn DecisionMaker,
) -> Option<(Vec<Target>, Vec<crate::game_state::TargetAssignment>)> {
    let mut requirement_contexts = trigger_target_requirement_contexts(requirements);
    if !game.source_snapshot_is_exempt_from_range(
        Some(trigger.source),
        trigger.source_snapshot.as_ref(),
    ) {
        for requirement in &mut requirement_contexts {
            requirement.legal_targets.retain(|target| match target {
                Target::Player(player) => game.player_is_within_range(chooser, *player),
                Target::Object(object) => {
                    game.object_is_within_range(chooser, *object, Some(trigger.source))
                }
            });
            requirement.legal_target_sets.retain(|set| {
                set.iter()
                    .all(|target| requirement.legal_targets.contains(target))
            });
        }
    }
    if requirement_contexts
        .iter()
        .any(|requirement| requirement.legal_targets.len() < requirement.min_targets)
    {
        return None;
    }
    let ctx = crate::decisions::context::TargetsContext::new(
        chooser,
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

fn choose_trigger_targets(
    game: &GameState,
    trigger: &TriggeredAbilityEntry,
    entry: &StackEntry,
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

    let choosers = requirements
        .iter()
        .map(|requirement| {
            resolve_trigger_target_chooser(game, trigger, entry, requirement.chooser.as_ref())
        })
        .collect::<Option<Vec<_>>>()?;
    let first_chooser = *choosers.first()?;
    if choosers.iter().all(|chooser| *chooser == first_chooser) {
        return choose_trigger_targets_with_one_chooser(
            game,
            trigger,
            first_chooser,
            requirements,
            decision_maker,
        );
    }

    // Multiple players can be assigned distinct target decisions within one
    // trigger. Prompt each requirement's chooser in authored order, retaining
    // the existing distinct-player constraint across those prompts.
    let mut selected_targets = Vec::new();
    let mut assignments = Vec::with_capacity(requirements.len());
    let mut selected_distinct_players = std::collections::HashMap::<usize, Vec<PlayerId>>::new();
    for (requirement, chooser) in requirements.iter().zip(choosers) {
        let mut requirement_ctx =
            trigger_target_requirement_contexts(std::slice::from_ref(requirement));
        let context = requirement_ctx.first_mut()?;
        if !game.source_snapshot_is_exempt_from_range(
            Some(trigger.source),
            trigger.source_snapshot.as_ref(),
        ) {
            context.legal_targets.retain(|target| match target {
                Target::Player(player) => game.player_is_within_range(chooser, *player),
                Target::Object(object) => {
                    game.object_is_within_range(chooser, *object, Some(trigger.source))
                }
            });
            context.legal_target_sets.retain(|set| {
                set.iter()
                    .all(|target| context.legal_targets.contains(target))
            });
        }
        if context.legal_targets.len() < context.min_targets {
            return None;
        }
        if let Some(group) = requirement.distinct_player_group
            && let Some(already_selected) = selected_distinct_players.get(&group)
        {
            context.legal_targets.retain(|target| {
                !matches!(target, Target::Player(player) if already_selected.contains(player))
            });
            context.legal_target_sets.retain(|set| {
                set.iter().all(|target| {
                    !matches!(target, Target::Player(player) if already_selected.contains(player))
                })
            });
        }

        let ctx = crate::decisions::context::TargetsContext::new(
            chooser,
            trigger.source,
            format!("{}'s triggered ability", trigger.source_name),
            requirement_ctx.clone(),
        );
        let proposed = decision_maker.decide_targets(game, &ctx);
        if decision_maker.awaiting_choice() {
            return None;
        }
        let chosen =
            crate::targeting::normalize_targets_for_requirements(&requirement_ctx, proposed)?;
        let start = selected_targets.len();
        if let Some(group) = requirement.distinct_player_group {
            selected_distinct_players
                .entry(group)
                .or_default()
                .extend(chosen.iter().filter_map(|target| match target {
                    Target::Player(player) => Some(*player),
                    Target::Object(_) => None,
                }));
        }
        selected_targets.extend(chosen);
        assignments.push(crate::game_state::TargetAssignment {
            spec: requirement.spec.clone(),
            range: start..selected_targets.len(),
        });
    }

    Some((selected_targets, assignments))
}

/// Create a stack entry for a triggered ability, handling target selection.
///
/// Returns None if the trigger has mandatory targets but no legal targets exist.
pub(super) fn create_triggered_stack_entry_with_targets(
    game: &mut GameState,
    trigger: &TriggeredAbilityEntry,
    decision_maker: &mut dyn DecisionMaker,
    _trigger_queue: &mut TriggerQueue,
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

    let explicit_requirements = target_requirements_from_explicit_choices(game, trigger, &entry);
    let mut program_requirements = extract_target_requirements_from_program_with_modes(
        game,
        &trigger.ability.effects,
        trigger.controller,
        Some(trigger.source),
        entry.chosen_modes.as_deref(),
    );
    refresh_trigger_program_target_requirements(game, trigger, &entry, &mut program_requirements);

    let requirements = if !program_requirements.is_empty()
        && target_requirements_cover_existing(&program_requirements, &explicit_requirements)
    {
        program_requirements
    } else {
        let explicit_requirement_count = explicit_requirements.len();
        let mut requirements = explicit_requirements;
        for requirement in program_requirements {
            if !target_requirement_reuses_existing(
                &requirement,
                &requirements[..explicit_requirement_count],
            ) {
                requirements.push(requirement);
            }
        }
        requirements
    };

    let (chosen_targets, target_assignments) =
        choose_trigger_targets(game, trigger, &entry, &requirements, decision_maker)?;
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
    let source_snapshot = trigger
        .source_snapshot
        .clone()
        .or_else(|| {
            game.object(trigger.source).map(|obj| {
                ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                    obj, game, effects,
                )
            })
        })
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
    add_triggering_object_tag(game, &trigger.triggering_event, &mut entry.tagged_objects);
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
            AttackEventTarget::Battle(battle_id) => {
                if let Some(protector) = game.battle_protector(battle_id) {
                    entry = entry.with_defending_player(protector);
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
            AttackEventTarget::Battle(battle_id) => {
                if let Some(protector) = game.battle_protector(battle_id) {
                    entry = entry.with_defending_player(protector);
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
            AttackEventTarget::Battle(battle_id) => {
                if let Some(protector) = game.battle_protector(battle_id) {
                    entry = entry.with_defending_player(protector);
                }
            }
        }
    }
    if trigger.triggering_event.kind() == EventKind::Damage
        && let Some(damage) = trigger
            .triggering_event
            .downcast::<crate::events::DamageEvent>()
        && damage.is_combat
        && let Some(defending_player) = combat_damage_defending_player(game, damage)
    {
        entry = entry.with_defending_player(defending_player);
    }

    if trigger.ability.trigger.saga_chapters().is_some() {
        entry = entry.with_chapter_ability_source(trigger.source);
    }
    if crate::triggers::check::is_intrinsic_siege_defeat_trigger(trigger) {
        entry = entry.with_battle_defeat_source(trigger.source);
    }

    entry
}

fn combat_damage_defending_player(
    game: &GameState,
    damage: &crate::events::DamageEvent,
) -> Option<PlayerId> {
    match damage.target {
        crate::events::DamageTarget::Player(player) => Some(player),
        crate::events::DamageTarget::Object(_) => {
            let combat = game.combat.as_ref()?;
            let attack_target = get_attack_target(combat, damage.source)?;
            match attack_target {
                AttackTarget::Player(player) => Some(*player),
                AttackTarget::Planeswalker(planeswalker) => game
                    .object(*planeswalker)
                    .map(|object| game.controller_of(object)),
                AttackTarget::Battle(battle) => game.battle_protector(*battle),
            }
        }
    }
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

    struct ChooseRequiredObjectTarget {
        target: ObjectId,
        target_prompts: usize,
    }

    impl DecisionMaker for ChooseRequiredObjectTarget {
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
                "triggering-object-tagged target should be legal; got {:?}",
                ctx.requirements
            );
            vec![Target::Object(self.target)]
        }
    }

    #[derive(Default)]
    struct PausingSectorDecisionMaker {
        response: Option<usize>,
        awaiting: bool,
        prompted_player: Option<PlayerId>,
    }

    impl PausingSectorDecisionMaker {
        fn respond(&mut self, option: usize) {
            self.response = Some(option);
            self.awaiting = false;
        }
    }

    impl DecisionMaker for PausingSectorDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(response) = self.response.take() {
                return vec![response];
            }
            self.prompted_player = Some(ctx.player);
            self.awaiting = true;
            Vec::new()
        }

        fn awaiting_choice(&self) -> bool {
            self.awaiting
        }
    }

    #[test]
    fn u036_priority_driver_resumes_sector_batch_without_partial_commit() {
        use crate::ability::Ability;
        use crate::card::{CardBuilder, PowerToughness};
        use crate::cards::CardDefinitionBuilder;
        use crate::ids::CardId;
        use crate::marker::SectorDesignation::{Beta, Gamma};
        use crate::static_abilities::StaticAbility;
        use crate::types::CardType;
        use crate::zone::Zone;

        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let sculptor = CardDefinitionBuilder::new(CardId::new(), "Space Sculptor")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::space_sculptor()))
            .build();
        game.create_object_from_definition(&sculptor, alice, Zone::Battlefield);
        let creature = |name: &str| {
            CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build()
        };
        let alice_creature =
            game.create_object_from_card(&creature("Alice Creature"), alice, Zone::Battlefield);
        let bob_creature =
            game.create_object_from_card(&creature("Bob Creature"), bob, Zone::Battlefield);
        let mut queue = TriggerQueue::new();
        let mut dm = PausingSectorDecisionMaker::default();

        check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).expect("first prompt");
        assert_eq!(dm.prompted_player, Some(bob));
        assert_eq!(game.sector_designation(alice_creature), None);
        assert_eq!(game.sector_designation(bob_creature), None);

        dm.respond(1);
        check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).expect("second prompt");
        assert_eq!(dm.prompted_player, Some(alice));
        assert_eq!(game.sector_designation(alice_creature), None);
        assert_eq!(game.sector_designation(bob_creature), None);

        dm.respond(2);
        check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).expect("atomic commit");
        assert_eq!(game.sector_designation(bob_creature), Some(Beta));
        assert_eq!(game.sector_designation(alice_creature), Some(Gamma));
    }

    struct PendingLegendChoiceDm {
        object_prompts: usize,
    }

    impl DecisionMaker for PendingLegendChoiceDm {
        fn awaiting_choice(&self) -> bool {
            true
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.object_prompts += 1;
            Vec::new()
        }
    }

    #[test]
    fn legend_rule_choice_is_not_committed_while_awaiting_decision() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let legend = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_200),
            "Pending Legend",
        )
        .supertypes(vec![crate::types::Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
        let first = game.create_object_from_definition(&legend, alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&legend, alice, Zone::Battlefield);

        let mut trigger_queue = TriggerQueue::new();
        let mut dm = PendingLegendChoiceDm { object_prompts: 0 };
        check_and_apply_sbas_with(&mut game, &mut trigger_queue, &mut dm)
            .expect("SBA check should surface the legend prompt without failing");

        assert_eq!(dm.object_prompts, 1);
        for id in [first, second] {
            assert_eq!(
                game.object(id).map(|object| object.zone),
                Some(Zone::Battlefield),
                "no legend may be killed by the fallback while the choice is pending"
            );
        }
    }

    #[test]
    fn trigger_program_targets_keep_triggering_object_tag_context() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let lesser_creature = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_102),
            "Lesser Creature",
        )
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Green,
        ]]))
        .card_types(vec![CardType::Creature])
        .build();
        let lesser_id =
            game.create_object_from_definition(&lesser_creature, alice, Zone::Battlefield);

        let target_filter = crate::target::ObjectFilter::creature()
            .controlled_by(crate::target::PlayerFilter::You)
            .match_tagged(
                crate::tag::TagKey::from("triggering"),
                crate::target::TaggedOpbjectRelation::ManaValueLtTagged,
            );
        let target_spec = ChooseSpec::target(ChooseSpec::Object(target_filter))
            .with_count(crate::effect::ChoiceCount::up_to(1));
        let mut ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::enters_battlefield(
                crate::target::ObjectFilter::creature()
                    .controlled_by(crate::target::PlayerFilter::You),
                None,
            ),
            vec![
                Effect::tag_triggering_object("triggering"),
                Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                    target_spec.clone(),
                )),
            ],
        );
        if let crate::ability::AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.choices = vec![target_spec];
        }

        let source = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_103),
            "Contextual Trigger Source",
        )
        .mana_cost(crate::mana::ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Green],
            vec![crate::mana::ManaSymbol::Green],
            vec![crate::mana::ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .with_ability(ability)
        .build();
        let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source_id)
                .expect("trigger source should exist on battlefield"),
            &game,
        );
        let event = TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                source_id,
                Zone::Hand,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(source_snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &event) {
            trigger_queue.add(trigger);
        }
        assert_eq!(trigger_queue.entries.len(), 1);

        let mut dm = ChooseRequiredObjectTarget {
            target: lesser_id,
            target_prompts: 0,
        };
        put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
            .expect("trigger with contextual target should stack");

        assert_eq!(dm.target_prompts, 1);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![Target::Object(lesser_id)]);
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

    #[test]
    fn multiplayer_800_4d_discards_queued_trigger_controlled_by_player_who_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let watcher = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_104),
            "Departed Trigger Controller",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::player_loses_game(crate::target::PlayerFilter::Opponent),
            vec![Effect::gain_life(1)],
        ))
        .build();
        game.create_object_from_definition(&watcher, alice, Zone::Battlefield);
        assert!(game.mark_player_lost(bob));
        let event = game
            .take_pending_trigger_events()
            .into_iter()
            .find(|event| event.kind() == crate::events::EventKind::PlayerLosesGame)
            .expect("Bob's loss should produce an event");
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &event) {
            trigger_queue.add(trigger);
        }
        assert_eq!(trigger_queue.entries.len(), 1);
        assert!(game.leave_game(alice));
        let mut dm = crate::decision::AutoPassDecisionMaker;

        put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
            .expect("discarding an ineligible trigger should succeed");

        assert!(trigger_queue.is_empty());
        assert!(game.stack.is_empty());
    }

    #[test]
    fn ability_triggered_event_uses_second_stacking_pass_without_self_recursion() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let ordinary = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_113),
            "Ordinary Upkeep Trigger",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
            vec![Effect::gain_life(1)],
        ))
        .build();
        game.create_object_from_definition(&ordinary, alice, Zone::Battlefield);

        let watcher = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_114),
            "Ability Trigger Watcher",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::another_ability_triggers(),
            vec![Effect::gain_life(2)],
        ))
        .build();
        game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

        let event = TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &event) {
            trigger_queue.add(trigger);
        }
        assert_eq!(trigger_queue.entries.len(), 1);

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("both stacking passes should complete");

        let names = game
            .stack
            .iter()
            .map(|entry| entry.source_name.as_deref().unwrap_or("?"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["Ordinary Upkeep Trigger", "Ability Trigger Watcher"],
            "ordinary triggers stack in the first pass and trigger-on-trigger abilities in the second"
        );
        assert!(trigger_queue.is_empty());
        assert!(
            !trigger_queue.has_ability_triggered_events(),
            "the watcher's own trigger must not recursively trigger itself"
        );
    }

    #[test]
    fn ability_triggered_second_pass_follows_complete_first_pass_apnap_order() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;

        for (card_id, name, controller) in [
            (91_115, "Alice Ordinary", alice),
            (91_116, "Bob Ordinary", bob),
        ] {
            let ordinary = crate::cards::CardDefinitionBuilder::new(
                crate::ids::CardId::from_raw(card_id),
                name,
            )
            .card_types(vec![CardType::Enchantment])
            .with_ability(crate::ability::Ability::triggered(
                crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::Any),
                vec![Effect::gain_life(1)],
            ))
            .build();
            game.create_object_from_definition(&ordinary, controller, Zone::Battlefield);
        }

        let watcher = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_117),
            "Alice Ability Watcher",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::another_ability_triggers(),
            vec![Effect::gain_life(2)],
        ))
        .build();
        game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

        let event = TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &event) {
            trigger_queue.add(trigger);
        }

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("two-pass APNAP stacking should complete");

        let names = game
            .stack
            .iter()
            .map(|entry| entry.source_name.as_deref().unwrap_or("?"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "Alice Ordinary",
                "Bob Ordinary",
                "Alice Ability Watcher",
                "Alice Ability Watcher",
            ],
            "both controllers' ordinary triggers must stack before the active player's trigger-on-trigger entries"
        );
    }

    #[test]
    fn ability_triggered_events_respect_trigger_frequency_before_second_pass() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let mut limited_ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::beginning_of_upkeep(crate::target::PlayerFilter::You),
            vec![Effect::gain_life(1)],
        );
        let crate::ability::AbilityKind::Triggered(triggered) = &mut limited_ability.kind else {
            unreachable!();
        };
        triggered.intervening_if = Some(crate::ConditionExpr::MaxTimesEachTurn(1));

        let ordinary = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_118),
            "Limited Ordinary Trigger",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(limited_ability)
        .build();
        game.create_object_from_definition(&ordinary, alice, Zone::Battlefield);

        let watcher = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::from_raw(91_119),
            "Limited Ability Watcher",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::triggered(
            crate::triggers::Trigger::another_ability_triggers(),
            vec![Effect::gain_life(2)],
        ))
        .build();
        game.create_object_from_definition(&watcher, alice, Zone::Battlefield);

        let event = TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let mut trigger_queue = TriggerQueue::new();
        for _ in 0..2 {
            for trigger in crate::triggers::check_triggers(&game, &event) {
                trigger_queue.add(trigger);
            }
        }

        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("frequency-limited two-pass stacking should complete");
        let names = game
            .stack
            .iter()
            .map(|entry| entry.source_name.as_deref().unwrap_or("?"))
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["Limited Ordinary Trigger", "Limited Ability Watcher"],
            "a suppressed extra occurrence must not emit a spurious AbilityTriggered event"
        );
    }
}
