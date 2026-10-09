use super::*;
use crate::filter::PlayerFilterExt;
use crate::target::PlayerFilter;
use std::collections::HashSet;

fn resolve_modal_count_value_for_source(
    game: &GameState,
    source_id: Option<ObjectId>,
    value: &crate::effect::Value,
    fallback: usize,
) -> usize {
    let x_value = source_id
        .and_then(|id| game.object(id))
        .and_then(|obj| obj.x_value)
        .and_then(|x| usize::try_from(x).ok());

    match value {
        crate::effect::Value::Fixed(n) => (*n).max(0) as usize,
        crate::effect::Value::X => x_value.unwrap_or(fallback),
        crate::effect::Value::XTimes(multiplier) => x_value
            .map(|x| ((x as i32) * *multiplier).max(0) as usize)
            .unwrap_or(fallback),
        _ => fallback,
    }
}

// ============================================================================
// Target Extraction
// ============================================================================

/// Check if a ChooseSpec requires player selection.
/// Check if a target spec requires the player to select a target.
fn object_filter_is_tagged_reference(filter: &crate::filter::ObjectFilter) -> bool {
    !filter.tagged_constraints.is_empty()
        && filter.tagged_constraints.iter().all(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::SameObjectId
            )
        })
}

pub fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        // Explicit target wrappers always require cast/activation-time selection.
        ChooseSpec::Target(_) => true,
        ChooseSpec::SurfaceHinted { spec: inner, .. }
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => requires_target_selection(inner),
        // These require target selection during casting
        ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::Player(_) => true,
        ChooseSpec::Object(filter) => !object_filter_is_tagged_reference(filter),
        ChooseSpec::AttackedPlayerOrPlaneswalker => false,
        // These don't require selection - they're resolved at execution time
        _ => false,
    }
}

/// Queue trigger matches for all triggered abilities that see this event.
pub(super) fn queue_triggers_for_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
) {
    let event = game.ensure_trigger_event_provenance(event);
    // Zone changes and turn-history updates invalidate characteristics. Build
    // the shared cache before trigger matching creates immutable derived views.
    game.refresh_continuous_state();
    let triggers = check_triggers(game, &event);
    for trigger in triggers {
        if crate::triggers::check::is_speed_rule_trigger(&trigger) {
            game.mark_speed_increase_triggered_this_turn(trigger.controller);
        }
        trigger_queue.add(trigger);
    }
}

/// Ingest an event into trigger system with optional delayed-trigger checks.
/// Spell casts always notify delayed triggers, including casts made during
/// another effect's resolution and casts made through the priority loop.
pub(crate) fn queue_triggers_from_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
    include_delayed: bool,
) {
    if game.action_observations_suppressed() {
        return;
    }
    let mut event = game.event_with_retained_trigger_capture(&event);
    let include_delayed = include_delayed || event.kind() == crate::events::EventKind::SpellCast;
    if event.ordinary_triggers_captured() && (!include_delayed || event.delayed_triggers_captured())
    {
        return;
    }
    if let Some(targeted) = event.downcast::<BecomesTargetedEvent>() {
        event = event.with_inner_event(targeted.clone().with_participant_snapshots(game));
    }
    game.record_turn_history_event(&event);
    if !event.ordinary_triggers_captured() {
        queue_triggers_for_event(game, trigger_queue, event.clone());
        if game.token_resource_failure().is_some() {
            return;
        }
        event.mark_ordinary_triggers_captured();
    }
    if include_delayed && !event.delayed_triggers_captured() {
        for trigger in crate::triggers::check_delayed_triggers(game, &event) {
            trigger_queue.add(trigger);
        }
        if game.token_resource_failure().is_some() {
            return;
        }
        event.mark_delayed_triggers_captured();
    }
    game.retain_trigger_capture_receipt(&event);
}

/// Capture the observers of one completed cast before its publishing effect
/// can run another instruction. A cast is its own completed transaction even
/// inside a held outer program; this does not drain or regroup other events.
/// The returned receipt prevents later reported/queued publication from
/// matching the same occurrence again. Intervening-if resolution checks remain
/// on the captured ability and run under their ordinary resolution owner.
pub(crate) fn capture_completed_spell_cast(
    game: &mut GameState,
    spell: ObjectId,
    caster: PlayerId,
    from_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
) -> Result<(TriggerEvent, TriggerQueue), crate::effects::ExecutionError> {
    use crate::effects::ExecutionError;
    let (root, meter) = game.begin_token_resource_scope();
    let checkpoint = game.clone();
    let result = (|| {
        game.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let cast = SpellCastEvent::try_from_completed_cast(spell, caster, from_zone, game)?;
        cast.required_completed_snapshot()?;
        if cast.targets.is_none() {
            return Err(ExecutionError::IncompleteEvidence(
                "completed cast publication requires its chosen target receipt".into(),
            ));
        }
        let mut event = game
            .ensure_trigger_event_provenance(TriggerEvent::new_with_provenance(cast, provenance));
        let mut captured = TriggerQueue::new();
        queue_triggers_from_event(game, &mut captured, event.clone(), true);
        if let Some(error) = game.token_resource_failure() {
            return Err(error);
        }
        event.mark_triggers_captured();
        Ok((event, captured))
    })();
    if let Err(error) = &result {
        game.record_token_resource_failure(error);
        game.restore_execution_checkpoint(checkpoint, false);
    }
    game.end_token_resource_scope(root, &meter);
    result
}

/// Queue trigger matches for events one instruction reported, in order.
///
/// Counters one instruction put on several objects form one simultaneous
/// event (CR 603.2c): consecutive counter events sharing a simultaneous batch
/// are matched together, so a "one or more ... on one or more ..." trigger
/// fires once for the whole placement. Dice rolled by one instruction are
/// grouped the same way for "whenever you roll one or more dice".
pub(crate) fn try_queue_triggers_from_reported_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
    include_delayed: bool,
) -> Result<(), crate::effects::ExecutionError> {
    try_queue_reported_events_with_batch_policy(game, trigger_queue, events, include_delayed, false)
}

fn try_queue_reported_events_with_batch_policy(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
    include_delayed: bool,
    counter_batches_only: bool,
) -> Result<(), crate::effects::ExecutionError> {
    let (root, meter) = game.begin_token_resource_scope();
    let checkpoint = game.clone();
    let queue_checkpoint = trigger_queue.clone();
    queue_reported_events_with_batch_policy(
        game,
        trigger_queue,
        events,
        include_delayed,
        counter_batches_only,
    );
    let result = game.token_resource_failure().map_or(Ok(()), Err);
    if result.is_err() {
        game.restore_execution_checkpoint(checkpoint, false);
        *trigger_queue = queue_checkpoint;
    }
    game.end_token_resource_scope(root, &meter);
    result
}

/// Legacy matching adapter. Counter-capable production callers use the
/// checked adapter above or an enclosing checked capture scope.
pub(crate) fn queue_triggers_from_reported_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
    include_delayed: bool,
) {
    queue_reported_events_with_batch_policy(game, trigger_queue, events, include_delayed, false);
}

fn queue_reported_events_with_batch_policy(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
    include_delayed: bool,
    counter_batches_only: bool,
) {
    if game.action_observations_suppressed() {
        return;
    }
    let groups_by_batch = |event: &TriggerEvent| {
        if counter_batches_only && event.kind() != crate::events::EventKind::MarkersChanged {
            None
        } else {
            event.simultaneous_batch()
        }
    };
    let mut seen = HashSet::new();
    let mut events = events
        .into_iter()
        .map(|event| game.event_with_retained_trigger_capture(&event))
        .filter(|event| !event.triggers_captured())
        // Pending ownership keeps canonical simultaneous metadata and siblings.
        .filter(|event| !game.event_is_pending_for_trigger_matching(event))
        .filter(|event| seen.insert(event.occurrence_key()))
        .map(Some)
        .collect::<Vec<_>>();
    for index in 0..events.len() {
        let Some(event) = events[index].take() else {
            continue;
        };
        if let Some(batch) = groups_by_batch(&event)
            && events[index + 1..]
                .iter()
                .flatten()
                .any(|next| groups_by_batch(next) == Some(batch))
        {
            // Damage one instruction deals to several recipients (or from
            // several sources) is one event too (CR 603.2c, 120.3).
            let mut simultaneous = vec![event];
            for later in events.iter_mut().skip(index + 1) {
                if later
                    .as_ref()
                    .is_some_and(|next| groups_by_batch(next) == Some(batch))
                {
                    simultaneous.extend(later.take());
                }
            }
            crate::events::other::bind_die_roll_batch_results(&mut simultaneous);
            crate::events::damage::bind_received_damage_amounts(&mut simultaneous);
            queue_triggers_for_simultaneous_events(game, trigger_queue, simultaneous.clone());
            if include_delayed {
                queue_delayed_triggers_for_simultaneous_events(game, trigger_queue, &simultaneous);
            }
            continue;
        }
        queue_triggers_from_event(game, trigger_queue, event, include_delayed);
    }
}

/// Keep legacy per-event behavior for other families while retaining counter
/// producers' authored batch boundaries. Never regroup separate instructions.
pub(super) fn queue_triggers_for_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
) -> Result<(), crate::effects::ExecutionError> {
    try_queue_reported_events_with_batch_policy(game, trigger_queue, events, false, true)
}

/// Like [`queue_triggers_for_events`], but delayed triggers observe the
/// events too. A mana ability's own production event is observed by
/// temporary "until end of turn, whenever a player taps ... for mana"
/// triggers (Bubbling Muck, Chaos Moon) exactly like printed ones; those are
/// triggered mana abilities and resolve immediately (CR 605.1b).
pub(super) fn queue_triggers_for_events_including_delayed(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
) -> Result<(), crate::effects::ExecutionError> {
    try_queue_reported_events_with_batch_policy(game, trigger_queue, events, true, true)
}

/// Queue trigger matches for events produced by one simultaneous game action.
///
/// Every event is recorded before matching, then trigger checks share a single
/// derived view and registry for the stable post-action state.
pub(super) fn queue_triggers_for_simultaneous_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
) {
    if game.action_observations_suppressed() {
        return;
    }
    let mut seen = HashSet::new();
    let events = events
        .into_iter()
        .map(|event| game.event_with_retained_trigger_capture(&event))
        .filter(|event| !event.ordinary_triggers_captured())
        .filter(|event| seen.insert(event.occurrence_key()))
        .collect::<Vec<_>>();
    let mut events = events
        .into_iter()
        .map(|event| game.ensure_trigger_event_provenance(event))
        .collect::<Vec<_>>();
    if events.is_empty() {
        return;
    }
    crate::events::other::bind_die_roll_batch_results(&mut events);
    crate::events::damage::bind_received_damage_amounts(&mut events);
    let previous_batch_start = game.turn_store.turn_history.begin_simultaneous_batch();
    for event in &events {
        game.record_turn_history_event(event);
    }

    game.refresh_continuous_state();
    let trigger_groups = check_triggers_batch(game, &events);
    let trigger_groups =
        match crate::triggers::counters::coalesce_counter_recipient_groups(trigger_groups) {
            Ok(groups) => groups,
            Err(error) => {
                // Typed capture/drain adapters own rollback. No entry from this
                // simultaneous action has reached the caller's queue yet.
                game.record_token_resource_failure(&error);
                game.turn_store
                    .turn_history
                    .end_simultaneous_batch(previous_batch_start);
                return;
            }
        };
    let mut speed_controllers = std::collections::HashSet::new();
    let mut simultaneous_groups_seen = HashSet::new();
    let mut zone_groups = std::collections::HashMap::new();
    // Queue indices of per-source / per-recipient damage groups, so later
    // assignments in the same action add their amount ("that much damage").
    let mut damage_groups: std::collections::HashMap<_, Vec<usize>> =
        std::collections::HashMap::new();
    for triggers in trigger_groups {
        let mut damage_groups_from_this_event = Vec::new();
        let mut zone_occurrences = std::collections::HashMap::new();
        // Delay inserting keys until this event's complete group is handled.
        // That preserves multiple identical ability instances on one object,
        // while suppressing their duplicate matches on later assignments in
        // the same simultaneous action.
        let mut groups_from_this_event = Vec::new();
        for trigger in triggers {
            if let Some(group) =
                trigger
                    .ability
                    .trigger
                    .simultaneous_trigger_key(&trigger.triggering_event)
                    // Counter recipients were already checked and coalesced by
                    // instance. Generic seen-key suppression must not discard an
                    // additional identical instance first matching a later receipt.
                    .filter(|group| {
                        !matches!(group,
                    crate::triggers::matcher_trait::SimultaneousTriggerKey::CounterRecipient { .. })
                    })
            {
                let key = (trigger.source_stable_id, trigger.trigger_identity, group);
                if matches!(group,
                    crate::triggers::matcher_trait::SimultaneousTriggerKey::ZoneChangeBatch
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::MillingBatch
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::PlayerMillingBatch(_)
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::ObjectLeavesGameBatch
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::PhasingBatch { .. }
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::TapStateBatch { .. }
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::PlayerTapStateBatch { .. })
                {
                    // Identical ability instances remain separate; match each
                    // occurrence to its corresponding entry from earlier events.
                    let occurrence = zone_occurrences.entry(key).or_insert(0usize);
                    let instance_key = (key, *occurrence);
                    *occurrence += 1;
                    if let Some(&index) = zone_groups.get(&instance_key) {
                        let previous: &mut crate::triggers::TriggeredAbilityEntry =
                            &mut trigger_queue.entries[index];
                        if let Some(amount) = trigger.event_value_amount {
                            previous.event_value_amount =
                                Some(previous.event_value_amount.unwrap_or(0) + amount);
                        }
                        crate::triggers::merge_trigger_group_tags(
                            &mut previous.tagged_objects,
                            &trigger.tagged_objects,
                        );
                        continue;
                    }
                    zone_groups.insert(instance_key, trigger_queue.entries.len());
                } else if simultaneous_groups_seen.contains(&key) {
                    if matches!(
                        group,
                        crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageSource(_)
                            | crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageTarget(_)
                            | crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageSourceTarget(_, _)
                            | crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageSourceController(_, _)
                    ) && let Some(indices) = damage_groups.get(&key)
                    {
                        for &index in indices {
                            let previous: &mut crate::triggers::TriggeredAbilityEntry =
                                &mut trigger_queue.entries[index];
                            if let Some(amount) = trigger.event_value_amount {
                                let Some(total) = previous.event_value_amount.unwrap_or(0).checked_add(amount) else {
                                    game.record_token_resource_failure(&crate::effects::ExecutionError::ResourceLimitExceeded {
                                        resource: "grouped damage trigger amount", requested: i32::MAX as u128 + 1,
                                        maximum: i32::MAX as u128,
                                    });
                                    game.turn_store.turn_history.end_simultaneous_batch(previous_batch_start);
                                    return;
                                };
                                previous.event_value_amount = Some(total);
                            }
                            crate::triggers::merge_trigger_group_tags(
                                &mut previous.tagged_objects,
                                &trigger.tagged_objects,
                            );
                        }
                    }
                    continue;
                } else if matches!(
                    group,
                    crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageSource(_)
                        | crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageTarget(_)
                            | crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageSourceTarget(_, _)
                            | crate::triggers::matcher_trait::SimultaneousTriggerKey::DamageSourceController(_, _)
                ) {
                    damage_groups_from_this_event.push((key, trigger_queue.entries.len()));
                }
                groups_from_this_event.push(key);
            }
            if crate::triggers::check::is_speed_rule_trigger(&trigger) {
                if !speed_controllers.insert(trigger.controller) {
                    continue;
                }
                game.mark_speed_increase_triggered_this_turn(trigger.controller);
            }
            trigger_queue.add(trigger);
        }
        simultaneous_groups_seen.extend(groups_from_this_event);
        for (key, index) in damage_groups_from_this_event {
            damage_groups.entry(key).or_default().push(index);
        }
    }
    game.turn_store
        .turn_history
        .end_simultaneous_batch(previous_batch_start);
    if game.token_resource_failure().is_none() {
        for mut event in events {
            event.mark_ordinary_triggers_captured();
            game.retain_trigger_capture_receipt(&event);
        }
    }
}
/// Delayed observers share occurrence proof while retaining their caller's
/// before/after-normal matching boundary and complete simultaneous grouping.
pub(super) fn queue_delayed_triggers_for_simultaneous_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: &[TriggerEvent],
) {
    if game.action_observations_suppressed() {
        return;
    }
    let mut seen = HashSet::new();
    let events = events
        .iter()
        .map(|event| game.event_with_retained_trigger_capture(event))
        .filter(|event| !event.delayed_triggers_captured())
        .filter(|event| seen.insert(event.occurrence_key()))
        .collect::<Vec<_>>();
    if events.is_empty() {
        return;
    }
    for trigger in crate::triggers::check_delayed_triggers_for_simultaneous_events(game, &events) {
        trigger_queue.add(trigger);
    }
    if game.token_resource_failure().is_none() {
        for mut event in events {
            event.mark_delayed_triggers_captured();
            game.retain_trigger_capture_receipt(&event);
        }
    }
}

pub(super) fn target_events_from_targets(
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    stack_ability: Option<ObjectId>,
    provenance: ProvNodeId,
) -> Vec<TriggerEvent> {
    // An object or player chosen for several instances of "target" becomes
    // the target of the spell or ability once (CR 115.3, 601.2c), so emit one
    // event per distinct target, in first-chosen order.
    let mut seen = Vec::with_capacity(targets.len());
    targets
        .iter()
        .filter(|target| {
            if seen.contains(*target) {
                return false;
            }
            seen.push(**target);
            true
        })
        .map(|target| {
            TriggerEvent::new_with_provenance(
                BecomesTargetedEvent::new_target(*target, source, source_controller, by_ability)
                    .with_stack_ability(stack_ability),
                provenance,
            )
        })
        .collect()
}

pub(super) fn is_crime_target(game: &GameState, committer: PlayerId, target: &Target) -> bool {
    let opponent = |player| {
        game.player(player)
            .is_some_and(|player| player.is_in_game())
            && game.are_opponents(committer, player)
    };
    match target {
        Target::Player(player) => opponent(*player),
        Target::Object(object_id) => {
            let Some(obj) = game.object(*object_id) else {
                // A spell or ability an opponent controls (CR 700.13).
                return game
                    .stack_ability_entry(*object_id)
                    .is_some_and(|entry| opponent(entry.controller));
            };
            if obj.zone == Zone::Graveyard {
                opponent(obj.owner)
            } else if matches!(obj.zone, Zone::Battlefield | Zone::Stack) {
                game.current_controller(*object_id).is_some_and(opponent)
            } else {
                false
            }
        }
    }
}

pub(super) fn targets_commit_crime(
    game: &GameState,
    committer: PlayerId,
    targets: &[Target],
) -> bool {
    targets
        .iter()
        .any(|target| is_crime_target(game, committer, target))
}

#[cfg(test)]
mod crime_scope_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;

    #[test]
    fn crimes_use_live_opponents_and_only_the_authored_rule_zones() {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        let [a, b, c, d] = [0, 1, 2, 3].map(PlayerId::from_index);
        game.set_teams(vec![vec![a, b], vec![c, d]]).unwrap();
        for (player, expected) in [(a, false), (b, false), (c, true), (d, true)] {
            assert_eq!(is_crime_target(&game, a, &Target::Player(player)), expected);
            for zone in [Zone::Battlefield, Zone::Stack, Zone::Graveyard,
                Zone::Hand, Zone::Library, Zone::Exile, Zone::Command]
            {
                let card = CardBuilder::new(CardId::new(), "Crime target")
                    .card_types(vec![CardType::Artifact]).build();
                let object = game.create_object_from_card(&card, player, zone);
                assert_eq!(is_crime_target(&game, a, &Target::Object(object)),
                    expected && matches!(zone, Zone::Battlefield | Zone::Stack | Zone::Graveyard),
                    "player={player:?}, zone={zone:?}");
            }
        }
        game.player_mut(c).unwrap().has_left_game = true;
        assert!(!is_crime_target(&game, a, &Target::Player(c)));
        assert!(!is_crime_target(&game, a, &Target::Player(PlayerId::from_index(99))));
        assert!(!is_crime_target(&game, a, &Target::Object(ObjectId::from_raw(999_999))));
    }

    #[test]
    fn crime_uses_current_permanent_control_graveyard_ownership_and_stack_ability_control() {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let [a, b, c] = [0, 1, 2].map(PlayerId::from_index);
        game.set_teams(vec![vec![a, b], vec![c]]).unwrap();
        let card = CardBuilder::new(CardId::new(), "Borrowed crime target")
            .card_types(vec![CardType::Artifact]).build();
        let object = game.create_object_from_card(&card, c, Zone::Battlefield);
        game.set_current_controller(object, b).unwrap();
        assert!(!is_crime_target(&game, a, &Target::Object(object)));
        let grave = game.move_object_by_effect(object, Zone::Graveyard).unwrap();
        assert!(is_crime_target(&game, a, &Target::Object(grave)));
        let source = game.create_object_from_card(&card, b, Zone::Battlefield);
        let ability_id = game.allocate_stack_ability_id();
        let mut entry = StackEntry::ability(source, c,
            crate::resolution::ResolutionProgram::from_effects(vec![]));
        entry.ability_id = Some(ability_id);
        game.push_to_stack(entry);
        assert!(is_crime_target(&game, a, &Target::Object(ability_id)),
            "the stack ability's controller is independent of its source");
        assert!(!is_crime_target(&game, a, &Target::Object(source)));
    }
}

fn queue_target_selection_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    provenance: ProvNodeId,
) {
    // The ability was just put on the stack: name its own stack entry.
    let stack_ability = if by_ability {
        game.stack
            .iter()
            .rev()
            .find(|entry| entry.is_ability && entry.object_id == source)
            .map(|entry| entry.target_id())
    } else {
        None
    };
    // "Whenever you and/or at least one permanent you control becomes the
    // target of a spell or ability" (Leyline of Combustion) triggers once per
    // spell or ability, however many of the matching things it targets.
    let mut targeting_batches_seen = HashSet::new();
    for mut event in target_events_from_targets(
        targets,
        source,
        source_controller,
        by_ability,
        stack_ability,
        provenance,
    ) {
        let event_provenance = game.alloc_child_event_provenance(provenance, event.kind());
        event.set_provenance(event_provenance);
        let mut candidates = TriggerQueue::new();
        queue_triggers_from_event(game, &mut candidates, event, true);
        for candidate in candidates.entries {
            if candidate
                .ability
                .trigger
                .simultaneous_trigger_key(&candidate.triggering_event)
                == Some(crate::triggers::matcher_trait::SimultaneousTriggerKey::TargetingBatch)
                && !targeting_batches_seen
                    .insert((candidate.source_stable_id, candidate.trigger_identity))
            {
                continue;
            }
            trigger_queue.add(candidate);
        }
    }
}

pub(super) fn queue_targeting_crime(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    provenance: ProvNodeId,
) {
    if !targets.is_empty() && targets_commit_crime(game, source_controller, targets) {
        let crime_event_provenance =
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::KeywordAction);
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::CommitCrime,
                    source_controller,
                    source,
                    1,
                ),
                crime_event_provenance,
            ),
            true,
        );
    }
}

pub(super) fn queue_becomes_targeted_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    provenance: ProvNodeId,
) {
    queue_target_selection_events(
        game,
        trigger_queue,
        targets,
        source,
        source_controller,
        by_ability,
        provenance,
    );
    queue_targeting_crime(
        game,
        trigger_queue,
        targets,
        source,
        source_controller,
        provenance,
    );
}

/// CR 601.2c/602.2b: target transitions occur before costs. Match now against
/// the complete announced stack object, but retain the queue inside the action
/// transaction until payment succeeds. No choices/stacking are performed here.
/// The temporary entry exposes exactly the metadata already announced; costs
/// still follow the existing pending-action representation.
pub(super) fn capture_announced_targeting(
    game: &mut GameState,
    entry: StackEntry,
) -> Result<TriggerQueue, GameLoopError> {
    if entry.targets.is_empty() {
        return Ok(TriggerQueue::new());
    }
    let checkpoint = game.clone();
    let slot = game.stack.len();
    game.stack.push(entry.clone());
    game.bump_mutation_revision();
    game.mark_continuous_state_dirty();
    if let Err(error) = game.refresh_continuous_state() {
        *game = checkpoint;
        return Err(crate::effects::ExecutionError::ContinuousDiscovery(error).into());
    }
    let mut captured = TriggerQueue::new();
    queue_target_selection_events(
        game,
        &mut captured,
        &entry.targets,
        entry.object_id,
        entry.controller,
        entry.is_ability,
        entry.provenance,
    );
    let removed = game.stack.remove(slot);
    debug_assert_eq!(removed.target_id(), entry.target_id());
    game.bump_mutation_revision();
    game.mark_continuous_state_dirty();
    Ok(captured)
}

pub(super) fn queue_ability_activated_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
    source: ObjectId,
    activator: PlayerId,
    is_mana_ability: bool,
    _source_stable_id: Option<StableId>,
    activation_cost_has_tap: bool,
) -> Result<(), GameLoopError> {
    let activation_entry = game
        .stack
        .iter()
        .rev()
        .find(|entry| !is_mana_ability && entry.is_ability && entry.object_id == source);
    let snapshot = game
        .object(source)
        .map(|object| ObjectSnapshot::from_object(object, game))
        .or_else(|| game.source_departure_snapshot(source).cloned())
        .or_else(|| {
            activation_entry
                .and_then(|entry| entry.source_snapshot.as_ref())
                .filter(|snapshot| snapshot.object_id == source)
                .cloned()
        });
    let ability_index = activation_entry.and_then(|entry| entry.ability_index);
    let activated_ability = ability_index.and_then(|index| {
        activation_entry
            .and_then(|entry| entry.source_snapshot.as_ref())
            .and_then(|snapshot| snapshot.abilities.get(index))
            .cloned()
            .or_else(|| game.current_ability(source, index))
    });
    let stack_entry_provenance = activation_entry.map(|entry| entry.provenance);
    let x_value = activation_entry.and_then(|entry| entry.x_value);
    let activation_cost_has_x = activation_entry.is_some_and(|entry| entry.activation_cost_has_x);
    let mana_spent_total =
        activation_entry.map_or(0, |entry| entry.mana_spent_on_activation.total());
    let mana_sources_tag = crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
    let mana_sources_spent = game
        .stack
        .iter()
        .rev()
        .find(|entry| entry.is_ability && entry.object_id == source)
        .and_then(|entry| entry.tagged_objects.get(&mana_sources_tag))
        .cloned()
        .unwrap_or_default();
    let activation = AbilityActivatedEvent::from_effective_ability(
        source,
        activator,
        is_mana_ability,
        activated_ability,
        snapshot,
    )
    .with_activation_cost_has_x(activation_cost_has_x)
    .with_activation_cost_has_tap(activation_cost_has_tap)
    .with_x_value(x_value)
    .with_stack_entry_provenance(stack_entry_provenance)
    .with_mana_sources_spent(mana_sources_spent)
    .with_mana_spent_total(mana_spent_total);
    queue_prepared_activation_notification_with_outputs(
        game,
        trigger_queue,
        decision_maker,
        activation,
    )
    .map(|_| ())
}

/// Queue the exact prepared payload and retain actual notification and immediate
/// triggered-mana children. Scalar root facades project this same owner.
pub(super) fn queue_prepared_activation_notification_with_outputs(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
    activation: crate::events::AbilityActivatedEvent,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, GameLoopError> {
    let is_mana = activation.is_mana_ability;
    let notification = game.complete_activation_notification(activation);
    for event in &notification.outcome.events {
        queue_triggers_from_event(game, trigger_queue, event.clone(), true);
    }
    let mut outputs = vec![notification];
    if is_mana {
        match resolve_triggered_mana_abilities_with_outputs(game, trigger_queue, decision_maker) {
            Err(GameLoopError::MandatoryLoopDraw) => game.mark_mandatory_loop_draw(),
            result => outputs.extend(result?),
        }
    }
    Ok(outputs)
}

pub(super) fn activated_ability_has_tap_cost(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
) -> bool {
    game.current_ability(source, ability_index)
        .is_some_and(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => activated.has_tap_cost(),
            _ => false,
        })
}

pub(super) fn keyword_action_from_alternative_effect(
    effect: AlternativePaymentEffect,
) -> KeywordActionKind {
    match effect {
        AlternativePaymentEffect::Convoke => KeywordActionKind::Convoke,
        AlternativePaymentEffect::Improvise => KeywordActionKind::Improvise,
    }
}

pub(super) fn payment_contribution_tag(effect: AlternativePaymentEffect) -> &'static str {
    match effect {
        AlternativePaymentEffect::Convoke => "convoked_this_spell",
        AlternativePaymentEffect::Improvise => "improvised_this_spell",
    }
}

pub(super) fn record_keyword_payment_contribution(
    contributions: &mut Vec<KeywordPaymentContribution>,
    permanent_id: ObjectId,
    effect: AlternativePaymentEffect,
) {
    let contribution = KeywordPaymentContribution {
        permanent_id,
        effect,
    };
    if !contributions.contains(&contribution) {
        contributions.push(contribution);
    }
}

pub(super) fn apply_keyword_payment_tags_for_resolution(
    game: &GameState,
    entry: &StackEntry,
    ctx: &mut ExecutionContext,
) {
    for contribution in &entry.keyword_payment_contributions {
        if let Some(obj) = game.object(contribution.permanent_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object(payment_contribution_tag(contribution.effect), snapshot);
        }
    }

    for crew_id in &entry.crew_contributors {
        if let Some(obj) = game.object(*crew_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object("crewed_it_this_turn", snapshot);
        }
    }

    for saddle_id in &entry.saddle_contributors {
        if let Some(obj) = game.object(*saddle_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object("saddled_it_this_turn", snapshot);
        }
    }
}

/// Drain pending death and custom trigger events and enqueue all matches.
fn simultaneous_rule_ltb_batch_events(pending_events: &[TriggerEvent]) -> Vec<TriggerEvent> {
    use crate::events::cause::CauseType;
    use crate::events::zones::ZoneChangeEvent;

    let mut batch_events: Vec<TriggerEvent> = Vec::new();

    for event in pending_events {
        let Some(zone_change) = event.downcast::<ZoneChangeEvent>() else {
            continue;
        };
        if zone_change.from != crate::zone::Zone::Battlefield
            || zone_change.to == crate::zone::Zone::Battlefield
            || !matches!(
                zone_change.cause.cause_type,
                CauseType::StateBasedAction | CauseType::LegendRule
            )
        {
            continue;
        }

        let merge_index = batch_events.iter().position(|existing| {
            existing
                .downcast::<ZoneChangeEvent>()
                .is_some_and(|existing_zone_change| {
                    existing_zone_change.from == zone_change.from
                        && existing_zone_change.to == zone_change.to
                        && existing_zone_change.cause == zone_change.cause
                })
        });

        let Some(index) = merge_index else {
            batch_events.push(event.clone());
            continue;
        };

        let Some(mut merged_zone_change) =
            batch_events[index].downcast::<ZoneChangeEvent>().cloned()
        else {
            continue;
        };
        if merged_zone_change.snapshots.is_empty()
            && let Some(snapshot) = merged_zone_change.snapshot.clone()
        {
            merged_zone_change.snapshots.push(snapshot);
        }
        for object in &zone_change.objects {
            if !merged_zone_change.objects.contains(object) {
                merged_zone_change.objects.push(*object);
            }
        }
        for result_object in &zone_change.result_objects {
            if !merged_zone_change.result_objects.contains(result_object) {
                merged_zone_change.result_objects.push(*result_object);
            }
        }
        for snapshot in zone_change.snapshots() {
            if !merged_zone_change
                .snapshots
                .iter()
                .any(|existing| existing.stable_id == snapshot.stable_id)
            {
                merged_zone_change.snapshots.push(snapshot.clone());
            }
        }
        merged_zone_change.snapshot = merged_zone_change.snapshots.first().cloned();
        for (tag, snapshots) in &zone_change.object_tags {
            merged_zone_change
                .object_tags
                .entry(tag.clone())
                .or_default()
                .extend(snapshots.clone());
        }

        let provenance = batch_events[index].provenance();
        let mut lookback_source_snapshots =
            batch_events[index].lookback_source_snapshots().to_vec();
        for snapshot in event.lookback_source_snapshots() {
            if !lookback_source_snapshots
                .iter()
                .any(|existing| existing.stable_id == snapshot.stable_id)
            {
                lookback_source_snapshots.push(snapshot.clone());
            }
        }
        let mut merged_event = TriggerEvent::new_with_provenance(merged_zone_change, provenance);
        if let Some(source_snapshot) = batch_events[index].source_snapshot().cloned() {
            merged_event = merged_event.with_source_snapshot(source_snapshot);
        }
        merged_event = merged_event.with_lookback_source_snapshots(lookback_source_snapshots);
        batch_events[index] = merged_event;
    }

    batch_events
        .into_iter()
        .filter(|event| {
            event
                .downcast::<ZoneChangeEvent>()
                .is_some_and(|zone_change| zone_change.snapshots().len() > 1)
        })
        .collect()
}

pub fn drain_pending_trigger_events(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    // No decision channel: duration-end requests stay queued. This callback
    // cannot fail, so the type system proves the matching-only drain infallible.
    let result =
        drain_pending_trigger_events_inner::<std::convert::Infallible>(game, trigger_queue, |_| {
            Ok(false)
        });
    match result {
        Ok(()) => {}
        Err(never) => match never {},
    }
}

/// Checked matching-only drain. Like the legacy no-decision adapter, this
/// leaves duration-end returns queued and never introduces player prompts.
/// A failed simultaneous projection restores the entire queue and pending
/// event set, even when the caller has no enclosing execution resource scope.
pub fn try_drain_pending_trigger_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), crate::effects::ExecutionError> {
    let (root, meter) = game.begin_token_resource_scope();
    let checkpoint = game.clone();
    let queue_checkpoint = trigger_queue.clone();
    let mut result = drain_pending_trigger_events_inner::<crate::effects::ExecutionError>(
        game,
        trigger_queue,
        |_| Ok(false),
    );
    if let Some(error) = game.token_resource_failure() {
        result = Err(error);
    }
    if result.is_err() {
        game.restore_execution_checkpoint(checkpoint, false);
        *trigger_queue = queue_checkpoint;
    }
    game.end_token_resource_scope(root, &meter);
    result
}

/// `drain_pending_trigger_events` for a caller with a player decision
/// channel: exile-until returns whose duration ends here (CR 610.3c) ask
/// their entry choices (a returned Clone's copy, an Aura's object, an
/// optional entry payment) through `decision_maker` instead of taking the
/// default answer.
pub fn drain_pending_trigger_events_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), crate::effects::ExecutionError> {
    let (root, meter) = game.begin_token_resource_scope();
    let checkpoint = game.clone();
    let queue_checkpoint = trigger_queue.clone();
    let mut result = drain_pending_trigger_events_inner(game, trigger_queue, |game| {
        if decision_maker.answers_player_choices() && game.has_pending_duration_end_returns() {
            game.process_pending_duration_end_returns(decision_maker)?;
            Ok(!decision_maker.awaiting_choice())
        } else {
            Ok(false)
        }
    });
    if let Some(error) = game.token_resource_failure() {
        result = Err(error);
    }
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint;
        *trigger_queue = queue_checkpoint;
    }
    game.end_token_resource_scope(root, &meter);
    result
}

fn drain_pending_trigger_events_inner<E>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    mut execute_duration_returns: impl FnMut(&mut GameState) -> Result<bool, E>,
) -> Result<(), E> {
    if game.action_observations_suppressed() {
        return Ok(());
    }
    for entry in game.take_pending_trigger_entries() {
        trigger_queue.add(entry);
    }

    let mut one_or_more_zone_changes_seen = HashSet::new();
    loop {
        let pending_events = game.take_pending_trigger_events();
        if pending_events.is_empty() {
            // CR 610.3c: exile-until durations that ended are returned with
            // the players answering the entry choices; without a player
            // decision channel they wait for the next caller that has one.
            if execute_duration_returns(game)? {
                continue;
            }
            break;
        }
        // CR 610.3c: a card exiled "until" its source leaves returns right
        // after that source leaves, as a new event. Every event already
        // pending happened before that return, so they are all matched first
        // (CR 603.2): a returning creature doesn't see them. The return's own
        // events are matched on the next pass.
        let mut departed_sources = Vec::new();
        let batch_lki_events = simultaneous_rule_ltb_batch_events(&pending_events);
        // Every producer-marked simultaneous action is matched as a whole.
        // Grouped trigger keys decide which occurrences coalesce; restricting
        // this to a few event kinds splits tap, phasing, and die-roll batches.
        let groups_by_batch = |event: &TriggerEvent| event.simultaneous_batch();
        let mut pending_events = pending_events.into_iter().map(Some).collect::<Vec<_>>();
        for index in 0..pending_events.len() {
            let Some(event) = pending_events[index].take() else {
                continue;
            };
            if let Some(batch) = groups_by_batch(&event) {
                // CR 603.2c: every event of one simultaneous action is matched
                // together, even when unrelated events (a sacrifice event, a
                // replacement's follow-up) were queued between them.
                let mut simultaneous = vec![event];
                for later in pending_events.iter_mut().skip(index + 1) {
                    if later
                        .as_ref()
                        .is_some_and(|next| groups_by_batch(next) == Some(batch))
                    {
                        simultaneous.extend(later.take());
                    }
                }
                crate::events::other::bind_die_roll_batch_results(&mut simultaneous);
                crate::events::damage::bind_received_damage_amounts(&mut simultaneous);
                queue_triggers_for_simultaneous_events(game, trigger_queue, simultaneous.clone());
                // CR 603.7b: a one-shot delayed trigger sees the whole group.
                queue_delayed_triggers_for_simultaneous_events(game, trigger_queue, &simultaneous);
                for event in &simultaneous {
                    if let Some(change) = event.downcast::<crate::events::ZoneChangeEvent>()
                        && change.from == crate::zone::Zone::Battlefield
                        && change.to != crate::zone::Zone::Battlefield
                    {
                        departed_sources.extend(change.objects.iter().copied());
                    }
                }
                continue;
            }
            let source_leave = event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
                .and_then(|zone_change| {
                    (zone_change.from == crate::zone::Zone::Battlefield
                        && zone_change.to != crate::zone::Zone::Battlefield)
                        .then(|| zone_change.objects.clone())
                });
            let queue_start = trigger_queue.entries.len();
            queue_triggers_from_event(game, trigger_queue, event, true);
            suppress_duplicate_one_or_more_zone_change_triggers(
                trigger_queue,
                queue_start,
                &mut one_or_more_zone_changes_seen,
            );
            if let Some(source_ids) = source_leave {
                departed_sources.extend(source_ids);
            }
        }

        for event in batch_lki_events {
            let Some(zone_change) = event.downcast::<crate::events::zones::ZoneChangeEvent>()
            else {
                continue;
            };
            let source_stable_ids: HashSet<_> = zone_change
                .snapshots()
                .iter()
                .map(|snapshot| snapshot.stable_id)
                .collect();
            trigger_queue.entries.retain(|entry| {
                if entry.source_snapshot.is_none()
                    || !source_stable_ids.contains(&entry.source_stable_id)
                {
                    return true;
                }
                let Some(entry_zone_change) = entry
                    .triggering_event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                else {
                    return true;
                };
                entry_zone_change.from != zone_change.from
                    || entry_zone_change.to != zone_change.to
                    || entry_zone_change.cause != zone_change.cause
            });
            for trigger in crate::triggers::check_triggers(game, &event) {
                if trigger.source_snapshot.is_some()
                    && source_stable_ids.contains(&trigger.source_stable_id)
                {
                    trigger_queue.add(trigger);
                }
            }
        }

        for source_id in departed_sources {
            game.return_exiled_for_source_leave(source_id);
        }
    }
    Ok(())
}

fn suppress_duplicate_one_or_more_zone_change_triggers(
    trigger_queue: &mut TriggerQueue,
    queue_start: usize,
    seen: &mut HashSet<(
        crate::ids::StableId,
        crate::triggers::TriggerIdentity,
        crate::zone::Zone,
        crate::zone::Zone,
        crate::events::cause::CauseType,
        Option<crate::ids::ObjectId>,
        Option<crate::ids::PlayerId>,
        Vec<crate::ids::ObjectId>,
    )>,
) {
    let mut added = trigger_queue.entries.split_off(queue_start);
    added.retain(|entry| {
        let Some(zone_change) = entry
            .triggering_event
            .downcast::<crate::events::zones::ZoneChangeEvent>()
        else {
            return true;
        };
        if !entry
            .ability
            .trigger
            .display()
            .to_ascii_lowercase()
            .contains("one or more")
        {
            return true;
        }
        let mut event_objects = zone_change.destination_objects().to_vec();
        event_objects.sort();
        event_objects.dedup();
        let key = (
            entry.source_stable_id,
            entry.trigger_identity,
            zone_change.from,
            zone_change.to,
            zone_change.cause.cause_type,
            zone_change.cause.source,
            zone_change.cause.source_controller,
            event_objects,
        );
        seen.insert(key)
    });
    trigger_queue.entries.append(&mut added);
}

pub type ExtractedTarget<'a> = crate::effects::TargetSelectionProfile<'a>;

/// Extract a ChooseSpec from an Effect, if it has one that requires selection.
pub fn extract_target_spec(effect: &Effect) -> Option<ExtractedTarget<'_>> {
    effect.target_selection_profile()
}

/// Counter transfers retain their authored endpoint order through wrappers.
pub(super) fn counter_transfer_target_specs(effect: &Effect) -> Option<(ChooseSpec, ChooseSpec)> {
    if let Some(value) = effect.downcast_ref::<crate::effects::FightEffect>() {
        return Some((value.creature1.clone(), value.creature2.clone()));
    }
    if let Some(value) = effect.downcast_ref::<crate::effects::MoveAllCountersEffect>() {
        return Some((value.from.clone(), value.to.clone()));
    }
    if let Some(value) = effect.downcast_ref::<crate::effects::MoveCountersEffect>() {
        return Some((value.from.clone(), value.to.clone()));
    }
    if let Some(value) = effect.downcast_ref::<crate::effects::MoveOneCounterEffect>() {
        return Some((value.from.clone(), value.to.clone()));
    }
    if let Some(child) = effect.transparent_child_effect() {
        return counter_transfer_target_specs(child);
    }
    if let Some(value) = effect.downcast_ref::<crate::effects::MayEffect>()
        && value.effects.len() == 1
    {
        return counter_transfer_target_specs(&value.effects[0]);
    }
    None
}

fn counter_endpoint_profile(spec: &ChooseSpec) -> ExtractedTarget<'_> {
    let (min_targets, max_targets) = exchange_target_bounds(spec);
    crate::effects::TargetSelectionProfile {
        spec,
        chooser: None,
        description: "counter transfer endpoint",
        min_targets,
        max_targets,
        count_value: None,
        distribution_value: None,
        distribution_min_per_target: 1,
        reuse_policy: crate::effects::TargetReusePolicy::AlwaysDeclareNew,
    }
}

/// Earlier synthetic declarations are borrowed once; fresh equal specs still
/// denote independent roles. Indices refer to the announced assignment table.
pub(super) fn counter_transfer_target_bindings(
    effect: &Effect,
    declared: &[DeclaredTarget],
) -> Option<Vec<(ChooseSpec, Option<usize>)>> {
    let (from, to) = counter_transfer_target_specs(effect)?;
    let mut shadow = declared.to_vec();
    let mut roles = Vec::new();
    for spec in [from, to] {
        if !requires_target_selection(&spec) {
            continue;
        }
        let previous = shadow.iter().position(|old| {
            old.synthetic_prelude
                && !old.synthetic_prelude_consumed
                && target_spec_reuses_declared_target(&spec, &old.spec)
        });
        let profile = counter_endpoint_profile(&spec);
        if !profile_reuses_declared_target(&profile, &mut shadow) {
            declare_target(&profile, &mut shadow);
        }
        roles.push((spec, previous));
    }
    Some(roles)
}

fn exchange_control_target_specs(effect: &Effect) -> Option<(ChooseSpec, ChooseSpec)> {
    if let Some(exchange) = effect.downcast_ref::<crate::effects::ExchangeControlEffect>()
        && exchange.permanent1 != exchange.permanent2
    {
        return Some((exchange.permanent1.clone(), exchange.permanent2.clone()));
    }

    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(specs) = exchange_control_target_specs(&tagged.effect)
    {
        return Some(specs);
    }

    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = exchange_control_target_specs(child);
        }
    });
    found
}

/// Target-count bounds an exchange target declares itself ("exchange control
/// of this creature and up to one target creature", Gilded Drake).
fn exchange_target_bounds(spec: &ChooseSpec) -> (usize, Option<usize>) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => exchange_target_bounds(spec),
        ChooseSpec::WithCount(_, count) => (count.min, count.max),
        _ => (1, Some(1)),
    }
}

fn relaxed_exchange_later_target_spec(spec: &ChooseSpec) -> ChooseSpec {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(relaxed_exchange_later_target_spec(spec)),
            hints: hints.clone(),
        },
        ChooseSpec::Target(inner) => {
            ChooseSpec::Target(Box::new(relaxed_exchange_later_target_spec(inner)))
        }
        ChooseSpec::Object(filter) => {
            let mut filter = filter.clone();
            filter.other = false;
            filter.tagged_constraints.clear();
            ChooseSpec::Object(filter)
        }
        ChooseSpec::WithCount(inner, count) => {
            ChooseSpec::WithCount(Box::new(relaxed_exchange_later_target_spec(inner)), *count)
        }
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(
            Box::new(relaxed_exchange_later_target_spec(inner)),
            *count,
            value.clone(),
        ),
        _ => spec.clone(),
    }
}

#[derive(Clone)]
pub(super) struct DeclaredTarget {
    spec: ChooseSpec,
    synthetic_prelude: bool,
    synthetic_prelude_consumed: bool,
}

fn declare_target(profile: &ExtractedTarget<'_>, declared: &mut Vec<DeclaredTarget>) {
    if matches!(
        profile.reuse_policy,
        crate::effects::TargetReusePolicy::AlwaysDeclareNew
            | crate::effects::TargetReusePolicy::SyntheticPrelude
    ) || !declared
        .iter()
        .any(|declared| target_spec_reuses_declared_target(profile.spec, &declared.spec))
    {
        declared.push(DeclaredTarget {
            spec: profile.spec.clone(),
            synthetic_prelude: matches!(
                profile.reuse_policy,
                crate::effects::TargetReusePolicy::SyntheticPrelude
            ),
            synthetic_prelude_consumed: false,
        });
    }
}

fn append_declared_targets_added_after(
    base_len: usize,
    declared: Vec<DeclaredTarget>,
    added: &mut Vec<DeclaredTarget>,
) {
    added.extend(declared.into_iter().skip(base_len));
}

/// Target state shared across children of one coordinated Oracle clause.
///
/// Ordinary sibling targets remain independent. Only lowering-generated
/// synthetic preludes cross the sibling boundary, and each such prelude can
/// still be consumed by at most one compatible target-bearing child.
#[derive(Default)]
pub(crate) struct CoordinatedTargetState {
    shared: Vec<DeclaredTarget>,
    additions: Vec<DeclaredTarget>,
    base_len: usize,
}

impl CoordinatedTargetState {
    fn from_declared(declared: &[DeclaredTarget]) -> Self {
        Self {
            shared: declared.to_vec(),
            additions: Vec::new(),
            base_len: declared.len(),
        }
    }

    fn child_state(&self) -> Vec<DeclaredTarget> {
        self.shared.clone()
    }

    fn merge_child_state(&mut self, child: Vec<DeclaredTarget>) {
        let shared_len = self.shared.len();
        for (shared, child) in self.shared.iter_mut().zip(&child) {
            if shared.synthetic_prelude {
                shared.synthetic_prelude_consumed |= child.synthetic_prelude_consumed;
            }
        }
        for declared in child.into_iter().skip(shared_len) {
            self.additions.push(declared.clone());
            if declared.synthetic_prelude {
                self.shared.push(declared);
            }
        }
    }

    fn finish(self, declared: &mut Vec<DeclaredTarget>) {
        let synthetic_consumption = self
            .shared
            .iter()
            .filter(|target| target.synthetic_prelude)
            .map(|target| target.synthetic_prelude_consumed)
            .collect::<Vec<_>>();
        let mut result = self.shared[..self.base_len].to_vec();
        result.extend(self.additions);
        for (target, consumed) in result
            .iter_mut()
            .filter(|target| target.synthetic_prelude)
            .zip(synthetic_consumption)
        {
            target.synthetic_prelude_consumed = consumed;
        }
        *declared = result;
    }
}

fn resolved_target_bounds(
    game: &GameState,
    profile: &ExtractedTarget<'_>,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> (usize, Option<usize>) {
    let count = profile.spec.count();
    if !count.is_dynamic_x() {
        return (profile.min_targets, profile.max_targets);
    }

    let Some(source_id) = source_id else {
        return (profile.min_targets, profile.max_targets);
    };
    let resolved = if let Some(count_value) = profile.count_value {
        let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(source_id, caster, &mut decision_maker);
        ctx.x_value = game.object(source_id).and_then(|source| source.x_value);
        match crate::effects::helpers::resolve_value(game, count_value, &ctx) {
            Ok(value) => value.max(0) as usize,
            Err(_) => return (profile.min_targets, profile.max_targets),
        }
    } else if let Some(x) = game.object(source_id).and_then(|source| source.x_value) {
        x as usize
    } else {
        return (profile.min_targets, profile.max_targets);
    };
    if count.is_up_to_dynamic_x() {
        (0, Some(resolved))
    } else {
        (resolved, Some(resolved))
    }
}

fn player_filter_reuses_declared_target(candidate: &PlayerFilter, declared: &PlayerFilter) -> bool {
    candidate == declared
        || matches!(declared, PlayerFilter::Target(inner) if candidate == inner.as_ref())
        || matches!(candidate, PlayerFilter::Target(inner) if inner.as_ref() == declared)
}

pub(super) fn target_spec_reuses_declared_target(
    candidate: &ChooseSpec,
    declared: &ChooseSpec,
) -> bool {
    if candidate == declared || candidate.base() == declared.base() {
        return true;
    }
    if target_spec_references_previous_target_tag(candidate) {
        return true;
    }

    match (candidate.base(), declared.base()) {
        (ChooseSpec::Player(candidate), ChooseSpec::Player(declared)) => {
            player_filter_reuses_declared_target(candidate, declared)
        }
        (
            ChooseSpec::PlayerOrPlaneswalker(candidate),
            ChooseSpec::PlayerOrPlaneswalker(declared),
        ) => player_filter_reuses_declared_target(candidate, declared),
        _ => false,
    }
}

pub(super) fn target_requirement_reuses_existing(
    candidate: &TargetRequirement,
    existing: &[TargetRequirement],
) -> bool {
    existing
        .iter()
        .any(|existing| target_spec_reuses_declared_target(&candidate.spec, &existing.spec))
}

fn profile_reuses_declared_target(
    profile: &ExtractedTarget<'_>,
    declared: &mut [DeclaredTarget],
) -> bool {
    if profile.reuse_policy == crate::effects::TargetReusePolicy::SyntheticPrelude {
        return false;
    }

    for declared in declared {
        if !target_spec_reuses_declared_target(profile.spec, &declared.spec) {
            continue;
        }
        if profile.reuse_policy == crate::effects::TargetReusePolicy::AlwaysDeclareNew
            && (!declared.synthetic_prelude || declared.synthetic_prelude_consumed)
        {
            continue;
        }
        if declared.synthetic_prelude {
            declared.synthetic_prelude_consumed = true;
        }
        return true;
    }
    false
}

pub(super) fn resolve_modal_mode_counts(
    game: &GameState,
    source_id: Option<ObjectId>,
    modal: crate::effects::ModalEffectSpec<'_>,
) -> (usize, usize) {
    if source_id
        .and_then(|id| game.object(id))
        .is_some_and(|source| source.optional_costs_paid.was_entwined())
    {
        let all_modes = modal.modes.len();
        return (all_modes, all_modes);
    }

    if let Some(range) = modal.conditional_mode_range
        && source_id
            .and_then(|id| game.object(id))
            .is_some_and(|source| {
                source
                    .optional_costs_paid
                    .was_paid_label(range.required_optional_cost.clone())
            })
    {
        let max_modes = resolve_modal_count_value_for_source(
            game,
            source_id,
            &range.max_modes,
            modal.modes.len(),
        );
        let min_modes =
            resolve_modal_count_value_for_source(game, source_id, &range.min_modes, max_modes);
        return (min_modes, max_modes);
    }

    let max_modes = resolve_modal_count_value_for_source(
        game,
        source_id,
        modal.max_modes,
        modal.modes.len().max(1),
    );
    let min_modes =
        resolve_modal_count_value_for_source(game, source_id, modal.min_modes, max_modes);
    (min_modes, max_modes)
}

#[allow(dead_code)]
pub(super) fn effect_mode_has_legal_targets_with_view(
    game: &GameState,
    mode: &crate::effect::EffectMode,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    mode.effects.iter().all(|effect| {
        spell_effect_has_legal_targets_internal_with_preview_mode_selection(
            game,
            effect,
            caster,
            source_id,
            None,
            &mut consumed_modal_selection,
            &mut declared_targets,
            true,
            view,
        )
    })
}

fn declared_player_target_candidates_with_view(
    game: &GameState,
    declared_targets: &[DeclaredTarget],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Option<Vec<PlayerId>> {
    let player_target = declared_targets
        .iter()
        .find(|declared| matches!(declared.spec.base(), ChooseSpec::Player(_)))?;
    let mut candidates = crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
        game,
        &player_target.spec,
        caster,
        source_id,
        None,
        view,
    )
    .into_iter()
    .filter_map(|target| match target {
        Target::Player(player) => Some(player),
        Target::Object(_) => None,
    })
    .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    (!candidates.is_empty()).then_some(candidates)
}

fn distinct_player_assignment_exists(candidate_sets: &[Vec<PlayerId>]) -> bool {
    fn recurse(
        candidate_sets: &[Vec<PlayerId>],
        assigned: &mut HashSet<PlayerId>,
        index: usize,
    ) -> bool {
        if index == candidate_sets.len() {
            return true;
        }
        for player in &candidate_sets[index] {
            if assigned.insert(*player) {
                if recurse(candidate_sets, assigned, index + 1) {
                    return true;
                }
                assigned.remove(player);
            }
        }
        false
    }

    let mut ordered = candidate_sets.to_vec();
    ordered.sort_by_key(Vec::len);
    recurse(&ordered, &mut HashSet::new(), 0)
}

fn distinct_player_modal_selection_exists(
    legal_modes: &[(usize, Vec<PlayerId>)],
    min_points: usize,
    max_points: usize,
    allow_repeated_modes: bool,
) -> bool {
    fn recurse(
        legal_modes: &[(usize, Vec<PlayerId>)],
        min_points: usize,
        max_points: usize,
        allow_repeated_modes: bool,
        mode_index: usize,
        selected_points: usize,
        selected_players: &mut HashSet<PlayerId>,
    ) -> bool {
        if selected_points >= min_points {
            return true;
        }
        if mode_index == legal_modes.len() {
            return false;
        }

        if recurse(
            legal_modes,
            min_points,
            max_points,
            allow_repeated_modes,
            mode_index + 1,
            selected_points,
            selected_players,
        ) {
            return true;
        }

        let (point_cost, candidates) = &legal_modes[mode_index];
        let next_points = selected_points.saturating_add(*point_cost);
        if next_points > max_points {
            return false;
        }
        for player in candidates {
            if selected_players.insert(*player) {
                let next_mode = if allow_repeated_modes {
                    mode_index
                } else {
                    mode_index + 1
                };
                if recurse(
                    legal_modes,
                    min_points,
                    max_points,
                    allow_repeated_modes,
                    next_mode,
                    next_points,
                    selected_players,
                ) {
                    return true;
                }
                selected_players.remove(player);
            }
        }
        false
    }

    if min_points == 0 {
        return true;
    }
    recurse(
        legal_modes,
        min_points,
        max_points,
        allow_repeated_modes,
        0,
        0,
        &mut HashSet::new(),
    )
}

fn modal_effect_has_legal_targets_internal_with_view(
    game: &GameState,
    modal: crate::effects::ModalEffectSpec<'_>,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    declared_targets: &mut Vec<DeclaredTarget>,
    require_full_selection: bool,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let (min_modes, max_modes) = resolve_modal_mode_counts(game, source_id, modal);
    if min_modes > max_modes {
        return false;
    }
    if modal.modes.is_empty() || max_modes == 0 {
        return min_modes == 0;
    }

    if let Some(chosen_modes) = chosen_modes {
        let mut selected_count = 0usize;
        let mut seen_modes = std::collections::HashSet::new();
        let base_declared_targets = declared_targets.clone();
        let base_declared_len = base_declared_targets.len();
        let mut declared_targets_from_modes = Vec::new();
        let mut distinct_player_candidates = Vec::new();

        for mode_idx in chosen_modes {
            let Some(mode) = modal.modes.get(*mode_idx) else {
                return false;
            };

            if !modal.allow_repeated_modes && !seen_modes.insert(*mode_idx) {
                return false;
            }

            let point_cost = modal
                .mode_point_costs
                .get(*mode_idx)
                .copied()
                .unwrap_or(1)
                .max(1) as usize;

            let mut mode_consumed_modal_selection = false;
            let mut mode_declared_targets = base_declared_targets.clone();
            if !mode.effects.iter().all(|effect| {
                spell_effect_has_legal_targets_internal_with_preview_mode_selection(
                    game,
                    effect,
                    caster,
                    source_id,
                    None,
                    &mut mode_consumed_modal_selection,
                    &mut mode_declared_targets,
                    require_full_selection,
                    view,
                )
            }) {
                return false;
            };
            if modal.distinct_player_targets_per_mode {
                let Some(candidates) = declared_player_target_candidates_with_view(
                    game,
                    &mode_declared_targets[base_declared_len..],
                    caster,
                    source_id,
                    view,
                ) else {
                    return false;
                };
                distinct_player_candidates.push(candidates);
            }
            append_declared_targets_added_after(
                base_declared_len,
                mode_declared_targets,
                &mut declared_targets_from_modes,
            );
            selected_count += point_cost;
        }

        let valid_selection = if require_full_selection {
            selected_count >= min_modes && selected_count <= max_modes
        } else {
            selected_count <= max_modes
        } && (!modal.distinct_player_targets_per_mode
            || distinct_player_assignment_exists(&distinct_player_candidates));
        if valid_selection {
            declared_targets.extend(declared_targets_from_modes);
        }
        return valid_selection;
    }

    if modal.distinct_player_targets_per_mode {
        let legal_modes = modal
            .modes
            .iter()
            .enumerate()
            .filter_map(|(mode_idx, mode)| {
                let base_declared_len = declared_targets.len();
                let mut mode_consumed_modal_selection = false;
                let mut mode_declared_targets = declared_targets.clone();
                let legal = mode.effects.iter().all(|effect| {
                    spell_effect_has_legal_targets_internal_with_preview_mode_selection(
                        game,
                        effect,
                        caster,
                        source_id,
                        None,
                        &mut mode_consumed_modal_selection,
                        &mut mode_declared_targets,
                        require_full_selection,
                        view,
                    )
                });
                if !legal {
                    return None;
                }
                let candidates = declared_player_target_candidates_with_view(
                    game,
                    &mode_declared_targets[base_declared_len..],
                    caster,
                    source_id,
                    view,
                )?;
                let point_cost = modal
                    .mode_point_costs
                    .get(mode_idx)
                    .copied()
                    .unwrap_or(1)
                    .max(1) as usize;
                Some((point_cost, candidates))
            })
            .collect::<Vec<_>>();
        return distinct_player_modal_selection_exists(
            &legal_modes,
            min_modes,
            max_modes,
            modal.allow_repeated_modes,
        );
    }

    let legal_mode_count = modal
        .modes
        .iter()
        .filter(|mode| {
            let mut mode_consumed_modal_selection = false;
            let mut mode_declared_targets = declared_targets.clone();
            mode.effects.iter().all(|effect| {
                spell_effect_has_legal_targets_internal_with_preview_mode_selection(
                    game,
                    effect,
                    caster,
                    source_id,
                    None,
                    &mut mode_consumed_modal_selection,
                    &mut mode_declared_targets,
                    require_full_selection,
                    view,
                )
            })
        })
        .count();

    if min_modes == 0 {
        return true;
    }

    if modal.allow_repeated_modes {
        legal_mode_count > 0
    } else {
        legal_mode_count >= min_modes
    }
}

fn distribution_supports_minimum_target_count(
    game: &GameState,
    extracted: &ExtractedTarget<'_>,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    legal_targets: &[Target],
    min_targets: usize,
) -> bool {
    let Some(value) = extracted.distribution_value else {
        return true;
    };
    if min_targets == 0 {
        return true;
    }
    let Some(source) = source_id else {
        return true;
    };

    let resolved_targets = legal_targets
        .iter()
        .take(min_targets)
        .map(|target| match target {
            Target::Object(id) => crate::effects::ResolvedTarget::Object(*id),
            Target::Player(id) => crate::effects::ResolvedTarget::Player(*id),
        })
        .collect::<Vec<_>>();
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, caster, &mut decision_maker)
        .with_targets(resolved_targets);
    ctx.x_value = game.object(source).and_then(|object| object.x_value);
    let Ok(total) = crate::effects::helpers::resolve_value(game, value, &ctx) else {
        return true;
    };
    let required = extracted
        .distribution_min_per_target
        .saturating_mul(min_targets as u32);
    total.max(0) as u32 >= required
}

/// Some restrictive relative clauses are represented as a conditional around
/// the targeted effect so the authored sentence can round-trip. Unlike an
/// ordinary trailing "if" clause, these predicates constrain which object may
/// be announced as the target in the first place.
fn target_announcement_condition(effect: &Effect) -> Option<&crate::effect::Condition> {
    let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }
    match &conditional.condition {
        crate::effect::Condition::TargetSpellCastOrderThisTurn(_) => Some(&conditional.condition),
        _ => None,
    }
}

fn target_satisfies_announcement_condition(
    game: &GameState,
    condition: &crate::effect::Condition,
    caster: PlayerId,
    source: ObjectId,
    target: &Target,
) -> bool {
    let resolved_target = match target {
        Target::Object(id) => crate::effects::ResolvedTarget::Object(*id),
        Target::Player(id) => crate::effects::ResolvedTarget::Player(*id),
    };
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let context = crate::effects::ExecutionContext::new(source, caster, &mut decisions)
        .with_targets(vec![resolved_target]);
    crate::condition_eval::evaluate_condition_resolution(game, condition, &context).unwrap_or(false)
}

fn retain_targets_satisfying_announcement_condition(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    legal_targets: &mut Vec<Target>,
) {
    let Some(condition) = target_announcement_condition(effect) else {
        return;
    };
    let Some(source) = source_id else {
        legal_targets.clear();
        return;
    };
    legal_targets.retain(|target| {
        target_satisfies_announcement_condition(game, condition, caster, source, target)
    });
}

fn spell_effect_has_legal_targets_internal_with_preview_mode_selection(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
    require_full_mode_selection: bool,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return spell_effect_has_legal_targets_internal_with_preview_mode_selection(
            game,
            &with_id.effect,
            caster,
            source_id,
            chosen_modes,
            consumed_modal_selection,
            declared_targets,
            require_full_mode_selection,
            view,
        );
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::SentenceLeadingThen
                | ironsmith_core::SequenceSurface::CommaThen
        )
    {
        for inner in &sequence.effects {
            if !spell_effect_has_legal_targets_internal_with_preview_mode_selection(
                game,
                inner,
                caster,
                source_id,
                chosen_modes,
                consumed_modal_selection,
                declared_targets,
                require_full_mode_selection,
                view,
            ) {
                return false;
            }
        }
        return true;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface.is_coordinated()
    {
        let mut coordinated = CoordinatedTargetState::from_declared(declared_targets);
        for inner in &sequence.effects {
            let mut child_declared_targets = coordinated.child_state();
            if !spell_effect_has_legal_targets_internal_with_preview_mode_selection(
                game,
                inner,
                caster,
                source_id,
                chosen_modes,
                consumed_modal_selection,
                &mut child_declared_targets,
                require_full_mode_selection,
                view,
            ) {
                return false;
            }
            coordinated.merge_child_state(child_declared_targets);
        }
        coordinated.finish(declared_targets);
        return true;
    }

    if let Some(modal) = effect.modal_effect_spec() {
        let modes_for_this_modal = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };
        return modal_effect_has_legal_targets_internal_with_view(
            game,
            modal,
            caster,
            source_id,
            modes_for_this_modal,
            declared_targets,
            require_full_mode_selection,
            view,
        );
    }

    if let Some(extracted) = extract_target_spec(effect)
        && requires_target_selection(extracted.spec)
    {
        if profile_reuses_declared_target(&extracted, declared_targets) {
            return true;
        }
        declare_target(&extracted, declared_targets);
        let (min_targets, _) = resolved_target_bounds(game, &extracted, caster, source_id);
        // For "any number" effects, we can cast even with no legal targets.
        if min_targets == 0 {
            return true;
        }
        let chooser_candidates = extracted.chooser.map_or_else(
            || vec![None],
            |chooser| {
                delegated_target_chooser_candidates(game, caster, source_id, chooser)
                    .into_iter()
                    .map(Some)
                    .collect()
            },
        );
        return chooser_candidates.into_iter().any(|chooser| {
            let spec = chooser.map_or_else(
                || extracted.spec.clone(),
                |chooser| specialize_iterated_player_choose_spec(extracted.spec, chooser),
            );
            let specialized = ExtractedTarget {
                spec: &spec,
                ..extracted
            };
            // A player relation to an earlier target is checked when the
            // targets are chosen together; here any candidate suffices.
            let candidate_spec = relax_target_player_relation(&spec);
            let mut legal_targets =
                crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
                    game,
                    &candidate_spec,
                    caster,
                    source_id,
                    None,
                    view,
                );
            retain_targets_satisfying_announcement_condition(
                game,
                effect,
                caster,
                source_id,
                &mut legal_targets,
            );
            legal_targets.len() >= min_targets
                && distribution_supports_minimum_target_count(
                    game,
                    &specialized,
                    caster,
                    source_id,
                    &legal_targets,
                    min_targets,
                )
        });
    }

    true
}

#[allow(dead_code)]
pub(super) fn spell_effect_has_legal_targets_with_view(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    spell_effect_has_legal_targets_internal_with_preview_mode_selection(
        game,
        effect,
        caster,
        source_id,
        chosen_modes,
        &mut consumed_modal_selection,
        &mut declared_targets,
        true,
        view,
    )
}

#[allow(dead_code)]
pub(super) fn spell_effect_has_legal_targets_internal_with_view(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let mut declared_targets = Vec::new();
    spell_effect_has_legal_targets_internal_with_preview_mode_selection(
        game,
        effect,
        caster,
        source_id,
        chosen_modes,
        consumed_modal_selection,
        &mut declared_targets,
        true,
        view,
    )
}

pub(super) fn extract_target_requirements_from_effect_internal(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
    requirements: &mut Vec<TargetRequirement>,
    references: Option<&crate::cost::prospective_references::CostReferenceBindings>,
    declaration: Option<crate::cost::CounterRemovalDeclaration>,
) {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        extract_target_requirements_from_effect_internal(
            game,
            &with_id.effect,
            caster,
            source_id,
            chosen_modes,
            consumed_modal_selection,
            declared_targets,
            requirements,
            references,
            declaration,
        );
        return;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::SentenceLeadingThen
                | ironsmith_core::SequenceSurface::CommaThen
        )
    {
        for inner in &sequence.effects {
            extract_target_requirements_from_effect_internal(
                game,
                inner,
                caster,
                source_id,
                chosen_modes,
                consumed_modal_selection,
                declared_targets,
                requirements,
                references,
                declaration,
            );
        }
        return;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface.is_coordinated()
    {
        let mut coordinated = CoordinatedTargetState::from_declared(declared_targets);
        for inner in &sequence.effects {
            let mut child_declared_targets = coordinated.child_state();
            extract_target_requirements_from_effect_internal(
                game,
                inner,
                caster,
                source_id,
                chosen_modes,
                consumed_modal_selection,
                &mut child_declared_targets,
                requirements,
                references,
                declaration,
            );
            coordinated.merge_child_state(child_declared_targets);
        }
        coordinated.finish(declared_targets);
        return;
    }

    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        extract_for_players_target_requirements(
            game,
            for_players,
            caster,
            source_id,
            consumed_modal_selection,
            declared_targets,
            requirements,
            references,
            declaration,
        );
        return;
    }

    if let Some(modal) = effect.modal_effect_spec() {
        let modes_for_this_modal = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };
        if let Some(chosen_modes) = modes_for_this_modal {
            let distinct_player_group = modal.distinct_player_targets_per_mode.then(|| {
                requirements
                    .iter()
                    .filter_map(|requirement| requirement.distinct_player_group)
                    .max()
                    .map_or(0, |group| group + 1)
            });
            let base_declared_targets = declared_targets.clone();
            let base_declared_len = base_declared_targets.len();
            let mut declared_targets_from_modes = Vec::new();
            for mode_idx in chosen_modes {
                if let Some(mode) = modal.modes.get(*mode_idx) {
                    let mode_requirement_start = requirements.len();
                    let mut mode_declared_targets = base_declared_targets.clone();
                    for inner in &mode.effects {
                        extract_target_requirements_from_effect_internal(
                            game,
                            inner,
                            caster,
                            source_id,
                            None,
                            consumed_modal_selection,
                            &mut mode_declared_targets,
                            requirements,
                            references,
                            declaration,
                        );
                    }
                    for requirement in &mut requirements[mode_requirement_start..] {
                        requirement.description =
                            format!("{} — {}", mode.source_text, requirement.description);
                    }
                    if let Some(group) = distinct_player_group
                        && let Some(requirement) = requirements[mode_requirement_start..]
                            .iter_mut()
                            .find(|requirement| {
                                matches!(requirement.spec.base(), ChooseSpec::Player(_))
                            })
                    {
                        requirement.distinct_player_group = Some(group);
                    }
                    append_declared_targets_added_after(
                        base_declared_len,
                        mode_declared_targets,
                        &mut declared_targets_from_modes,
                    );
                }
            }
            declared_targets.extend(declared_targets_from_modes);
        }
        return;
    }

    if let Some((from, to)) = counter_transfer_target_specs(effect) {
        for spec in [from, to] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let profile = counter_endpoint_profile(&spec);
            if profile_reuses_declared_target(&profile, declared_targets) {
                continue;
            }
            declare_target(&profile, declared_targets);
            let (min_targets, max_targets) = exchange_target_bounds(&spec);
            // The destination can depend on the source selected in the same
            // announcement. Enumerate candidates without that unresolved
            // relation, then carry its shared-player group into validation.
            let candidates_spec = if prior_shared_player_requirement(&spec, requirements).is_some()
            {
                relax_target_player_relation(&spec)
            } else {
                spec.clone()
            };
            let candidates_spec =
                if prior_relative_target_requirement(&spec, requirements).is_some() {
                    relax_relative_object_target_source_exclusion(&candidates_spec)
                } else {
                    candidates_spec
                };
            let legal_targets = compute_legal_targets(game, &candidates_spec, caster, source_id);
            if !legal_targets.is_empty() {
                let legal_target_sets =
                    crate::targeting::legal_target_sets_for_spec(game, &spec, &legal_targets);
                let shared_player_group = link_target_controller_requirement(
                    game,
                    &spec,
                    &legal_targets,
                    requirements,
                    caster,
                    source_id,
                );
                let distinct_player_group =
                    link_relative_target_to_prior_requirement(&spec, requirements);
                requirements.push(TargetRequirement {
                    spec,
                    chooser: None,
                    legal_targets,
                    legal_target_sets,
                    aggregate_constraint: None,
                    description: "counter transfer endpoint".to_string(),
                    min_targets,
                    max_targets,
                    distinct_player_group,
                    shared_player_group,
                    distribution_value: None,
                    distribution_min_per_target: 1,
                });
            }
        }
        return;
    }

    if let Some((first, second)) = exchange_control_target_specs(effect) {
        for spec in [first, relaxed_exchange_later_target_spec(&second)] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let (min_targets, max_targets) = exchange_target_bounds(&spec);
            let profile = crate::effects::TargetSelectionProfile {
                spec: &spec,
                chooser: None,
                description: "target",
                min_targets,
                max_targets,
                count_value: None,
                distribution_value: None,
                distribution_min_per_target: 1,
                reuse_policy: crate::effects::TargetReusePolicy::AlwaysDeclareNew,
            };
            declare_target(&profile, declared_targets);
            let legal_targets = compute_legal_targets_with_counter_declaration(
                game,
                &spec,
                caster,
                source_id,
                references,
                declaration,
            );
            if !legal_targets.is_empty() {
                let legal_target_sets =
                    crate::targeting::legal_target_sets_for_spec(game, &spec, &legal_targets);
                requirements.push(TargetRequirement {
                    spec,
                    chooser: None,
                    legal_targets,
                    legal_target_sets,
                    aggregate_constraint: None,
                    description: "target".to_string(),
                    min_targets,
                    max_targets,
                    distinct_player_group: None,
                    shared_player_group: None,
                    distribution_value: None,
                    distribution_min_per_target: 1,
                });
            }
        }
        return;
    }

    if let Some(extracted) = extract_target_spec(effect)
        && requires_target_selection(extracted.spec)
    {
        if profile_reuses_declared_target(&extracted, declared_targets) {
            return;
        }
        declare_target(&extracted, declared_targets);
        let mut relaxed_spec = extracted.spec.clone();
        if prior_relative_target_requirement(extracted.spec, requirements).is_some() {
            relaxed_spec = relax_relative_object_target_source_exclusion(&relaxed_spec);
        }
        if prior_shared_player_requirement(extracted.spec, requirements).is_some() {
            // The shared-player group and distinct-object group are separate
            // constraints; a dependent target can require both at once.
            relaxed_spec = relax_target_player_relation(&relaxed_spec);
        }
        let mut legal_targets = compute_legal_targets_with_counter_declaration(
            game,
            &relaxed_spec,
            caster,
            source_id,
            references,
            declaration,
        );
        retain_targets_satisfying_announcement_condition(
            game,
            effect,
            caster,
            source_id,
            &mut legal_targets,
        );
        let (min_targets, max_targets) =
            resolved_target_bounds(game, &extracted, caster, source_id);
        let legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, extracted.spec, &legal_targets);
        let aggregate_constraint = crate::targeting::resolved_target_aggregate_constraint(
            game,
            extracted.spec,
            caster,
            source_id,
            &legal_targets,
        );
        // For "any number" effects (min_targets == 0), we can cast even with no legal targets.
        // For required targets (min_targets > 0), we need at least min_targets legal targets.
        let has_enough_targets = crate::targeting::has_enough_legal_targets_for_spec(
            game,
            extracted.spec,
            &legal_targets,
            min_targets,
        ) && aggregate_constraint
            .as_ref()
            .is_none_or(|constraint| constraint.supports_minimum(min_targets));
        if has_enough_targets || extracted.chooser.is_some() {
            let distinct_player_group =
                link_relative_target_to_prior_requirement(extracted.spec, requirements);
            let shared_player_group = link_target_controller_requirement(
                game,
                extracted.spec,
                &legal_targets,
                requirements,
                caster,
                source_id,
            );
            requirements.push(TargetRequirement {
                spec: extracted.spec.clone(),
                chooser: extracted.chooser.cloned(),
                legal_targets,
                legal_target_sets,
                aggregate_constraint,
                description: extracted.description.to_string(),
                min_targets,
                max_targets,
                distinct_player_group,
                shared_player_group,
                distribution_value: extracted.distribution_value.cloned(),
                distribution_min_per_target: extracted.distribution_min_per_target,
            });
        }
    }
}

/// The earlier requirement whose player a `TargetPlayerOrControllerOfTarget`
/// relation refers to: a player target, else (for an owner relation such as
/// "a card in that player's graveyard") an earlier object target's controller.
/// Relations to the previously announced object need joint assignment rather
/// than filtering an endpoint before that object has been chosen.
fn player_filter_has_prior_object_controller(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target) => true,
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_has_prior_object_controller(base)
                || player_filter_has_prior_object_controller(excluded)
        }
        _ => false,
    }
}

fn relax_prior_target_player_filter(filter: &PlayerFilter) -> PlayerFilter {
    // A dependency under exclusion cannot be replaced with Any in place:
    // Any minus Any is empty. Enumerate a superset, then validate exact pairs.
    if player_filter_has_prior_object_controller(filter) {
        return PlayerFilter::Any;
    }
    match filter {
        PlayerFilter::Target(_)
        | PlayerFilter::TargetPlayerOrControllerOfTarget
        | PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target) => PlayerFilter::Any,
        PlayerFilter::Excluding { base, excluded } => PlayerFilter::Excluding {
            base: Box::new(relax_prior_target_player_filter(base)),
            excluded: Box::new(relax_prior_target_player_filter(excluded)),
        },
        _ => filter.clone(),
    }
}

fn prior_shared_player_requirement(
    spec: &ChooseSpec,
    requirements: &[TargetRequirement],
) -> Option<usize> {
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    if filter
        .controller
        .as_ref()
        .is_some_and(player_filter_has_prior_object_controller)
        || filter
            .owner
            .as_ref()
            .is_some_and(player_filter_has_prior_object_controller)
    {
        return requirements
            .iter()
            .rposition(|r| matches!(r.spec.base(), ChooseSpec::Object(_)));
    }
    if [&filter.controller, &filter.owner]
        .iter()
        .any(|relation| matches!(relation, Some(PlayerFilter::Target(_))))
    {
        return requirements
            .iter()
            .rposition(|requirement| matches!(requirement.spec.base(), ChooseSpec::Player(_)));
    }
    let relation = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    if filter.controller != relation && filter.owner != relation {
        return None;
    }
    requirements.iter().rposition(|requirement| {
        matches!(
            requirement.spec.base(),
            ChooseSpec::Player(_) | ChooseSpec::PlayerOrPlaneswalker(_) | ChooseSpec::Object(_)
        )
    })
}

pub(super) fn relax_target_player_relation(spec: &ChooseSpec) -> ChooseSpec {
    match spec {
        ChooseSpec::Target(inner) => {
            ChooseSpec::Target(Box::new(relax_target_player_relation(inner)))
        }
        ChooseSpec::WithCount(inner, count) => {
            ChooseSpec::WithCount(Box::new(relax_target_player_relation(inner)), *count)
        }
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(relax_target_player_relation(spec)),
            hints: hints.clone(),
        },
        ChooseSpec::Object(filter) => {
            let mut filter = filter.clone();
            for player_filter in [&mut filter.controller, &mut filter.owner] {
                if let Some(original) = player_filter.as_ref() {
                    let relaxed = relax_prior_target_player_filter(original);
                    *player_filter = if relaxed == PlayerFilter::Any {
                        None
                    } else {
                        Some(relaxed)
                    };
                }
            }
            ChooseSpec::Object(filter)
        }
        _ => spec.clone(),
    }
}

fn link_target_controller_requirement(
    game: &GameState,
    spec: &ChooseSpec,
    candidates: &[Target],
    requirements: &mut [TargetRequirement],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Option<crate::decisions::context::SharedTargetPlayerGroup> {
    let ChooseSpec::Object(filter) = spec.base() else {
        return None;
    };
    let prior_index = prior_shared_player_requirement(spec, requirements)?;
    if filter
        .controller
        .as_ref()
        .is_some_and(player_filter_has_prior_object_controller)
        || filter
            .owner
            .as_ref()
            .is_some_and(player_filter_has_prior_object_controller)
    {
        let distinct = prior_relative_target_requirement(spec, requirements).is_some();
        let mut allowed_pairs = Vec::new();
        for prior in &requirements[prior_index].legal_targets {
            let Target::Object(id) = prior else {
                continue;
            };
            let Some(controller) = game.current_controller(*id) else {
                continue;
            };
            let mut bound = spec.clone();
            specialize_target_player_relation_in_choose_spec(
                &mut bound,
                controller,
                ResolutionPlayerRelation::PriorObjectController,
            );
            let bound = relax_relative_object_target_source_exclusion(&bound);
            let exact_candidates = compute_legal_targets(game, &bound, caster, source_id);
            for candidate in candidates {
                if exact_candidates.contains(candidate) && (!distinct || candidate != prior) {
                    allowed_pairs.push((*prior, *candidate));
                }
            }
        }
        return Some(crate::decisions::context::SharedTargetPlayerGroup {
            group: 0,
            target_players: Vec::new(),
            pair_constraint: Some(crate::decisions::context::TargetPairConstraint {
                prior_requirement: prior_index,
                allowed_pairs,
            }),
        });
    }
    let by_owner = !filter.controller.as_ref().is_some_and(|controller| {
        *controller == PlayerFilter::TargetPlayerOrControllerOfTarget
            || player_filter_has_prior_object_controller(controller)
    });
    let group = requirements
        .iter()
        .filter_map(|r| r.shared_player_group.as_ref().map(|g| g.group))
        .max()
        .map_or(0, |g| g + 1);
    let map_players = |targets: &[Target], by_owner: bool| {
        targets
            .iter()
            .filter_map(|target| {
                let player = match target {
                    Target::Player(player) => *player,
                    Target::Object(id) if by_owner => game.object(*id)?.owner,
                    Target::Object(id) => game.current_controller(*id)?,
                };
                Some((*target, player))
            })
            .collect::<Vec<_>>()
    };
    let prior = &mut requirements[prior_index];
    let group = prior
        .shared_player_group
        .as_ref()
        .map_or(group, |g| g.group);
    prior.shared_player_group = Some(crate::decisions::context::SharedTargetPlayerGroup {
        group,
        target_players: map_players(&prior.legal_targets, false),
        pair_constraint: None,
    });
    Some(crate::decisions::context::SharedTargetPlayerGroup {
        group,
        target_players: map_players(candidates, by_owner),
        pair_constraint: None,
    })
}

fn relative_target_player_exclusion_base(filter: &PlayerFilter) -> Option<&PlayerFilter> {
    filter.relative_target_exclusion_base()
}

fn relax_relative_object_target_source_exclusion(spec: &ChooseSpec) -> ChooseSpec {
    match spec {
        ChooseSpec::Target(inner) => ChooseSpec::Target(Box::new(
            relax_relative_object_target_source_exclusion(inner),
        )),
        ChooseSpec::WithCount(inner, count) => ChooseSpec::WithCount(
            Box::new(relax_relative_object_target_source_exclusion(inner)),
            *count,
        ),
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(
            Box::new(relax_relative_object_target_source_exclusion(inner)),
            *count,
            value.clone(),
        ),
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(relax_relative_object_target_source_exclusion(spec)),
            hints: hints.clone(),
        },
        ChooseSpec::Object(filter) => {
            let mut filter = filter.clone();
            filter.other = false;
            ChooseSpec::Object(filter)
        }
        _ => spec.clone(),
    }
}

fn prior_relative_target_requirement(
    spec: &ChooseSpec,
    requirements: &[TargetRequirement],
) -> Option<usize> {
    match spec.base() {
        ChooseSpec::Player(filter) => {
            relative_target_player_exclusion_base(filter)?;
            requirements
                .iter()
                .rposition(|requirement| matches!(requirement.spec.base(), ChooseSpec::Player(_)))
        }
        ChooseSpec::Object(filter)
            if filter.other
                && filter.source_surface.is_none()
                && filter.tagged_constraints.is_empty() =>
        {
            requirements
                .iter()
                .rposition(|requirement| matches!(requirement.spec.base(), ChooseSpec::Object(_)))
        }
        ChooseSpec::ObjectOrPlayer(filter, _)
            if filter.other
                && filter.source_surface.is_none()
                && filter.tagged_constraints.is_empty() =>
        {
            requirements.iter().rposition(|requirement| {
                matches!(
                    requirement.spec.base(),
                    ChooseSpec::ObjectOrPlayer(_, _) | ChooseSpec::Object(_)
                )
            })
        }
        _ => None,
    }
}

fn link_relative_target_to_prior_requirement(
    spec: &ChooseSpec,
    requirements: &mut [TargetRequirement],
) -> Option<usize> {
    let prior_index = prior_relative_target_requirement(spec, requirements)?;
    let next_group = requirements
        .iter()
        .filter_map(|requirement| requirement.distinct_player_group)
        .max()
        .map_or(0, |group| group + 1);
    let group = requirements[prior_index]
        .distinct_player_group
        .unwrap_or(next_group);
    requirements[prior_index].distinct_player_group = Some(group);
    Some(group)
}

fn extract_for_players_target_requirements(
    game: &GameState,
    for_players: &crate::effects::ForPlayersEffect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
    requirements: &mut Vec<TargetRequirement>,
    references: Option<&crate::cost::prospective_references::CostReferenceBindings>,
    declaration: Option<crate::cost::CounterRemovalDeclaration>,
) {
    let mut filter_ctx = crate::filter::FilterContext::new(caster)
        .with_active_player(game.turn.active_player)
        .with_opponents(
            game.turn_store
                .turn_order
                .iter()
                .copied()
                .filter(|player_id| *player_id != caster)
                .collect(),
        );
    if let Some(source_id) = source_id {
        filter_ctx = filter_ctx.with_source(source_id);
    }
    let players = game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .filter(|player| for_players.filter.matches_player(player.id, &filter_ctx))
        .map(|player| player.id)
        .collect::<Vec<_>>();

    for player in players {
        for inner in &for_players.effects {
            extract_target_requirements_from_iterated_effect(
                game,
                inner,
                caster,
                source_id,
                player,
                consumed_modal_selection,
                declared_targets,
                requirements,
                references,
                declaration,
            );
        }
    }
}

fn extract_target_requirements_from_iterated_effect(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    iterated_player: PlayerId,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
    requirements: &mut Vec<TargetRequirement>,
    references: Option<&crate::cost::prospective_references::CostReferenceBindings>,
    declaration: Option<crate::cost::CounterRemovalDeclaration>,
) {
    if let Some(extracted) = extract_target_spec(effect)
        && requires_target_selection(extracted.spec)
    {
        let spec = specialize_iterated_player_choose_spec(extracted.spec, iterated_player);
        let profile = ExtractedTarget {
            spec: &spec,
            chooser: extracted.chooser,
            description: extracted.description,
            min_targets: extracted.min_targets,
            max_targets: extracted.max_targets,
            count_value: extracted.count_value,
            distribution_value: extracted.distribution_value,
            distribution_min_per_target: extracted.distribution_min_per_target,
            reuse_policy: extracted.reuse_policy,
        };
        if profile_reuses_declared_target(&profile, declared_targets) {
            return;
        }
        declare_target(&profile, declared_targets);
        let legal_targets = compute_legal_targets_with_counter_declaration(
            game,
            &spec,
            caster,
            source_id,
            references,
            declaration,
        );
        let (min_targets, max_targets) = resolved_target_bounds(game, &profile, caster, source_id);
        let legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, &spec, &legal_targets);
        let aggregate_constraint = crate::targeting::resolved_target_aggregate_constraint(
            game,
            &spec,
            caster,
            source_id,
            &legal_targets,
        );
        let has_enough_targets = crate::targeting::has_enough_legal_targets_for_spec(
            game,
            &spec,
            &legal_targets,
            min_targets,
        ) && aggregate_constraint
            .as_ref()
            .is_none_or(|constraint| constraint.supports_minimum(min_targets));
        if has_enough_targets || extracted.chooser.is_some() {
            requirements.push(TargetRequirement {
                spec,
                chooser: extracted.chooser.cloned(),
                legal_targets,
                legal_target_sets,
                aggregate_constraint,
                description: extracted.description.to_string(),
                min_targets,
                max_targets,
                distinct_player_group: None,
                shared_player_group: None,
                distribution_value: extracted.distribution_value.cloned(),
                distribution_min_per_target: extracted.distribution_min_per_target,
            });
        }
        return;
    }

    extract_target_requirements_from_effect_internal(
        game,
        effect,
        caster,
        source_id,
        None,
        consumed_modal_selection,
        declared_targets,
        requirements,
        references,
        declaration,
    );
}

fn delegated_target_chooser_candidates(
    game: &GameState,
    controller: PlayerId,
    source_id: Option<ObjectId>,
    chooser: &PlayerFilter,
) -> Vec<PlayerId> {
    let filter_ctx = game.filter_context_for(controller, source_id);
    game.players
        .iter()
        .filter(|player| player.is_in_game())
        .filter_map(|player| {
            crate::filter::player_filter_matches_game(chooser, player.id, game, &filter_ctx)
                .then_some(player.id)
        })
        .collect()
}

pub(crate) fn specialize_iterated_player_choose_spec(
    spec: &ChooseSpec,
    player: PlayerId,
) -> ChooseSpec {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(specialize_iterated_player_choose_spec(spec, player)),
            hints: hints.clone(),
        },
        ChooseSpec::Target(inner) => ChooseSpec::Target(Box::new(
            specialize_iterated_player_choose_spec(inner, player),
        )),
        ChooseSpec::Player(filter) => {
            ChooseSpec::Player(specialize_iterated_player_filter(filter, player))
        }
        ChooseSpec::Object(filter) => {
            ChooseSpec::Object(specialize_iterated_player_object_filter(filter, player))
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => ChooseSpec::ObjectOrPlayer(
            specialize_iterated_player_object_filter(object_filter, player),
            specialize_iterated_player_filter(player_filter, player),
        ),
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            ChooseSpec::PlayerOrPlaneswalker(specialize_iterated_player_filter(filter, player))
        }
        ChooseSpec::EachPlayer(filter) => {
            ChooseSpec::EachPlayer(specialize_iterated_player_filter(filter, player))
        }
        ChooseSpec::All(filter) => {
            ChooseSpec::All(specialize_iterated_player_object_filter(filter, player))
        }
        ChooseSpec::WithCount(inner, count) => ChooseSpec::WithCount(
            Box::new(specialize_iterated_player_choose_spec(inner, player)),
            *count,
        ),
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(
            Box::new(specialize_iterated_player_choose_spec(inner, player)),
            *count,
            value.clone(),
        ),
        _ => spec.clone(),
    }
}

fn specialize_iterated_player_object_filter(
    filter: &crate::filter::ObjectFilter,
    player: PlayerId,
) -> crate::filter::ObjectFilter {
    let mut filter = filter.clone();
    filter.controller = filter
        .controller
        .as_ref()
        .map(|controller| specialize_iterated_player_filter(controller, player));
    filter.owner = filter
        .owner
        .as_ref()
        .map(|owner| specialize_iterated_player_filter(owner, player));
    filter.cast_by = filter
        .cast_by
        .as_ref()
        .map(|cast_by| specialize_iterated_player_filter(cast_by, player));
    filter.targets_player = filter
        .targets_player
        .as_ref()
        .map(|targets_player| specialize_iterated_player_filter(targets_player, player));
    filter.targets_only_player = filter
        .targets_only_player
        .as_ref()
        .map(|targets_only_player| specialize_iterated_player_filter(targets_only_player, player));
    filter.attacking_player_or_planeswalker_controlled_by = filter
        .attacking_player_or_planeswalker_controlled_by
        .as_ref()
        .map(|attacking_player| specialize_iterated_player_filter(attacking_player, player));
    filter.protected_by = filter
        .protected_by
        .as_ref()
        .map(|protector| specialize_iterated_player_filter(protector, player));
    filter.attached_to_player = filter
        .attached_to_player
        .as_ref()
        .map(|attached_to_player| specialize_iterated_player_filter(attached_to_player, player));
    if let Some(attached_to_object) = filter.attached_to_object.as_ref() {
        filter.attached_to_object = Some(Box::new(specialize_iterated_player_object_filter(
            attached_to_object,
            player,
        )));
    }
    filter.entered_battlefield_controller = filter
        .entered_battlefield_controller
        .as_ref()
        .map(|controller| specialize_iterated_player_filter(controller, player));
    filter.discarded_or_cycled_this_turn_by = filter
        .discarded_or_cycled_this_turn_by
        .as_ref()
        .map(|actor| specialize_iterated_player_filter(actor, player));
    filter.dealt_damage_to_player_this_turn = filter
        .dealt_damage_to_player_this_turn
        .as_ref()
        .map(|damaged| specialize_iterated_player_filter(damaged, player));
    filter.last_drawn_this_turn = filter
        .last_drawn_this_turn
        .as_ref()
        .map(|drawer| specialize_iterated_player_filter(drawer, player));
    if let Some(constraint) = filter.counters_put_on_this_turn.as_mut() {
        constraint.source_controller =
            specialize_iterated_player_filter(&constraint.source_controller, player);
    }
    if let Some(targets_object) = filter.targets_object.as_ref() {
        filter.targets_object = Some(Box::new(specialize_iterated_player_object_filter(
            targets_object,
            player,
        )));
    }
    if let Some(targets_only_object) = filter.targets_only_object.as_ref() {
        filter.targets_only_object = Some(Box::new(specialize_iterated_player_object_filter(
            targets_only_object,
            player,
        )));
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_ref() {
        filter.blocked_or_was_blocked_by_this_turn = Some(Box::new(
            specialize_iterated_player_object_filter(combat_partner, player),
        ));
    }
    filter.no_shared_creature_types_with = filter
        .no_shared_creature_types_with
        .iter()
        .map(|inner| specialize_iterated_player_object_filter(inner, player))
        .collect();
    for relation in &mut filter.characteristic_relations {
        relation.comparison =
            specialize_iterated_player_object_filter(&relation.comparison, player);
    }
    filter.any_of = filter
        .any_of
        .iter()
        .map(|inner| specialize_iterated_player_object_filter(inner, player))
        .collect();
    filter
}

fn specialize_iterated_player_filter(filter: &PlayerFilter, player: PlayerId) -> PlayerFilter {
    match filter {
        PlayerFilter::IteratedPlayer => PlayerFilter::Specific(player),
        PlayerFilter::Target(inner) => {
            PlayerFilter::Target(Box::new(specialize_iterated_player_filter(inner, player)))
        }
        PlayerFilter::AliasedTarget(inner) => {
            PlayerFilter::AliasedTarget(Box::new(specialize_iterated_player_filter(inner, player)))
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            PlayerFilter::CardsInHandAtLeastMoreThanYou {
                base: Box::new(specialize_iterated_player_filter(base, player)),
                count: *count,
            }
        }
        PlayerFilter::HasMoreLifeThanYou { base } => PlayerFilter::HasMoreLifeThanYou {
            base: Box::new(specialize_iterated_player_filter(base, player)),
        },
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => {
            PlayerFilter::WasDealtDamageBySourceThisGame {
                base: Box::new(specialize_iterated_player_filter(base, player)),
            }
        }
        PlayerFilter::LostLifeThisTurn { base } => PlayerFilter::LostLifeThisTurn {
            base: Box::new(specialize_iterated_player_filter(base, player)),
        },
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base,
            sources,
            minimum,
        } => PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base: Box::new(specialize_iterated_player_filter(base, player)),
            sources: Box::new(specialize_iterated_player_object_filter(sources, player)),
            minimum: *minimum,
        },
        PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player: compared,
            filter,
            fewer,
            as_you_activate,
        } => PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player: Box::new(specialize_iterated_player_filter(compared, player)),
            filter: Box::new(specialize_iterated_player_object_filter(filter, player)),
            fewer: *fewer,
            as_you_activate: *as_you_activate,
        },
        PlayerFilter::ControlsMost { filter } => PlayerFilter::ControlsMost {
            filter: Box::new(specialize_iterated_player_object_filter(filter, player)),
        },
        PlayerFilter::ControlsFewestTied { filter } => PlayerFilter::ControlsFewestTied {
            filter: Box::new(specialize_iterated_player_object_filter(filter, player)),
        },
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => PlayerFilter::MaxSpeed {
            base: Box::new(specialize_iterated_player_filter(base, player)),
            has_max_speed: *has_max_speed,
        },
        PlayerFilter::Excluding { base, excluded } => PlayerFilter::Excluding {
            base: Box::new(specialize_iterated_player_filter(base, player)),
            excluded: Box::new(specialize_iterated_player_filter(excluded, player)),
        },
        _ => filter.clone(),
    }
}

fn count_target_selection_slots_from_effect_internal(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
) -> usize {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return count_target_selection_slots_from_effect_internal(
            &with_id.effect,
            chosen_modes,
            consumed_modal_selection,
            declared_targets,
        );
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::SentenceLeadingThen
                | ironsmith_core::SequenceSurface::CommaThen
        )
    {
        return sequence
            .effects
            .iter()
            .map(|inner| {
                count_target_selection_slots_from_effect_internal(
                    inner,
                    chosen_modes,
                    consumed_modal_selection,
                    declared_targets,
                )
            })
            .sum();
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface.is_coordinated()
    {
        let mut coordinated = CoordinatedTargetState::from_declared(declared_targets);
        let mut count = 0;
        for inner in &sequence.effects {
            let mut child_declared_targets = coordinated.child_state();
            count += count_target_selection_slots_from_effect_internal(
                inner,
                chosen_modes,
                consumed_modal_selection,
                &mut child_declared_targets,
            );
            coordinated.merge_child_state(child_declared_targets);
        }
        coordinated.finish(declared_targets);
        return count;
    }

    if let Some(modal) = effect.modal_effect_spec() {
        let modes_for_this_modal = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };

        let base_declared_targets = declared_targets.clone();
        let base_declared_len = base_declared_targets.len();
        let mut declared_targets_from_modes = Vec::new();
        let mut count = 0usize;
        for mode_idx in modes_for_this_modal.into_iter().flatten() {
            let Some(mode) = modal.modes.get(*mode_idx) else {
                continue;
            };
            let mut mode_declared_targets = base_declared_targets.clone();
            count += mode
                .effects
                .iter()
                .map(|inner| {
                    count_target_selection_slots_from_effect_internal(
                        inner,
                        None,
                        consumed_modal_selection,
                        &mut mode_declared_targets,
                    )
                })
                .sum::<usize>();
            append_declared_targets_added_after(
                base_declared_len,
                mode_declared_targets,
                &mut declared_targets_from_modes,
            );
        }
        declared_targets.extend(declared_targets_from_modes);
        return count;
    }

    if let Some((from, to)) = counter_transfer_target_specs(effect) {
        let mut count = 0;
        for spec in [from, to] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let profile = counter_endpoint_profile(&spec);
            if profile_reuses_declared_target(&profile, declared_targets) {
                continue;
            }
            declare_target(&profile, declared_targets);
            count += 1;
        }
        return count;
    }

    if let Some((first, second)) = exchange_control_target_specs(effect) {
        let mut count = 0;
        for spec in [first, second] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let (min_targets, max_targets) = exchange_target_bounds(&spec);
            let profile = crate::effects::TargetSelectionProfile {
                spec: &spec,
                chooser: None,
                description: "target",
                min_targets,
                max_targets,
                count_value: None,
                distribution_value: None,
                distribution_min_per_target: 1,
                reuse_policy: crate::effects::TargetReusePolicy::AlwaysDeclareNew,
            };
            declare_target(&profile, declared_targets);
            count += 1;
        }
        return count;
    }

    let Some(extracted) = extract_target_spec(effect) else {
        return 0;
    };
    if !requires_target_selection(extracted.spec) {
        return 0;
    }
    if profile_reuses_declared_target(&extracted, declared_targets) {
        return 0;
    }
    declare_target(&extracted, declared_targets);
    1
}

pub(crate) fn count_target_selection_slots_for_effect(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
) -> usize {
    count_target_selection_slots_from_effect_internal(
        effect,
        chosen_modes,
        consumed_modal_selection,
        declared_targets,
    )
}

pub(crate) fn count_target_selection_slots_for_isolated_effect(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
) -> usize {
    let mut declared_targets = Vec::new();
    count_target_selection_slots_from_effect_internal(
        effect,
        chosen_modes,
        consumed_modal_selection,
        &mut declared_targets,
    )
}

pub(crate) fn count_target_selection_slots_for_coordinated_child(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    coordinated: &mut CoordinatedTargetState,
) -> usize {
    let mut child_declared_targets = coordinated.child_state();
    let count = count_target_selection_slots_from_effect_internal(
        effect,
        chosen_modes,
        consumed_modal_selection,
        &mut child_declared_targets,
    );
    coordinated.merge_child_state(child_declared_targets);
    count
}

pub(crate) fn extract_target_requirements_for_effect_with_state(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
) -> Vec<TargetRequirement> {
    let mut requirements = Vec::new();
    let mut declared_targets = Vec::new();
    extract_target_requirements_from_effect_internal(
        game,
        effect,
        caster,
        source_id,
        chosen_modes,
        consumed_modal_selection,
        &mut declared_targets,
        &mut requirements,
        None,
        None,
    );
    requirements
}

fn cast_time_selected_effects_from_program(
    game: &GameState,
    program: &crate::resolution::ResolutionProgram,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Effect> {
    let Some(source_id) = source_id else {
        return program.flattened_default_effects().to_vec();
    };

    let mut selected = Vec::new();
    for segment in &program.segments {
        let applicable = segment
            .self_replacements
            .iter()
            .filter(|branch| {
                crate::condition_eval::evaluate_condition_cast_time(
                    game,
                    &branch.condition,
                    caster,
                    source_id,
                )
            })
            .collect::<Vec<_>>();

        let effects = match applicable.first() {
            Some(branch)
                if effects_have_new_cast_time_target_selection(&branch.replacement_effects)
                    || !effects_have_cast_time_target_selection(&segment.default_effects) =>
            {
                &branch.replacement_effects
            }
            _ => &segment.default_effects,
        };
        // Ordinary conditions resolve after targets are announced. Keep both
        // branches available to target discovery without reading future coin
        // receipts or treating a combat trigger as a spell being cast.
        selected.extend(effects.iter().flat_map(|effect| {
            announcement_effects_from_conditionals(game, effect, caster, source_id)
        }));
    }

    selected
}

fn effect_tree_contains_modes(effect: &Effect) -> bool {
    if effect.0.get_modal_spec().is_some() {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= effect_tree_contains_modes(child));
    found
}

fn announcement_effects_from_conditionals(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source: ObjectId,
) -> Vec<Effect> {
    let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() else {
        return vec![effect.clone()];
    };
    // A relative target clause still constrains announcement candidates.
    if target_announcement_condition(effect).is_some() {
        return vec![effect.clone()];
    }
    let branches = if effect_tree_contains_modes(effect) {
        // Conditional modal spells announce the modes in their applicable
        // branch (for example a commander-dependent mode count).
        if crate::condition_eval::evaluate_condition_cast_time(
            game,
            &conditional.condition,
            caster,
            source,
        ) {
            conditional.if_true.iter().collect::<Vec<_>>()
        } else {
            conditional.if_false.iter().collect::<Vec<_>>()
        }
    } else {
        conditional
            .if_true
            .iter()
            .chain(&conditional.if_false)
            .collect()
    };
    branches
        .into_iter()
        .flat_map(|child| announcement_effects_from_conditionals(game, child, caster, source))
        .collect()
}

fn effects_have_cast_time_target_selection(effects: &[Effect]) -> bool {
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    effects.iter().any(|effect| {
        count_target_selection_slots_from_effect_internal(
            effect,
            None,
            &mut consumed_modal_selection,
            &mut declared_targets,
        ) > 0
    })
}

fn effects_have_new_cast_time_target_selection(effects: &[Effect]) -> bool {
    effects
        .iter()
        .any(effect_has_new_cast_time_target_selection)
}

fn effect_has_new_cast_time_target_selection(effect: &Effect) -> bool {
    if let Some(modal) = effect.modal_effect_spec() {
        return modal.modes.iter().any(|mode| {
            mode.effects
                .iter()
                .any(effect_has_new_cast_time_target_selection)
        });
    }

    let Some(extracted) = extract_target_spec(effect) else {
        return false;
    };
    requires_target_selection(extracted.spec)
        && !target_spec_references_previous_target_tag(extracted.spec)
}

fn target_spec_references_previous_target_tag(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Object(filter) => object_filter_references_previous_target_tag(filter),
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            object_filter_references_previous_target_tag(object_filter)
                || player_filter_references_previous_target_tag(player_filter)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            player_filter_references_previous_target_tag(filter)
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            target_spec_references_previous_target_tag(inner)
        }
        _ => false,
    }
}

fn player_filter_references_previous_target_tag(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::ControllerOf(object_ref)
        | PlayerFilter::OwnerOf(object_ref)
        | PlayerFilter::AliasedOwnerOf(object_ref)
        | PlayerFilter::AliasedControllerOf(object_ref) => {
            matches!(object_ref, crate::filter::ObjectRef::Tagged(_))
        }
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            player_filter_references_previous_target_tag(inner)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_references_previous_target_tag(base)
                || player_filter_references_previous_target_tag(excluded)
        }
        _ => false,
    }
}

fn object_filter_references_previous_target_tag(filter: &crate::filter::ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        !matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        )
    })
}

pub fn extract_target_requirements_from_program_with_modes(
    game: &GameState,
    program: &crate::resolution::ResolutionProgram,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> Vec<TargetRequirement> {
    let selected = cast_time_selected_effects_from_program(game, program, caster, source_id);
    extract_target_requirements_with_modes(game, &selected, caster, source_id, chosen_modes)
}

/// Extract target requirements from a list of effects with optional mode choices.
pub(crate) fn extract_target_requirements_with_modes(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> Vec<TargetRequirement> {
    extract_target_requirements_with_modes_and_references(
        game,
        effects,
        caster,
        source_id,
        chosen_modes,
        None,
    )
}

pub(crate) fn extract_target_requirements_with_modes_and_references(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    references: Option<&crate::cost::prospective_references::CostReferenceBindings>,
) -> Vec<TargetRequirement> {
    extract_target_requirements_with_modes_and_announcements(
        game,
        effects,
        caster,
        source_id,
        chosen_modes,
        references,
        None,
    )
}

pub(crate) fn extract_target_requirements_with_modes_and_announcements(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    references: Option<&crate::cost::prospective_references::CostReferenceBindings>,
    declaration: Option<crate::cost::CounterRemovalDeclaration>,
) -> Vec<TargetRequirement> {
    let mut requirements = Vec::new();
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();

    for effect in effects {
        extract_target_requirements_from_effect_internal(
            game,
            effect,
            caster,
            source_id,
            chosen_modes,
            &mut consumed_modal_selection,
            &mut declared_targets,
            &mut requirements,
            references,
            declaration,
        );
    }

    requirements
}

/// Extract target requirements from a list of effects.
pub(super) fn extract_target_requirements(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<TargetRequirement> {
    extract_target_requirements_with_modes(game, effects, caster, source_id, None)
}

pub(crate) fn spell_has_legal_targets_with_modes(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> bool {
    let view = crate::derived_view::DerivedGameView::new(game);
    spell_has_legal_targets_with_modes_and_view(
        game,
        effects,
        caster,
        source_id,
        chosen_modes,
        &view,
    )
}

pub(crate) fn spell_program_has_legal_targets_with_modes(
    game: &GameState,
    program: &crate::resolution::ResolutionProgram,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> bool {
    let selected = cast_time_selected_effects_from_program(game, program, caster, source_id);
    spell_has_legal_targets_with_modes(game, &selected, caster, source_id, chosen_modes)
}

pub(crate) fn spell_program_has_legal_targets_with_modes_and_view(
    game: &GameState,
    program: &crate::resolution::ResolutionProgram,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let selected = cast_time_selected_effects_from_program(game, program, caster, source_id);
    spell_has_legal_targets_with_modes_and_view(
        game,
        &selected,
        caster,
        source_id,
        chosen_modes,
        view,
    )
}

pub(crate) fn spell_has_legal_targets_with_mode_preview(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: &[usize],
) -> bool {
    let view = crate::derived_view::DerivedGameView::new(game);
    spell_has_legal_targets_with_mode_preview_and_view(
        game,
        effects,
        caster,
        source_id,
        chosen_modes,
        &view,
    )
}

pub(crate) fn spell_has_legal_targets_with_mode_preview_and_view(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: &[usize],
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    for effect in effects {
        if !spell_effect_has_legal_targets_internal_with_preview_mode_selection(
            game,
            effect,
            caster,
            source_id,
            Some(chosen_modes),
            &mut consumed_modal_selection,
            &mut declared_targets,
            false,
            view,
        ) {
            return false;
        }
    }
    true
}

pub(crate) fn target_spec_uses_chosen_creature_type(spec: &ChooseSpec) -> bool {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::ObjectOrPlayer(filter, _) => {
            filter.chosen_creature_type && !filter.has_chosen_type_this_way_surface()
        }
        _ => false,
    }
}

fn effect_uses_chosen_creature_type_target(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal: &mut bool,
) -> bool {
    if let Some(modal) = effect.modal_effect_spec() {
        let modes = if !*consumed_modal {
            *consumed_modal = true;
            chosen_modes
        } else {
            None
        };
        return modal.modes.iter().enumerate().any(|(index, mode)| {
            modes.is_none_or(|selected| selected.contains(&index))
                && mode.effects.iter().any(|child| {
                    effect_uses_chosen_creature_type_target(child, None, consumed_modal)
                })
        });
    }
    if effect
        .target_selection_profile()
        .is_some_and(|profile| target_spec_uses_chosen_creature_type(profile.spec))
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_uses_chosen_creature_type_target(child, chosen_modes, consumed_modal);
    });
    found
}

fn effects_use_chosen_creature_type_target(
    effects: &[Effect],
    chosen_modes: Option<&[usize]>,
) -> bool {
    let mut consumed_modal = false;
    effects.iter().any(|effect| {
        effect_uses_chosen_creature_type_target(effect, chosen_modes, &mut consumed_modal)
    })
}

pub(crate) fn spell_program_uses_chosen_creature_type_target(
    game: &GameState,
    program: &crate::resolution::ResolutionProgram,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> bool {
    let effects = cast_time_selected_effects_from_program(game, program, caster, source_id);
    effects_use_chosen_creature_type_target(&effects, chosen_modes)
}

/// Enumerate legal completions of a subtype announcement before targets exist.
/// Each candidate uses the ordinary target legality checks with one fixed type.
pub(super) fn creature_type_announcement_options(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> Option<Vec<crate::types::Subtype>> {
    let source = source_id?;
    if game.chosen_subtype(source).is_some()
        || game.chosen_card_type(source).is_some()
        || !effects_use_chosen_creature_type_target(effects, chosen_modes)
    {
        return None;
    }
    let mut preview = game.clone();
    let mut options = Vec::new();
    for subtype in crate::types::SubtypeFamily::Creature.all_subtypes() {
        preview.set_chosen_subtype(source, *subtype);
        if spell_has_legal_targets_with_modes(&preview, effects, caster, source_id, chosen_modes) {
            options.push(*subtype);
        }
    }
    Some(options)
}

pub(super) fn pending_spell_creature_type_options(
    game: &GameState,
    pending: &PendingCast,
) -> Option<Vec<crate::types::Subtype>> {
    let program = game.object(pending.spell_id)?.spell_effect.as_ref()?;
    let effects = cast_time_selected_effects_from_program(
        game,
        program,
        pending.caster,
        Some(pending.spell_id),
    );
    creature_type_announcement_options(
        game,
        &effects,
        pending.caster,
        Some(pending.spell_id),
        pending.chosen_modes.as_deref(),
    )
}

pub(crate) fn spell_has_legal_targets_with_modes_and_view(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    if let Some(options) =
        creature_type_announcement_options(game, effects, caster, source_id, chosen_modes)
    {
        return !options.is_empty();
    }
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    for effect in effects {
        if !spell_effect_has_legal_targets_internal_with_preview_mode_selection(
            game,
            effect,
            caster,
            source_id,
            chosen_modes,
            &mut consumed_modal_selection,
            &mut declared_targets,
            true,
            view,
        ) {
            return false;
        }
    }
    declared_targets.windows(2).all(|pair| {
        let [source, recipient] = pair else {
            unreachable!()
        };
        let ChooseSpec::Object(_) = source.spec.base() else {
            return true;
        };
        let ChooseSpec::Object(recipient_filter) = recipient.spec.base() else {
            return true;
        };
        if source.spec.count() != crate::effect::ChoiceCount::exactly(1)
            || recipient.spec.count() != crate::effect::ChoiceCount::exactly(1)
            || recipient_filter.controller != Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        {
            return true;
        }
        let sources = crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
            game,
            &source.spec,
            caster,
            source_id,
            None,
            view,
        );
        let recipients = crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
            game,
            &relax_target_player_relation(&relax_relative_object_target_source_exclusion(
                &recipient.spec,
            )),
            caster,
            source_id,
            None,
            view,
        );
        sources.iter().any(|source| {
            let Target::Object(source_id) = source else {
                return false;
            };
            let controller = view.current_controller(*source_id);
            recipients.iter().any(|recipient| match recipient {
                Target::Object(id) => {
                    (!recipient_filter.other || id != source_id)
                        && controller.is_some()
                        && view.current_controller(*id) == controller
                }
                _ => false,
            })
        })
    })
}

/// Check if a spell has all required legal targets.
/// Returns true if all targeting requirements have enough legal targets,
/// or if the spell has no targeting requirements.
/// For "any number" effects (min_targets == 0), no legal targets are required.
pub fn spell_has_legal_targets(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> bool {
    let view = crate::derived_view::DerivedGameView::new(game);
    spell_has_legal_targets_with_modes_and_view(game, effects, caster, source_id, None, &view)
}

/// Compute legal targets for a given ChooseSpec.
///
/// The `caster` parameter is used for resolving "you control" and similar filters.
/// The `source_id` is used for "other" filters (exclude the source itself).
pub fn compute_legal_targets(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    crate::targeting::compute_legal_targets(game, spec, caster, source_id)
}

/// Compute legal targets for a given ChooseSpec with additional tagged-object context.
///
/// This is used for cases where a target filter references tagged constraints like
/// "that crewed it this turn" or "that saddled it this turn" during target selection.
pub fn compute_legal_targets_with_tagged_objects(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    tagged_objects: Option<
        &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
) -> Vec<Target> {
    crate::targeting::compute_legal_targets_with_tagged_objects(
        game,
        spec,
        caster,
        source_id,
        tagged_objects,
    )
}

pub(crate) fn compute_legal_targets_with_tagged_objects_combat_context_and_view(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    tagged_objects: Option<
        &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    defending_player: Option<PlayerId>,
    defending_player_reference: Option<crate::combat_state::DefendingPlayerReference>,
    attacking_player: Option<PlayerId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    if let Some(source) = source_id {
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = crate::effects::ExecutionContext::new(source, caster, &mut dm);
        ctx.source_snapshot = source_snapshot.cloned();
        if let Some(tagged) = tagged_objects {
            ctx.tagged_objects = tagged.clone();
        }
        ctx.combat.defending_player = defending_player;
        ctx.combat.defending_player_reference = defending_player_reference;
        ctx.combat.attacking_player = attacking_player;
        return crate::targeting::compute_legal_targets_with_execution_context_and_view(
            game, spec, &ctx, view,
        );
    }
    let combat_context = defending_player.zip(attacking_player);
    crate::targeting::compute_legal_targets_with_tagged_objects_combat_context_with_view(
        game,
        spec,
        caster,
        source_id,
        source_snapshot,
        tagged_objects,
        combat_context,
        view,
    )
}

fn compute_legal_targets_with_source_snapshot_and_view(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    tagged_objects: Option<
        &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
    crate::targeting::compute_legal_targets_with_tagged_objects_source_snapshot_with_view(
        game,
        spec,
        caster,
        source_id,
        source_snapshot,
        tagged_objects,
        view,
    )
}

/// Check if a player matches a PlayerFilter with explicit combat context.
pub fn player_matches_filter_with_combat(
    player_id: PlayerId,
    filter: &crate::target::PlayerFilter,
    game: &GameState,
    controller: PlayerId,
    combat: Option<&CombatState>,
) -> bool {
    use crate::combat_state::{get_attacking_player, is_defending_player};
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::Any => true,
        PlayerFilter::You => player_id == controller,
        PlayerFilter::NotYou => player_id != controller,
        PlayerFilter::Opponent => game.are_opponents(controller, player_id),
        PlayerFilter::PlayerToYourLeft => {
            game.closest_in_game_player_to_left_matching(controller, |_| true) == Some(player_id)
        }
        PlayerFilter::PlayerToYourRight => {
            game.closest_in_game_player_to_right_matching(controller, |_| true) == Some(player_id)
        }
        PlayerFilter::Active => game.is_active_player(player_id),
        PlayerFilter::Teammate => game.are_teammates(controller, player_id),
        PlayerFilter::Defending => combat
            .map(|c| is_defending_player(c, player_id))
            .unwrap_or(false),
        PlayerFilter::Attacking => combat
            .map(|c| get_attacking_player(c, game) == Some(player_id))
            .unwrap_or(false),
        PlayerFilter::DamagedPlayer => false,
        PlayerFilter::EffectController => player_id == controller,
        PlayerFilter::Specific(id) => player_id == *id,
        PlayerFilter::MostLifeTied => game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.life)
            .max()
            .is_some_and(|max_life| {
                game.player(player_id)
                    .is_some_and(|player| player.is_in_game() && player.life == max_life)
            }),
        PlayerFilter::LowestLifeTied => game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.life)
            .min()
            .is_some_and(|min_life| {
                game.player(player_id)
                    .is_some_and(|player| player.is_in_game() && player.life == min_life)
            }),
        PlayerFilter::MostCardsInHand => game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.hand.len())
            .max()
            .and_then(|max_hand| {
                let leaders = game
                    .players
                    .iter()
                    .filter(|player| player.is_in_game() && player.hand.len() == max_hand)
                    .map(|player| player.id)
                    .collect::<Vec<_>>();
                match leaders.as_slice() {
                    [leader] => Some(*leader == player_id),
                    _ => None,
                }
            })
            .unwrap_or(false),
        PlayerFilter::CastCardTypeThisTurn(card_type) => game
            .turn_store
            .turn_history
            .spell_cast_snapshot_history()
            .iter()
            .any(|snapshot| {
                snapshot.controller == player_id && snapshot.card_types.contains(card_type)
            }),
        // Source-relative history is not meaningful while validating a
        // standalone player target; these filters are used by effect loops.
        PlayerFilter::AttackedBySourceThisTurn
        | PlayerFilter::WasDealtDamageBySourceThisGame { .. }
        | PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => false,
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => {
            let filter_ctx = game.filter_context_for(controller, None);
            crate::filter::player_filter_matches_game(filter, player_id, game, &filter_ctx)
        }
        PlayerFilter::LostLifeThisTurn { base } => {
            player_matches_filter_with_combat(player_id, base, game, controller, combat)
                && game
                    .turn_store
                    .turn_history
                    .player_lost_life_this_turn(player_id)
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            if !player_matches_filter_with_combat(player_id, base, game, controller, combat) {
                return false;
            }
            let candidate_hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            let your_hand = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
            candidate_hand >= your_hand.saturating_add(*count as usize)
        }
        PlayerFilter::HasMoreLifeThanYou { base } => {
            player_matches_filter_with_combat(player_id, base, game, controller, combat)
                && game
                    .player(player_id)
                    .zip(game.player(controller))
                    .is_some_and(|(candidate, you)| candidate.life > you.life)
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { .. }
        | PlayerFilter::ControlsMost { .. }
        | PlayerFilter::ControlsFewestTied { .. } => {
            let filter_ctx = game.filter_context_for(controller, None);
            crate::filter::player_filter_matches_game(filter, player_id, game, &filter_ctx)
        }
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => {
            player_matches_filter_with_combat(player_id, base, game, controller, combat)
                && game.has_max_speed(player_id) == *has_max_speed
        }
        PlayerFilter::OpponentOf(base) => game.players.iter().any(|other| {
            other.is_in_game()
                && game.are_opponents(other.id, player_id)
                && player_matches_filter_with_combat(other.id, base, game, controller, combat)
        }),
        PlayerFilter::ChosenPlayer => false,
        PlayerFilter::TaggedPlayer(_) => false,
        PlayerFilter::IteratedPlayer => {
            // IteratedPlayer is resolved at runtime during iteration, not here
            false
        }
        PlayerFilter::TargetPlayerOrControllerOfTarget => false,
        PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) => {
            // Target filters are resolved through targeting, not direct matching
            true
        }
        PlayerFilter::Excluding { base, excluded } => {
            if filter.relative_target_exclusion_base().is_some() {
                player_matches_filter_with_combat(player_id, base, game, controller, combat)
            } else {
                player_matches_filter_with_combat(player_id, base, game, controller, combat)
                    && !player_matches_filter_with_combat(
                        player_id, excluded, game, controller, combat,
                    )
            }
        }
        PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_)
        | PlayerFilter::AliasedOwnerOf(_)
        | PlayerFilter::AliasedControllerOf(_) => {
            // These require object resolution, not applicable for simple player matching
            false
        }
    }
}

/// Validate targets for a stack entry that's about to resolve.
///
/// Per MTG Rule 608.2b:
/// - If a spell/ability has targets and ALL targets are now illegal, it fizzles
/// - If SOME targets are still legal, the spell/ability resolves and does as much as possible
///
/// Returns (valid_targets, all_targets_invalid)
pub(super) fn collect_validation_target_specs_from_effect(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
    specs: &mut Vec<ChooseSpec>,
) {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::SentenceLeadingThen
                | ironsmith_core::SequenceSurface::CommaThen
        )
    {
        for inner in &sequence.effects {
            collect_validation_target_specs_from_effect(
                inner,
                chosen_modes,
                consumed_modal_selection,
                declared_targets,
                specs,
            );
        }
        return;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>()
        && sequence.surface.is_coordinated()
    {
        let mut coordinated = CoordinatedTargetState::from_declared(declared_targets);
        for inner in &sequence.effects {
            let mut child_declared_targets = coordinated.child_state();
            collect_validation_target_specs_from_effect(
                inner,
                chosen_modes,
                consumed_modal_selection,
                &mut child_declared_targets,
                specs,
            );
            coordinated.merge_child_state(child_declared_targets);
        }
        coordinated.finish(declared_targets);
        return;
    }

    if let Some(modal) = effect.modal_effect_spec() {
        let modes_for_this_modal = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };

        if let Some(chosen_modes) = modes_for_this_modal {
            for mode_idx in chosen_modes {
                if let Some(mode) = modal.modes.get(*mode_idx) {
                    for inner in &mode.effects {
                        collect_validation_target_specs_from_effect(
                            inner,
                            None,
                            consumed_modal_selection,
                            declared_targets,
                            specs,
                        );
                    }
                }
            }
        }
        return;
    }

    if let Some((from, to)) = counter_transfer_target_specs(effect) {
        for spec in [from, to] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let profile = counter_endpoint_profile(&spec);
            if profile_reuses_declared_target(&profile, declared_targets) {
                continue;
            }
            declare_target(&profile, declared_targets);
            specs.push(spec);
        }
        return;
    }

    if let Some(extracted) = extract_target_spec(effect)
        && requires_target_selection(extracted.spec)
    {
        if profile_reuses_declared_target(&extracted, declared_targets) {
            return;
        }
        declare_target(&extracted, declared_targets);
        specs.push(extracted.spec.clone());
    }
}

fn effect_contains_exchange_control(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::ExchangeControlEffect>()
        .is_some()
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && effect_contains_exchange_control(child) {
            found = true;
        }
    });
    found
}

/// The target specs of every exchange-control effect in this stack entry,
/// with each later target relaxed exactly as it was when targets were chosen.
fn stack_entry_exchange_control_specs(
    game: &GameState,
    entry: &StackEntry,
) -> Result<Vec<ChooseSpec>, crate::effects::ExecutionError> {
    let effects = if let Some(effects) = &entry.ability_effects {
        effects.clone()
    } else if let Some(obj) = game.object(entry.object_id) {
        get_effects_for_stack_entry(game, entry, obj)?
    } else {
        crate::resolution::ResolutionProgram::default()
    };

    Ok(effects
        .all_effects()
        .iter()
        .filter(|effect| effect_contains_exchange_control(effect))
        .filter_map(|effect| exchange_control_target_specs(effect))
        .flat_map(|(first, second)| [first, relaxed_exchange_later_target_spec(&second)])
        .filter(requires_target_selection)
        .collect())
}

/// An exchange-control target whose recorded assignment spec went stale is
/// still checked against the exchange's own printed target restrictions
/// (CR 608.2b): a target that stopped being a creature, or changed
/// controller in response, is illegal and the exchange doesn't happen
/// (CR 701.12a).
fn exchange_control_target_still_targetable(
    game: &GameState,
    entry: &StackEntry,
    exchange_specs: &[ChooseSpec],
    target: &Target,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> bool {
    let Target::Object(object_id) = target else {
        return false;
    };
    if !game
        .object(*object_id)
        .is_some_and(|object| object.zone == Zone::Battlefield)
    {
        return false;
    }

    exchange_specs.iter().any(|spec| {
        compute_legal_targets_with_source_snapshot_and_view(
            game,
            spec,
            entry.controller,
            Some(entry.object_id),
            entry.source_snapshot.as_ref(),
            if entry.tagged_objects.is_empty() {
                None
            } else {
                Some(&entry.tagged_objects)
            },
            view,
        )
        .contains(target)
    })
}

pub(super) fn stack_entry_validation_target_specs(
    game: &GameState,
    entry: &StackEntry,
) -> Result<Vec<ChooseSpec>, crate::effects::ExecutionError> {
    let effects = if let Some(effects) = &entry.ability_effects {
        effects.clone()
    } else if let Some(obj) = game.object(entry.object_id) {
        get_effects_for_stack_entry(game, entry, obj)?
    } else {
        crate::resolution::ResolutionProgram::default()
    };

    let mut specs = Vec::new();
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    for effect in effects.all_effects() {
        collect_validation_target_specs_from_effect(
            effect,
            entry.chosen_modes.as_deref(),
            &mut consumed_modal_selection,
            &mut declared_targets,
            &mut specs,
        );
    }
    Ok(specs)
}

pub(super) fn validate_stack_entry_targets(
    game: &GameState,
    entry: &StackEntry,
) -> Result<
    (
        Vec<ResolvedTarget>,
        Vec<crate::game_state::TargetAssignment>,
        bool,
    ),
    crate::effects::ExecutionError,
> {
    validate_stack_entry_targets_with_context(game, entry, None)
}

pub(super) fn validate_stack_entry_targets_with_context(
    game: &GameState,
    entry: &StackEntry,
    ctx: Option<&crate::effects::ExecutionContext>,
) -> Result<
    (
        Vec<ResolvedTarget>,
        Vec<crate::game_state::TargetAssignment>,
        bool,
    ),
    crate::effects::ExecutionError,
> {
    let checked = game
        .continuous_query_snapshot()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(&checked);
    validate_stack_entry_targets_with_view(&checked, entry, &view, ctx)
}

fn combat_attacking_player_for_entry(game: &GameState, entry: &StackEntry) -> Option<PlayerId> {
    entry
        .triggering_event
        .as_ref()
        .and_then(|event| event.object_id())
        .and_then(|attacker| game.object(attacker))
        .map(|attacker| game.controller_of(attacker))
}

fn damaged_player_from_event(event: Option<&TriggerEvent>) -> Option<PlayerId> {
    let damage = event?.downcast::<crate::events::DamageEvent>()?;
    match damage.target {
        crate::events::DamageTarget::Player(player) => Some(player),
        crate::events::DamageTarget::Object(_) => None,
    }
}

fn replace_damaged_player_filter(filter: &mut crate::target::PlayerFilter, player: PlayerId) {
    if matches!(filter, crate::target::PlayerFilter::DamagedPlayer) {
        *filter = crate::target::PlayerFilter::Specific(player);
    }
}

fn replace_damaged_player_object_filter(
    filter: &mut crate::target::ObjectFilter,
    player: PlayerId,
) {
    if let Some(controller) = &mut filter.controller {
        replace_damaged_player_filter(controller, player);
    }
    if let Some(owner) = &mut filter.owner {
        replace_damaged_player_filter(owner, player);
    }
    if let Some(cast_by) = &mut filter.cast_by {
        replace_damaged_player_filter(cast_by, player);
    }
    if let Some(targets_player) = &mut filter.targets_player {
        replace_damaged_player_filter(targets_player, player);
    }
    if let Some(targets_only_player) = &mut filter.targets_only_player {
        replace_damaged_player_filter(targets_only_player, player);
    }
    if let Some(entered_battlefield_controller) = &mut filter.entered_battlefield_controller {
        replace_damaged_player_filter(entered_battlefield_controller, player);
    }
    if let Some(constraint) = filter.counters_put_on_this_turn.as_mut() {
        replace_damaged_player_filter(&mut constraint.source_controller, player);
    }
    if let Some(attached_to_player) = &mut filter.attached_to_player {
        replace_damaged_player_filter(attached_to_player, player);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replace_damaged_player_object_filter(attached_to, player);
    }
    for nested in &mut filter.any_of {
        replace_damaged_player_object_filter(nested, player);
    }
}

pub(super) fn choose_spec_with_recorded_players_from_event(
    spec: &crate::target::ChooseSpec,
    event: Option<&TriggerEvent>,
) -> crate::target::ChooseSpec {
    let mut spec = spec.clone();
    if let Some(player) = damaged_player_from_event(event) {
        replace_damaged_player_choose_spec(&mut spec, player);
    }
    // This trigger's inferred participant is the holder in the completed
    // receipt. Target announcement and later validation must bind that same
    // player before any live player-filter query, just as resolution does.
    if let Some(change) =
        event.and_then(|event| event.downcast::<crate::events::MonarchChangedEvent>())
    {
        spec = specialize_iterated_player_choose_spec(&spec, change.monarch);
    }
    spec
}

fn replace_damaged_player_choose_spec(spec: &mut crate::target::ChooseSpec, player: PlayerId) {
    use crate::target::ChooseSpec;

    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            replace_damaged_player_choose_spec(spec, player);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            replace_damaged_player_object_filter(filter, player);
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            replace_damaged_player_object_filter(object_filter, player);
            replace_damaged_player_filter(player_filter, player);
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            replace_damaged_player_filter(filter, player);
        }
        _ => {}
    }
}

// Activation-time comparisons are target-selection gates; resolution only rechecks
// that the chosen player still satisfies the underlying player class.
fn player_filter_for_resolution_target_validation(
    filter: &crate::target::PlayerFilter,
) -> crate::target::PlayerFilter {
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base } => {
            player_filter_for_resolution_target_validation(base)
        }
        // "target opponent who controls more creatures than you do as you
        // activate this ability" (Keeper of the Beasts): the comparison was a
        // restriction on choosing the target; on resolution the player must
        // still be an opponent (CR 608.2b).
        PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player,
            as_you_activate: true,
            ..
        } => PlayerFilter::OpponentOf(Box::new(
            player_filter_for_resolution_target_validation(player),
        )),
        PlayerFilter::Target(inner) => PlayerFilter::Target(Box::new(
            player_filter_for_resolution_target_validation(inner),
        )),
        PlayerFilter::AliasedTarget(inner) => PlayerFilter::AliasedTarget(Box::new(
            player_filter_for_resolution_target_validation(inner),
        )),
        PlayerFilter::Excluding { base, excluded } => PlayerFilter::Excluding {
            base: Box::new(player_filter_for_resolution_target_validation(base)),
            excluded: Box::new(player_filter_for_resolution_target_validation(excluded)),
        },
        PlayerFilter::MaxSpeed {
            base,
            has_max_speed,
        } => PlayerFilter::MaxSpeed {
            base: Box::new(player_filter_for_resolution_target_validation(base)),
            has_max_speed: *has_max_speed,
        },
        _ => filter.clone(),
    }
}

fn choose_spec_for_resolution_target_validation(
    spec: &crate::target::ChooseSpec,
) -> crate::target::ChooseSpec {
    use crate::target::ChooseSpec;

    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => ChooseSpec::SurfaceHinted {
            spec: Box::new(choose_spec_for_resolution_target_validation(spec)),
            hints: hints.clone(),
        },
        ChooseSpec::Target(inner) => ChooseSpec::Target(Box::new(
            choose_spec_for_resolution_target_validation(inner),
        )),
        ChooseSpec::WithCount(inner, count) => ChooseSpec::WithCount(
            Box::new(choose_spec_for_resolution_target_validation(inner)),
            *count,
        ),
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(
            Box::new(choose_spec_for_resolution_target_validation(inner)),
            *count,
            value.clone(),
        ),
        ChooseSpec::Player(filter) => {
            ChooseSpec::Player(player_filter_for_resolution_target_validation(filter))
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            ChooseSpec::PlayerOrPlaneswalker(player_filter_for_resolution_target_validation(filter))
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => ChooseSpec::ObjectOrPlayer(
            object_filter.clone(),
            player_filter_for_resolution_target_validation(player_filter),
        ),
        _ => spec.clone(),
    }
}

#[derive(Clone, Copy)]
enum ResolutionPlayerRelation {
    PriorPlayerOrController,
    PriorObjectController,
}

fn specialize_target_player_relation(
    filter: &mut crate::target::PlayerFilter,
    player: PlayerId,
    relation: ResolutionPlayerRelation,
) {
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::TargetPlayerOrControllerOfTarget
            if matches!(relation, ResolutionPlayerRelation::PriorPlayerOrController) =>
        {
            *filter = PlayerFilter::Specific(player);
        }
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target)
            if matches!(relation, ResolutionPlayerRelation::PriorObjectController) =>
        {
            *filter = PlayerFilter::Specific(player);
        }
        PlayerFilter::Target(inner)
        | PlayerFilter::AliasedTarget(inner)
        | PlayerFilter::WasDealtDamageBySourceThisGame { base: inner }
        | PlayerFilter::LostLifeThisTurn { base: inner }
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { base: inner, .. }
        | PlayerFilter::HasMoreLifeThanYou { base: inner }
        | PlayerFilter::OpponentOf(inner)
        | PlayerFilter::MaxSpeed { base: inner, .. } => {
            specialize_target_player_relation(inner, player, relation);
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, .. } => {
            specialize_target_player_relation(base, player, relation);
        }
        PlayerFilter::Excluding { base, excluded } => {
            specialize_target_player_relation(base, player, relation);
            specialize_target_player_relation(excluded, player, relation);
        }
        _ => {}
    }
}

fn specialize_target_player_relation_in_object_filter(
    filter: &mut crate::target::ObjectFilter,
    player: PlayerId,
    relation: ResolutionPlayerRelation,
) {
    for player_filter in [
        &mut filter.controller,
        &mut filter.cast_by,
        &mut filter.owner,
        &mut filter.targets_player,
        &mut filter.targets_only_player,
        &mut filter.attacking_player_or_planeswalker_controlled_by,
        &mut filter.protected_by,
        &mut filter.attached_to_player,
        &mut filter.entered_battlefield_controller,
    ]
    .into_iter()
    .flatten()
    {
        specialize_target_player_relation(player_filter, player, relation);
    }
    if let Some(constraint) = &mut filter.counters_put_on_this_turn {
        specialize_target_player_relation(&mut constraint.source_controller, player, relation);
    }
    for nested in &mut filter.any_of {
        specialize_target_player_relation_in_object_filter(nested, player, relation);
    }
    for nested in [
        &mut filter.targets_object,
        &mut filter.targets_only_object,
        &mut filter.attached_to_object,
        &mut filter.with_attached_object,
        &mut filter.without_attached_object,
    ]
    .into_iter()
    .flatten()
    {
        specialize_target_player_relation_in_object_filter(nested, player, relation);
    }
}

fn specialize_target_player_relation_in_choose_spec(
    spec: &mut crate::target::ChooseSpec,
    player: PlayerId,
    relation: ResolutionPlayerRelation,
) {
    use crate::target::ChooseSpec;

    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            specialize_target_player_relation_in_choose_spec(spec, player, relation);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            specialize_target_player_relation_in_object_filter(filter, player, relation);
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            specialize_target_player_relation_in_object_filter(object_filter, player, relation);
            specialize_target_player_relation(player_filter, player, relation);
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            specialize_target_player_relation(filter, player, relation);
        }
        _ => {}
    }
}

/// Legal targets for one announced target assignment of a stack entry, with
/// the entry's controller, source LKI, tagged objects and reflexive results.
/// Shared by the CR 608.2b recheck and by effects that change or choose new
/// targets (CR 115.7), which test new targets against the same requirement.
pub(crate) struct AssignmentLegalTargets {
    pub(crate) legal_targets: Vec<Target>,
    /// "Another target ..." after an earlier object assignment: the earlier
    /// assignment's targets can't be chosen again for this one.
    pub(crate) relative_object_target: bool,
    pub(crate) prior_object_targets: Vec<Target>,
}

pub(crate) fn current_stack_entry_target_assignments(
    game: &GameState,
    entry: &StackEntry,
) -> Result<Vec<crate::game_state::TargetAssignment>, crate::effects::ExecutionError> {
    let mut assignments = entry.target_assignments.clone();
    if entry.is_ability {
        return Ok(assignments);
    }
    // Declaration slots alone cannot prove the current copied spell's program.
    // Missing historical program evidence must not masquerade as no targets.
    game.current_spell_program(entry.object_id)
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    if assignments.is_empty() {
        return Ok(assignments);
    }
    let chars = game
        .calculated_characteristics(entry.object_id)
        .ok_or(crate::effects::ExecutionError::ContinuousDiscovery(
        crate::static_ability_processor::StaticEffectDiscoveryError::UnavailableCharacteristics {
            object: entry.object_id,
        },
    ))?;
    chars
        .validate_numeric_range()
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    for assignment in &mut assignments {
        for change in &chars.text_changes {
            assignment.spec = crate::continuous::text_change_predicates::rewrite_choose_spec_words(
                &assignment.spec,
                *change,
            )
            .map_err(|error| {
                crate::effects::ExecutionError::ContinuousDiscovery(
                    crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(
                        error,
                    ),
                )
            })?;
        }
    }
    Ok(assignments)
}

pub(crate) fn stack_entry_assignment_legal_targets(
    game: &GameState,
    entry: &StackEntry,
    assignment_index: usize,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Result<AssignmentLegalTargets, crate::effects::ExecutionError> {
    let mut current = entry.clone();
    current.target_assignments = current_stack_entry_target_assignments(game, entry)?;
    Ok(stack_entry_current_assignment_legal_targets(
        game,
        &current,
        assignment_index,
        view,
    ))
}

fn stack_entry_current_assignment_legal_targets(
    game: &GameState,
    entry: &StackEntry,
    assignment_index: usize,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> AssignmentLegalTargets {
    // Stack entries retain snapshots for both spells and abilities. Preserve
    // their explicit role across retargeting and resolution instead of
    // inferring "ability" merely from a snapshot's presence.
    if !entry.is_ability && !view.is_casting_spell(entry.object_id) {
        return view.with_casting_spell(entry.object_id, || {
            stack_entry_current_assignment_legal_targets(game, entry, assignment_index, view)
        });
    }
    let assignment = &entry.target_assignments[assignment_index];
    let resolved_spec = choose_spec_with_recorded_players_from_event(
        &assignment.spec,
        entry.triggering_event.as_ref(),
    );
    let mut resolved_spec = choose_spec_for_resolution_target_validation(&resolved_spec);
    let prior_object_targets: Vec<_> = entry.target_assignments[..assignment_index]
        .iter()
        .filter(|prior| matches!(prior.spec.base(), ChooseSpec::Object(_)))
        .flat_map(|prior| entry.targets[prior.range.clone()].iter())
        .copied()
        .collect();
    let relative_object_target = !prior_object_targets.is_empty()
        && matches!(resolved_spec.base(), ChooseSpec::Object(filter)
            if filter.other && filter.source_surface.is_none()
                && filter.tagged_constraints.is_empty());
    if relative_object_target {
        resolved_spec = relax_relative_object_target_source_exclusion(&resolved_spec);
    }
    // A typed object-controller relation names the prior object endpoint,
    // independently of earlier player targets. Keep an empty endpoint's
    // position: it must not fall back to a different earlier assignment.
    let prior_object_controller = entry.target_assignments[..assignment_index]
        .iter()
        .rev()
        .find(|prior| matches!(prior.spec.base(), ChooseSpec::Object(_)))
        .and_then(|prior| entry.targets.get(prior.range.clone()))
        .and_then(|targets| targets.first())
        .and_then(|target| match target {
            Target::Object(id) => view.current_controller(*id),
            Target::Player(_) => None,
        });
    if let Some(player) = prior_object_controller {
        specialize_target_player_relation_in_choose_spec(
            &mut resolved_spec,
            player,
            ResolutionPlayerRelation::PriorObjectController,
        );
    }
    if let Some(player) = prior_player_or_planeswalker_target(game, entry, assignment_index, view) {
        specialize_target_player_relation_in_choose_spec(
            &mut resolved_spec,
            player,
            ResolutionPlayerRelation::PriorPlayerOrController,
        );
    } else if let Some(player) = prior_object_targets
        .first()
        .and_then(|target| match target {
            // "a card in that player's graveyard" after an object target:
            // that player is the earlier target's current controller.
            Target::Object(id) => view.current_controller(*id),
            Target::Player(_) => None,
        })
    {
        specialize_target_player_relation_in_choose_spec(
            &mut resolved_spec,
            player,
            ResolutionPlayerRelation::PriorPlayerOrController,
        );
    }
    // Reflexive entries retain the resolving parent's results. Use
    // them again when rechecking legality after players can respond.
    let legal_targets = if !entry.effect_outcomes.is_empty()
        || entry.x_value.is_some()
        || super::sba_triggers::trigger_target_depends_on_selected_player(&resolved_spec)
    {
        let mut ctx =
            crate::effects::ExecutionContext::new_default(entry.object_id, entry.controller);
        ctx.x_value = entry.x_value;
        ctx.effect_outcomes = entry.effect_outcomes.clone();
        // Relative references bind earlier target groups; including this
        // assignment would make "another" exclude its own retained target.
        ctx.targets = entry.target_assignments[..assignment_index]
            .iter()
            .flat_map(|prior| entry.targets[prior.range.clone()].iter())
            .map(|target| match target {
                Target::Object(id) => ResolvedTarget::Object(*id),
                Target::Player(id) => ResolvedTarget::Player(*id),
            })
            .collect();
        ctx.tagged_objects = entry.tagged_objects.clone();
        ctx.source_snapshot = entry.source_snapshot.clone();
        if let Some(event) = entry.triggering_event.clone() {
            ctx = ctx.with_triggering_event(event);
        }
        ctx.event_value_amount = entry.event_value_amount;
        ctx.combat.defending_player = entry.defending_player;
        ctx.combat.defending_player_reference = entry.defending_player_reference;
        ctx.combat.attacking_player = combat_attacking_player_for_entry(game, entry);
        crate::targeting::compute_legal_targets_with_execution_context_and_view(
            game,
            &resolved_spec,
            &ctx,
            view,
        )
    } else if (entry.defending_player.is_some() || entry.defending_player_reference.is_some()) {
        compute_legal_targets_with_tagged_objects_combat_context_and_view(
            game,
            &resolved_spec,
            entry.controller,
            Some(entry.object_id),
            entry.source_snapshot.as_ref(),
            Some(&entry.tagged_objects),
            entry.defending_player,
            entry.defending_player_reference,
            combat_attacking_player_for_entry(game, entry),
            view,
        )
    } else {
        compute_legal_targets_with_source_snapshot_and_view(
            game,
            &resolved_spec,
            entry.controller,
            Some(entry.object_id),
            entry.source_snapshot.as_ref(),
            if entry.tagged_objects.is_empty() {
                None
            } else {
                Some(&entry.tagged_objects)
            },
            view,
        )
    };
    // CR 115.5: a spell or ability on the stack is an illegal target for
    // itself (matters when its targets are changed or new ones chosen).
    // An ability's `object_id` is its source permanent, which it may target;
    // the ability itself is named by `ability_id`.
    let self_id = if entry.is_ability {
        entry.ability_id
    } else {
        Some(entry.object_id)
    };
    let mut legal_targets = legal_targets;
    if let Some(self_id) = self_id {
        legal_targets.retain(|target| *target != Target::Object(self_id));
    }
    AssignmentLegalTargets {
        legal_targets,
        relative_object_target,
        prior_object_targets,
    }
}

/// Target-set restrictions belong to the announced assignment, not to each
/// candidate independently. Recheck current characteristics without selecting
/// a convenient smaller subset when the total becomes too large (CR608.2b).
fn assignment_aggregate_still_legal(
    game: &GameState,
    entry: &StackEntry,
    spec: &ChooseSpec,
    assigned: &[Target],
    view: &crate::derived_view::DerivedGameView<'_>,
    supplied_context: Option<&crate::effects::ExecutionContext>,
) -> Result<bool, crate::effects::ExecutionError> {
    use crate::effect::ChoiceAggregateMetric;
    let Some(constraint) = spec.target_set_aggregate_constraint() else {
        return Ok(true);
    };
    if assigned.is_empty() {
        return Ok(true);
    };
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(entry.object_id, entry.controller, &mut dm);
    ctx.x_value = entry.x_value;
    if let Some(snapshot) = entry.source_snapshot.clone() {
        ctx = ctx.with_source_snapshot(snapshot);
    }
    if let Some(event) = entry.triggering_event.clone() {
        ctx = ctx.with_triggering_event(event);
    }
    if let Some(amount) = entry.event_value_amount {
        ctx = ctx.with_event_value_amount(amount);
    }
    ctx = ctx.with_tagged_objects(entry.tagged_objects.clone());
    ctx.targets = entry
        .targets
        .iter()
        .map(|target| match target {
            Target::Object(id) => ResolvedTarget::Object(*id),
            Target::Player(id) => ResolvedTarget::Player(*id),
        })
        .collect();
    ctx.target_assignments = entry.target_assignments.clone();
    apply_keyword_payment_tags_for_resolution(game, entry, &mut ctx);
    let maximum = if let Some(supplied) = supplied_context {
        crate::effects::helpers::resolve_value(game, &constraint.maximum, supplied)?
    } else {
        crate::effects::helpers::resolve_value(game, &constraint.maximum, &ctx)?
    };
    let minimum = constraint
        .minimum
        .as_ref()
        .map(|value| {
            if let Some(supplied) = supplied_context {
                crate::effects::helpers::resolve_value(game, value, supplied)
            } else {
                crate::effects::helpers::resolve_value(game, value, &ctx)
            }
        })
        .transpose()?
        .map(i128::from);
    let maximum = i128::from(maximum);
    let mut total = 0i128;
    let mut types = 0u128;
    for target in assigned {
        let Target::Object(id) = target else { continue };
        // The legality of one member can depend on the other announced
        // members, including independently illegal targets (CR608.2b;
        // Run Away Together's controller comparison is the same purpose).
        // Use exact departure/phasing LKI, never a later stable-card incarnation.
        let (power, toughness, mana_value, card_types) =
            if let Some(object) = game.object(*id).filter(|_| !game.is_phased_out(*id)) {
                let chars = view.current_characteristics_arc(*id).ok_or_else(|| {
                    crate::effects::ExecutionError::UnresolvableValue(
                        "aggregate target characteristics are unavailable".into(),
                    )
                })?;
                let mana_value = chars.linked_face_mana_value.unwrap_or_else(|| {
                    chars.mana_cost.as_ref().map_or(0, |cost| {
                        if object.zone == Zone::Stack {
                            cost.mana_value_with_x(object.x_value.unwrap_or(0))
                        } else {
                            cost.mana_value()
                        }
                    })
                });
                (
                    chars.power,
                    chars.toughness,
                    mana_value,
                    chars.card_types.to_vec(),
                )
            } else {
                let snapshot = game.source_last_known_snapshot(*id).ok_or_else(|| {
                    crate::effects::ExecutionError::UnresolvableValue(
                    "aggregate target requires exact departed or phased characteristic evidence"
                        .into(),
                )
                })?;
                let mana_value = snapshot.linked_face_mana_value.unwrap_or_else(|| {
                    snapshot.mana_cost.as_ref().map_or(0, |cost| {
                        if snapshot.zone == Zone::Stack {
                            cost.mana_value_with_x(snapshot.x_value.unwrap_or(0))
                        } else {
                            cost.mana_value()
                        }
                    })
                });
                (
                    snapshot.power,
                    snapshot.toughness,
                    mana_value,
                    snapshot.card_types.clone(),
                )
            };
        match constraint.metric {
            ChoiceAggregateMetric::Power => {
                total += i128::from(if card_types.contains(&crate::types::CardType::Creature) {
                    power.unwrap_or(0)
                } else {
                    0
                })
            }
            ChoiceAggregateMetric::Toughness => {
                total += i128::from(if card_types.contains(&crate::types::CardType::Creature) {
                    toughness.unwrap_or(0)
                } else {
                    0
                })
            }
            ChoiceAggregateMetric::ManaValue => total += i128::from(mana_value),
            ChoiceAggregateMetric::DistinctCardTypes => {
                for ty in card_types {
                    types |= 1u128 << (ty as u32);
                }
            }
        }
    }
    if constraint.metric == ChoiceAggregateMetric::DistinctCardTypes {
        total = i128::from(types.count_ones());
    }
    Ok(total <= maximum && minimum.is_none_or(|minimum| total >= minimum))
}

pub(super) fn validate_stack_entry_targets_with_view(
    game: &GameState,
    entry: &StackEntry,
    view: &crate::derived_view::DerivedGameView<'_>,
    ctx: Option<&crate::effects::ExecutionContext>,
) -> Result<
    (
        Vec<ResolvedTarget>,
        Vec<crate::game_state::TargetAssignment>,
        bool,
    ),
    crate::effects::ExecutionError,
> {
    let mut current = entry.clone();
    current.target_assignments = current_stack_entry_target_assignments(game, entry)?;
    validate_current_stack_entry_targets_with_view(game, &current, view, ctx)
}

fn validate_current_stack_entry_targets_with_view(
    game: &GameState,
    entry: &StackEntry,
    view: &crate::derived_view::DerivedGameView<'_>,
    ctx: Option<&crate::effects::ExecutionContext>,
) -> Result<
    (
        Vec<ResolvedTarget>,
        Vec<crate::game_state::TargetAssignment>,
        bool,
    ),
    crate::effects::ExecutionError,
> {
    if !entry.is_ability && !view.is_casting_spell(entry.object_id) {
        return view.with_casting_spell(entry.object_id, || {
            validate_current_stack_entry_targets_with_view(game, entry, view, ctx)
        });
    }
    if entry.targets.is_empty() {
        return Ok((Vec::new(), Vec::new(), false));
    }

    if let Some(reference) = entry.defending_player_reference
        && entry
            .target_assignments
            .iter()
            .map(|assignment| &assignment.spec)
            .chain(stack_entry_validation_target_specs(game, entry)?.iter())
            .any(|spec| spec.mentions_player_filter(&PlayerFilter::Defending))
    {
        game.defending_player_candidates(reference)?;
    }

    if !entry.target_assignments.is_empty() {
        let mut valid_targets = Vec::new();
        let mut valid_assignments = Vec::with_capacity(entry.target_assignments.len());
        let mut invalid_count = 0usize;
        let exchange_specs = stack_entry_exchange_control_specs(game, entry)?;

        for (assignment_index, assignment) in entry.target_assignments.iter().enumerate() {
            let AssignmentLegalTargets {
                legal_targets,
                relative_object_target,
                prior_object_targets,
            } = stack_entry_current_assignment_legal_targets(game, entry, assignment_index, view);

            let start = valid_targets.len();
            let assigned = entry.targets.get(assignment.range.clone()).ok_or_else(|| {
                crate::effects::ExecutionError::InternalError(
                    "target assignment range is outside its retained targets".into(),
                )
            })?;
            if !assignment_aggregate_still_legal(
                game,
                entry,
                &assignment.spec,
                assigned,
                view,
                ctx,
            )? {
                invalid_count += assigned.len();
                valid_assignments.push(crate::game_state::TargetAssignment {
                    spec: assignment.spec.clone(),
                    range: start..start,
                });
                continue;
            }
            for target in assigned {
                if (legal_targets.contains(target)
                    && (!relative_object_target || !prior_object_targets.contains(target)))
                    || (!exchange_specs.is_empty()
                        && exchange_control_target_still_targetable(
                            game,
                            entry,
                            &exchange_specs,
                            target,
                            view,
                        ))
                {
                    valid_targets.push(match target {
                        Target::Object(id) => ResolvedTarget::Object(*id),
                        Target::Player(id) => ResolvedTarget::Player(*id),
                    });
                } else {
                    invalid_count += 1;
                }
            }
            let end = valid_targets.len();
            valid_assignments.push(crate::game_state::TargetAssignment {
                spec: assignment.spec.clone(),
                range: start..end,
            });
        }

        let all_invalid = invalid_count == entry.targets.len();
        return Ok((valid_targets, valid_assignments, all_invalid));
    }

    let validation_specs = stack_entry_validation_target_specs(game, entry)?;
    if validation_specs
        .iter()
        .any(|spec| spec.target_set_aggregate_constraint().is_some())
    {
        let [spec] = validation_specs.as_slice() else {
            return Err(crate::effects::ExecutionError::UnresolvableValue(
                "aggregate target groups require retained assignment boundaries".into(),
            ));
        };
        let mut assigned = entry.clone();
        assigned.target_assignments = vec![crate::game_state::TargetAssignment {
            spec: spec.clone(),
            range: 0..entry.targets.len(),
        }];
        return validate_current_stack_entry_targets_with_view(game, &assigned, view, ctx);
    }
    let legal_target_sets: Vec<Vec<Target>> = validation_specs
        .iter()
        .map(|spec| {
            let resolved_spec =
                choose_spec_with_recorded_players_from_event(spec, entry.triggering_event.as_ref());
            let resolved_spec = choose_spec_for_resolution_target_validation(&resolved_spec);
            if entry.x_value.is_some() {
                let mut execution = crate::effects::ExecutionContext::new_default(
                    entry.object_id,
                    entry.controller,
                );
                execution.x_value = entry.x_value;
                execution.source_snapshot = entry.source_snapshot.clone();
                execution.tagged_objects = entry.tagged_objects.clone();
                execution.effect_outcomes = entry.effect_outcomes.clone();
                execution.triggering_event = entry.triggering_event.clone();
                execution.event_value_amount = entry.event_value_amount;
                execution.combat.defending_player = entry.defending_player;
                execution.combat.defending_player_reference = entry.defending_player_reference;
                execution.combat.attacking_player = combat_attacking_player_for_entry(game, entry);
                return crate::targeting::compute_legal_targets_with_execution_context_and_view(
                    game,
                    &resolved_spec,
                    &execution,
                    view,
                );
            }
            if (entry.defending_player.is_some() || entry.defending_player_reference.is_some()) {
                return compute_legal_targets_with_tagged_objects_combat_context_and_view(
                    game,
                    &resolved_spec,
                    entry.controller,
                    Some(entry.object_id),
                    entry.source_snapshot.as_ref(),
                    None,
                    entry.defending_player,
                    entry.defending_player_reference,
                    combat_attacking_player_for_entry(game, entry),
                    view,
                );
            }
            compute_legal_targets_with_source_snapshot_and_view(
                game,
                &resolved_spec,
                entry.controller,
                Some(entry.object_id),
                entry.source_snapshot.as_ref(),
                None,
                view,
            )
        })
        .collect();

    let mut valid_targets = Vec::new();
    let mut invalid_count = 0;

    for target in &entry.targets {
        let is_valid = if !legal_target_sets.is_empty() {
            legal_target_sets
                .iter()
                .any(|legal_targets| legal_targets.contains(target))
        } else {
            match target {
                Target::Object(obj_id) => game.object(*obj_id).is_some_and(|obj| {
                    obj.zone == Zone::Battlefield
                        || (obj.zone == Zone::Stack
                            && (game.grand_melee().is_none()
                                || game.object_is_on_current_stack(*obj_id)))
                }),
                Target::Player(player_id) => game
                    .player(*player_id)
                    .map(|p| p.is_in_game())
                    .unwrap_or(false),
            }
        };

        if is_valid {
            valid_targets.push(match target {
                Target::Object(id) => ResolvedTarget::Object(*id),
                Target::Player(id) => ResolvedTarget::Player(*id),
            });
        } else {
            invalid_count += 1;
        }
    }

    let all_invalid = invalid_count == entry.targets.len();
    Ok((valid_targets, Vec::new(), all_invalid))
}

#[cfg(test)]
mod captured_incarnation_target_contract_tests {
    use super::*;
    fn reference(relation: crate::filter::TaggedOpbjectRelation) -> ChooseSpec {
        let mut filter = crate::filter::ObjectFilter::default();
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: "cost_object".into(),
                relation,
            });
        ChooseSpec::Object(filter)
    }
    #[test]
    fn modal_target_requirements_quote_their_modes() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let effects = vec![Effect::new(crate::effects::ChooseModeEffect::choose_one(
            vec![crate::effect::EffectMode::new(
                "Deal 1 damage to any target.",
                vec![Effect::deal_damage(1, ChooseSpec::AnyTarget)],
            )],
        ))];
        let requirements =
            extract_target_requirements_with_modes(&game, &effects, alice, None, Some(&[0]));
        assert_eq!(requirements.len(), 1);
        assert!(
            requirements[0]
                .description
                .starts_with("Deal 1 damage to any target. — ")
        );
    }

    #[test]
    fn captured_incarnation_is_a_resolution_reference() {
        let spec = reference(crate::filter::TaggedOpbjectRelation::SameObjectId);
        assert!(!requires_target_selection(&spec));
        assert!(!requires_target_selection(&ChooseSpec::WithCount(
            Box::new(spec),
            crate::ChoiceCount::exactly(1)
        )));
    }
    #[test]
    fn explicit_target_of_captured_incarnation_still_requires_selection() {
        let spec = reference(crate::filter::TaggedOpbjectRelation::SameObjectId);
        assert!(requires_target_selection(&ChooseSpec::Target(Box::new(
            spec
        ))));
    }
    #[test]
    fn relational_reference_still_requires_a_candidate() {
        assert!(requires_target_selection(&reference(
            crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        )));
    }
}

#[cfg(test)]
mod prior_object_controller_recheck_tests {
    use super::*;
    fn scenario(
        prior: u8,
        empty_source: bool,
        changed_source: bool,
    ) -> (GameState, StackEntry, usize, ObjectId, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Controller reference creature",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
        let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let to = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let spell = game.create_object_from_definition(&creature, alice, Zone::Hand);
        let endpoint = ChooseSpec::Target(Box::new(ChooseSpec::creature()));
        let mut destination = crate::filter::ObjectFilter::creature();
        destination.other = true;
        destination.controller = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
        let destination = ChooseSpec::Target(Box::new(ChooseSpec::Object(destination)));
        let mut entry = StackEntry::new(spell, alice);
        if prior != 0 {
            entry.targets.push(if prior == 1 {
                Target::Player(bob)
            } else {
                Target::Object(other)
            });
            entry
                .target_assignments
                .push(crate::game_state::TargetAssignment {
                    spec: if prior == 1 {
                        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                    } else {
                        endpoint.clone()
                    },
                    range: 0..1,
                });
        }
        let start = entry.targets.len();
        if !empty_source {
            entry.targets.push(Target::Object(from));
        }
        entry
            .target_assignments
            .push(crate::game_state::TargetAssignment {
                spec: endpoint,
                range: start..entry.targets.len(),
            });
        let start = entry.targets.len();
        entry.targets.push(Target::Object(to));
        entry
            .target_assignments
            .push(crate::game_state::TargetAssignment {
                spec: destination,
                range: start..entry.targets.len(),
            });
        if changed_source {
            game.object_mut(from).unwrap().initial_controller = bob;
            assert_eq!(game.current_controller(from), Some(bob));
        }
        let index = entry.target_assignments.len() - 1;
        (game, entry, index, to, other)
    }
    fn legal(prior: u8) {
        let (game, entry, index, to, other) = scenario(prior, false, false);
        let view = crate::derived_view::DerivedGameView::new(&game);
        let result = stack_entry_assignment_legal_targets(&game, &entry, index, &view).unwrap();
        assert!(
            result.legal_targets.contains(&Target::Object(to)),
            "the destination must use the source endpoint's controller at resolution"
        );
        assert!(
            !result.legal_targets.contains(&Target::Object(other)),
            "an unrelated prior declaration cannot supply the controller"
        );
    }
    #[test]
    fn typed_controller_reference_rechecks_two_endpoint_roles() {
        legal(0);
    }
    #[test]
    fn typed_controller_reference_ignores_unrelated_prior_player() {
        legal(1);
    }
    #[test]
    fn typed_controller_reference_ignores_unrelated_prior_object() {
        legal(2);
    }
    #[test]
    fn typed_controller_reference_uses_current_source_controller() {
        let (game, entry, index, to, other) = scenario(2, false, true);
        let view = crate::derived_view::DerivedGameView::new(&game);
        let result = stack_entry_assignment_legal_targets(&game, &entry, index, &view).unwrap();
        assert!(!result.legal_targets.contains(&Target::Object(to)));
        assert!(
            result.legal_targets.contains(&Target::Object(other)),
            "a source controller change must be reflected in the legal candidate set"
        );
    }
    #[test]
    fn empty_source_role_does_not_borrow_unrelated_object_controller() {
        let (game, entry, index, _, _) = scenario(2, true, false);
        let view = crate::derived_view::DerivedGameView::new(&game);
        let result = stack_entry_assignment_legal_targets(&game, &entry, index, &view).unwrap();
        assert!(
            result.legal_targets.is_empty(),
            "an empty source endpoint must preserve its position instead of falling back to a different assignment"
        );
    }
}

#[cfg(test)]
mod authored_initial_controller_view_tests {
    use super::*;
    fn entry(owner: PlayerId, controller: PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let creature = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Public controlled entry",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
        let card = game.create_object_from_definition(&creature, owner, Zone::Graveyard);
        let source = game.create_object_from_definition(&creature, controller, Zone::Battlefield);
        let instruction =
            crate::effect::Effect::new(crate::effects::PutOntoBattlefieldEffect::you_control(
                ChooseSpec::SpecificObject(card),
                false,
            ));
        let mut ctx = crate::effects::ExecutionContext::new_default(source, controller);
        let outcome = crate::effects::execute_effect(&mut game, &instruction, &mut ctx).unwrap();
        let ids = outcome.objects().expect("public battlefield entry result");
        assert_eq!(ids.len(), 1);
        let entered = ids[0];
        assert_ne!(entered, card);
        assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(entered).unwrap().owner, owner);
        assert_eq!(game.object(entered).unwrap().initial_controller, controller);
        assert_eq!(game.current_controller(entered), Some(controller));
        let view = crate::derived_view::DerivedGameView::new(&game);
        assert_eq!(
            view.current_controller(entered),
            Some(controller),
            "derived target/replacement consumer must retain the actual authored entry controller instead of restoring ownership"
        );
    }
    #[test]
    fn public_alice_owned_entry_under_bob_retains_derived_controller() {
        entry(PlayerId::from_index(0), PlayerId::from_index(1));
    }
    #[test]
    fn public_bob_owned_entry_under_alice_retains_derived_controller() {
        entry(PlayerId::from_index(1), PlayerId::from_index(0));
    }
}

#[cfg(test)]
mod negative_prior_controller_announcement_tests {
    use super::*;
    #[test]
    fn excluded_prior_controller_keeps_two_roles_and_opposing_candidate() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Excluded controller relation",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
        let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let same = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let opposing = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let mut destination = crate::filter::ObjectFilter::creature();
        destination.other = true;
        destination.controller = Some(PlayerFilter::Excluding {
            base: Box::new(PlayerFilter::Any),
            excluded: Box::new(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
        });
        let effect = Effect::new(crate::effects::MoveCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Target(Box::new(ChooseSpec::creature())),
            ChooseSpec::Target(Box::new(ChooseSpec::Object(destination))),
        ));
        let requirements = extract_target_requirements_for_effect_with_state(
            &game,
            &effect,
            alice,
            Some(from),
            None,
            &mut false,
        );
        assert_eq!(
            requirements.len(),
            2,
            "an unresolved excluded-controller relation must retain the destination role"
        );
        assert!(
            requirements[1]
                .legal_targets
                .contains(&Target::Object(opposing)),
            "opposing controller must remain a candidate until source assignment is known"
        );
        let contexts = requirements
            .iter()
            .map(|r| crate::decisions::context::TargetRequirementContext {
                description: r.description.clone(),
                legal_targets: r.legal_targets.clone(),
                legal_target_sets: r.legal_target_sets.clone(),
                aggregate_constraint: r.aggregate_constraint.clone(),
                min_targets: r.min_targets,
                max_targets: r.max_targets,
                distinct_player_group: r.distinct_player_group,
                shared_player_group: r.shared_player_group.clone(),
            })
            .collect::<Vec<_>>();
        assert!(
            crate::targeting::validate_flat_target_assignment(
                &contexts,
                &[Target::Object(from), Target::Object(opposing)]
            ),
            "different controllers satisfy the negative relation"
        );
        assert!(
            !crate::targeting::validate_flat_target_assignment(
                &contexts,
                &[Target::Object(from), Target::Object(same)]
            ),
            "same controller violates the negative relation"
        );
    }
}

#[cfg(test)]
mod multiplayer_prior_controller_announcement_tests {
    use super::*;
    #[test]
    fn negative_controller_pair_preserves_third_player_and_opponent_intersection() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Multiplayer controller relation",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
        let ids = (0..3)
            .map(|i| {
                game.create_object_from_definition(
                    &definition,
                    PlayerId::from_index(i),
                    Zone::Battlefield,
                )
            })
            .collect::<Vec<_>>();
        let mut destination = crate::filter::ObjectFilter::creature();
        destination.other = true;
        destination.controller = Some(PlayerFilter::Excluding {
            base: Box::new(PlayerFilter::Opponent),
            excluded: Box::new(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
        });
        let effect = Effect::new(crate::effects::MoveCountersEffect::new(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Target(Box::new(ChooseSpec::creature())),
            ChooseSpec::Target(Box::new(ChooseSpec::Object(destination))),
        ));
        let req = extract_target_requirements_for_effect_with_state(
            &game,
            &effect,
            alice,
            Some(ids[0]),
            None,
            &mut false,
        );
        assert_eq!(req.len(), 2);
        let contexts = req
            .iter()
            .map(|r| crate::decisions::context::TargetRequirementContext {
                description: r.description.clone(),
                legal_targets: r.legal_targets.clone(),
                legal_target_sets: r.legal_target_sets.clone(),
                aggregate_constraint: r.aggregate_constraint.clone(),
                min_targets: r.min_targets,
                max_targets: r.max_targets,
                distinct_player_group: r.distinct_player_group,
                shared_player_group: r.shared_player_group.clone(),
            })
            .collect::<Vec<_>>();
        for from in 0..3 {
            for to in 0..3 {
                assert_eq!(
                    crate::targeting::validate_flat_target_assignment(
                        &contexts,
                        &[Target::Object(ids[from]), Target::Object(ids[to])]
                    ),
                    to != 0 && from != to,
                    "retain both original Opponent and excluded-source-controller predicates"
                );
            }
        }
    }
}

#[cfg(test)]
mod announcement_target_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::types::CardType;
    fn observer(game: &mut GameState, a: PlayerId) -> ObjectId {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Target observer")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::triggered(
                crate::triggers::Trigger::new(crate::triggers::PlayerBecomesTargetedTrigger {
                    player_filter: PlayerFilter::You,
                    source_controller: PlayerFilter::Any,
                    source_kind: crate::filter::StackObjectKind::SpellOrAbility,
                }),
                vec![Effect::draw(1)],
            ))
            .build();
        game.create_object_from_definition(&definition, a, Zone::Battlefield)
    }
    #[test]
    fn announced_targets_keep_original_observer_and_clone_or_cancel_with_action() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let observer = observer(&mut game, a);
        let source = crate::card::CardBuilder::new(CardId::new(), "Ability source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&source, b, Zone::Battlefield);
        let mut state = PriorityLoopState::new(2);
        state.save_checkpoint(&game);
        let ability_id = game.allocate_stack_ability_id();
        let mut entry = StackEntry::ability(
            source,
            b,
            crate::resolution::ResolutionProgram::from_effects(vec![]),
        )
        .with_targets(vec![Target::Player(a), Target::Player(a)]);
        entry.ability_id = Some(ability_id);
        let captured = capture_announced_targeting(&mut game, entry.clone()).unwrap();
        assert_eq!(
            captured.entries.len(),
            1,
            "a repeated slot is one target transition"
        );
        assert!(
            game.stack.is_empty(),
            "announcement inspection does not finalize the action"
        );
        let targeted = captured.entries[0]
            .triggering_event
            .downcast::<BecomesTargetedEvent>()
            .unwrap();
        assert_eq!(targeted.stack_ability, Some(ability_id));
        assert_eq!(targeted.source, source);
        let history_count = game.turn_store.turn_history.event_records.len();
        let saved_game = game.clone();
        let saved_state = state.clone();
        let saved_queue = captured.clone();
        game.move_object_by_effect(observer, Zone::Graveyard)
            .unwrap();
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game.push_to_stack(entry);
        let mut queue = TriggerQueue::new();
        queue.append_captured(captured);
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(queue.entries[0].source, observer);
        assert_eq!(queue.entries[0].controller, a);
        game = saved_game;
        state = saved_state;
        assert_eq!(
            game.turn_store.turn_history.event_records.len(),
            history_count
        );
        assert_eq!(
            saved_queue.entries[0]
                .triggering_event
                .downcast::<BecomesTargetedEvent>()
                .unwrap()
                .stack_ability,
            Some(ability_id)
        );
        assert!(state.rollback_action(&mut game));
        assert!(game.object(observer).is_some());
        assert!(game.object(source).is_some());
        assert!(
            !game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .any(|record| record.event.kind() == crate::events::EventKind::BecomesTargeted)
        );
    }
}

#[cfg(test)]
mod completed_target_history_tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind};
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::types::CardType;
    #[test]
    fn restored_first_target_fact_does_not_reset_for_new_grants_but_new_incarnations_do() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let mut ability = Ability::triggered(
            crate::triggers::Trigger::becomes_targeted(),
            vec![Effect::draw(1)],
        );
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            panic!("trigger");
        };
        triggered.intervening_if = Some(crate::ConditionExpr::FirstTimeThisTurn);
        let definition = CardDefinitionBuilder::new(CardId::new(), "First target observer")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(ability)
            .build();
        let observer = game.create_object_from_definition(&definition, a, Zone::Battlefield);
        let spell = crate::card::CardBuilder::new(CardId::new(), "Targeting spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell = game.create_object_from_card(&spell, a, Zone::Stack);
        let event = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(observer, spell, a, false),
            Default::default(),
        );
        game.record_turn_history_event(&event);
        assert_eq!(crate::triggers::check_triggers(&game, &event).len(), 1);
        let ids = game
            .turn_store
            .turn_history
            .targeted_object_history_for_checkpoint();
        assert_eq!(ids, vec![observer]);
        game.turn_store.turn_history = Default::default();
        game.turn_store
            .turn_history
            .restore_targeted_object_history(ids)
            .unwrap();
        assert!(
            game.turn_store.turn_history.event_records.is_empty(),
            "the checkpoint does not invent a prior source/event"
        );
        let next = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(observer, spell, a, false),
            Default::default(),
        );
        assert!(crate::triggers::check_triggers(&game, &next).is_empty());
        let exiled = game.move_object_by_effect(observer, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(exiled, Zone::Battlefield)
            .unwrap();
        assert_ne!(observer, returned);
        let new_incarnation = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(returned, spell, a, false),
            Default::default(),
        );
        assert_eq!(
            crate::triggers::check_triggers(&game, &new_incarnation).len(),
            1
        );
        game.turn_store.turn_history.clear_for_new_turn();
        assert!(
            game.turn_store
                .turn_history
                .targeted_object_history_for_checkpoint()
                .is_empty()
        );
    }
}

#[cfg(test)]
mod current_aggregate_validation_tests {
    use super::*;
    #[test]
    fn unknown_dynamic_bound_is_not_silently_an_unrestricted_or_illegal_target() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Aggregate target")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
        let target = game.create_object_from_definition(&definition, b, Zone::Battlefield);
        let mut filter = crate::filter::ObjectFilter::creature();
        filter.target_set_aggregate_constraint =
            Some(Box::new(crate::effect::ChoiceAggregateConstraint::at_most(
                crate::effect::ChoiceAggregateMetric::ManaValue,
                crate::effect::Value::LastNotedLifeTotal,
            )));
        let mut entry = StackEntry::new(source, a).with_targets(vec![Target::Object(target)]);
        entry.target_assignments = vec![crate::game_state::TargetAssignment {
            spec: ChooseSpec::target(ChooseSpec::Object(filter)),
            range: 0..1,
        }];
        assert!(matches!(
            validate_stack_entry_targets(&game, &entry),
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ));
    }
}

fn prior_player_or_planeswalker_target(
    game: &GameState,
    entry: &StackEntry,
    before_assignment: usize,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Option<PlayerId> {
    entry
        .target_assignments
        .iter()
        .take(before_assignment)
        .rev()
        .filter(|assignment| {
            matches!(
                assignment.spec.base(),
                crate::target::ChooseSpec::PlayerOrPlaneswalker(_)
            )
        })
        .find_map(|assignment| {
            entry
                .targets
                .get(assignment.range.clone())?
                .iter()
                .find_map(|target| match target {
                    Target::Player(player) => Some(*player),
                    Target::Object(object) => game
                        .object(*object)
                        .filter(|_| game.current_has_card_type(*object, CardType::Planeswalker))
                        .and_then(|_| view.current_controller(*object)),
                })
        })
}

pub(super) fn compute_legal_targets_with_counter_declaration(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    references: Option<&crate::cost::prospective_references::CostReferenceBindings>,
    declaration: Option<crate::cost::CounterRemovalDeclaration>,
) -> Vec<Target> {
    let view = crate::derived_view::DerivedGameView::new(game)
        .with_target_reference_bindings(references.cloned().unwrap_or_default())
        .with_counter_removal_declaration(declaration);
    crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
        game, spec, caster, source_id, references, &view,
    )
}

#[cfg(test)]
mod completed_cast_capture_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::effects::ExecutionError;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::triggers::Trigger;
    use crate::types::CardType;

    #[test]
    fn incomplete_capture_rolls_back_history_and_prior_observer_matches() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let player = PlayerId(0);
        let source_card = CardBuilder::new(CardId::new(), "Cast observers")
            .card_types(vec![CardType::Enchantment]).build();
        let source = game.create_object_from_card(&source_card, player, Zone::Battlefield);
        let filtered = ObjectFilter {
            mana_value_eq_counters_on_source: Some(CounterType::Charge),
            ..ObjectFilter::spell()
        };
        {
            let object = game.object_mut(source).unwrap();
            object.counters.insert(CounterType::Charge, 1);
            object.abilities_mut().push(Ability::triggered(
                Trigger::spell_cast(None, PlayerFilter::You), vec![Effect::draw(1)],
            ));
            object.abilities_mut().push(Ability::triggered(
                Trigger::spell_cast(Some(filtered), PlayerFilter::You), vec![Effect::draw(1)],
            ));
        }
        let spell_card = CardBuilder::new(CardId::new(), "Missing announced X")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::X]])).build();
        let spell = game.create_object_from_card(&spell_card, player, Zone::Stack);
        game.push_to_stack(StackEntry::new(spell, player));
        let error = match capture_completed_spell_cast(&mut game, spell, player, Zone::Hand, Default::default()) {
            Err(error) => error,
            Ok(_) => panic!("missing X must not publish partial trigger matches"),
        };
        assert!(matches!(error, ExecutionError::IncompleteEvidence(_)));
        assert_eq!(game.player(player).unwrap().spells_cast_this_game, 0);
        assert_eq!(game.turn_store.turn_history.total_spells_cast_this_turn(), 0);
        assert!(game.effect_store.pending_trigger_entries.is_empty());
        game.object_mut(spell).unwrap().x_value = Some(1);
        let (receipt, mut captured) = capture_completed_spell_cast(
            &mut game, spell, player, Zone::Hand, Default::default(),
        ).unwrap();
        assert!(receipt.triggers_captured());
        assert_eq!(captured.take_all().len(), 2);
        game.record_turn_history_event(&receipt);
        assert_eq!(game.player(player).unwrap().spells_cast_this_game, 1);
        assert_eq!(game.turn_store.turn_history.total_spells_cast_this_turn(), 1);
    }
}
