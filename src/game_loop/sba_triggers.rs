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
        check_state_based_actions_with_effects, legend_rule_specs_from_actions,
    };

    // Refresh continuous state (static ability effects and "can't" effect tracking)
    // before checking SBAs. This ensures the layer system is up to date.
    game.refresh_continuous_state();

    loop {
        let all_effects = game.all_continuous_effects();
        let actions = check_state_based_actions_with_effects(game, &all_effects);
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
            let post_legend_effects = game.all_continuous_effects();
            let post_legend_actions =
                check_state_based_actions_with_effects(game, &post_legend_effects);
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

    // Group triggers by controller (APNAP order)
    let active_player = game.turn.active_player;
    let mut active_triggers = Vec::new();
    let mut other_triggers = Vec::new();

    for trigger in trigger_queue.take_all() {
        if trigger.controller == active_player {
            active_triggers.push(trigger);
        } else {
            other_triggers.push(trigger);
        }
    }

    // Active player's triggers go on stack first (resolve last)
    for trigger in active_triggers {
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

    // Then other players' triggers (in turn order)
    for trigger in other_triggers {
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

    Ok(())
}

fn is_triggered_mana_ability(game: &GameState, trigger: &TriggeredAbilityEntry) -> bool {
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

fn effects_could_add_mana(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    effects: &[crate::effect::Effect],
) -> bool {
    effects
        .iter()
        .any(|effect| effect_could_add_mana(game, source, controller, effect))
}

fn effect_could_add_mana(
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

fn resolve_triggered_stack_entry_immediately(
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

    let (valid_targets, all_targets_invalid) = validate_stack_entry_targets(game, &entry);
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

    ctx = ctx.with_targets(valid_targets);
    ctx.snapshot_targets(game);

    let effects = if let Some(ref ability_effects) = entry.ability_effects {
        ability_effects.clone()
    } else if let Some(obj) = game.object(entry.object_id) {
        get_effects_for_stack_entry(game, &entry, obj)
    } else {
        Vec::new()
    };

    let mut all_events = Vec::new();
    for effect in &effects {
        if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
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

fn resolve_triggered_mana_abilities_with_dm(
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

fn can_stack_trigger_this_turn(game: &GameState, trigger: &TriggeredAbilityEntry) -> bool {
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
fn create_triggered_stack_entry_with_targets(
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
    for target_spec in &trigger.ability.choices {
        // Compute legal targets for this spec
        let legal_targets = compute_legal_targets_with_tagged_objects(
            game,
            target_spec,
            trigger.controller,
            Some(trigger.source),
            tagged_objects_ref,
        );

        if legal_targets.is_empty() {
            // No legal targets - trigger can't go on stack
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
                min_targets: 1,
                max_targets: Some(1),
            }],
        );

        // Get the choice from the decision maker
        let targets = decision_maker.decide_targets(game, &ctx);

        if let Some(first_target) = targets.first() {
            chosen_targets.push(*first_target);
        } else {
            // No target chosen - use the first legal target as default
            if let Some(first_legal) = legal_targets.first() {
                chosen_targets.push(*first_legal);
            } else {
                return None;
            }
        }
    }

    // Add the chosen targets to the stack entry
    entry.targets = chosen_targets;

    Some(entry)
}

/// Convert a triggered ability entry to a stack entry.
fn triggered_to_stack_entry(game: &GameState, trigger: &TriggeredAbilityEntry) -> StackEntry {
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
