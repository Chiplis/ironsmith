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
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
}

pub fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        // Explicit target wrappers always require cast/activation-time selection.
        ChooseSpec::Target(_) => true,
        ChooseSpec::WithCount(inner, _) | ChooseSpec::WithCountValue(inner, _, _) => {
            requires_target_selection(inner)
        }
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
    let triggers = check_triggers(game, &event);
    for trigger in triggers {
        if crate::triggers::check::is_speed_rule_trigger(&trigger) {
            game.mark_speed_increase_triggered_this_turn(trigger.controller);
        }
        trigger_queue.add(trigger);
    }
}

/// Ingest an event into trigger system with optional delayed-trigger checks.
pub(super) fn queue_triggers_from_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
    include_delayed: bool,
) {
    game.record_turn_history_event(&event);
    queue_triggers_for_event(game, trigger_queue, event.clone());

    if include_delayed {
        let delayed = crate::triggers::check_delayed_triggers(game, &event);
        for trigger in delayed {
            trigger_queue.add(trigger);
        }
    }
}

/// Queue trigger matches for each event in this list.
pub(super) fn queue_triggers_for_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
) {
    for event in events {
        queue_triggers_from_event(game, trigger_queue, event, false);
    }
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
    let events = events
        .into_iter()
        .map(|event| game.ensure_trigger_event_provenance(event))
        .collect::<Vec<_>>();
    for event in &events {
        game.record_turn_history_event(event);
    }

    let trigger_groups = check_triggers_batch(game, &events);
    let mut speed_controllers = std::collections::HashSet::new();
    let mut simultaneous_groups_seen = HashSet::new();
    for triggers in trigger_groups {
        // Delay inserting keys until this event's complete group is handled.
        // That preserves multiple identical ability instances on one object,
        // while suppressing their duplicate matches on later assignments in
        // the same simultaneous action.
        let mut groups_from_this_event = Vec::new();
        for trigger in triggers {
            if let Some(group) = trigger
                .ability
                .trigger
                .simultaneous_trigger_key(&trigger.triggering_event)
            {
                let key = (trigger.source_stable_id, trigger.trigger_identity, group);
                if simultaneous_groups_seen.contains(&key) {
                    continue;
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
    }
}

pub(super) fn target_events_from_targets(
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    provenance: ProvNodeId,
) -> Vec<TriggerEvent> {
    targets
        .iter()
        .map(|target| {
            TriggerEvent::new_with_provenance(
                BecomesTargetedEvent::new_target(*target, source, source_controller, by_ability),
                provenance,
            )
        })
        .collect()
}

pub(super) fn is_crime_target(game: &GameState, committer: PlayerId, target: &Target) -> bool {
    match target {
        Target::Player(player) => *player != committer,
        Target::Object(object_id) => {
            let Some(obj) = game.object(*object_id) else {
                return false;
            };
            if obj.zone == Zone::Graveyard {
                obj.owner != committer
            } else {
                game.current_controller(*object_id) != Some(committer)
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

pub(super) fn queue_becomes_targeted_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    provenance: ProvNodeId,
) {
    for mut event in
        target_events_from_targets(targets, source, source_controller, by_ability, provenance)
    {
        let event_provenance = game.alloc_child_event_provenance(provenance, event.kind());
        event.set_provenance(event_provenance);
        queue_triggers_from_event(game, trigger_queue, event, true);
    }

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

pub(super) fn queue_ability_activated_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
    source: ObjectId,
    activator: PlayerId,
    is_mana_ability: bool,
    source_stable_id: Option<StableId>,
    activation_cost_has_tap: bool,
) {
    let snapshot = if let Some(obj) = game.object(source) {
        Some(ObjectSnapshot::from_object(obj, game))
    } else if let Some(stable_id) = source_stable_id {
        game.find_object_by_stable_id(stable_id)
            .and_then(|id| game.object(id))
            .map(|obj| ObjectSnapshot::from_object(obj, game))
    } else {
        None
    };
    if is_mana_ability {
        let is_land_source = game
            .object(source)
            .map(|obj| obj.is_land())
            .or_else(|| snapshot.as_ref().map(|snap| snap.is_land()))
            .unwrap_or(false);
        if is_land_source {
            game.turn_store
                .turn_history
                .players_tapped_land_for_mana_this_turn
                .insert(activator);
        }
    }
    let activation_entry = game
        .stack
        .iter()
        .rev()
        .find(|entry| entry.is_ability && entry.object_id == source);
    let ability_index = activation_entry.and_then(|entry| entry.ability_index);
    let activated_ability = ability_index
        .and_then(|ability_index| game.current_ability(source, ability_index))
        .or_else(|| {
            let ability_index = ability_index?;
            activation_entry?
                .source_snapshot
                .as_ref()?
                .abilities
                .get(ability_index)
                .cloned()
        });
    let is_loyalty_ability = !is_mana_ability
        && activated_ability
            .as_ref()
            .is_some_and(|ability| match &ability.kind {
                crate::ability::AbilityKind::Activated(activated) => activated.is_loyalty_ability(),
                _ => false,
            });
    let x_value = activation_entry.and_then(|entry| entry.x_value);
    let activation_cost_has_x = activation_entry.is_some_and(|entry| entry.activation_cost_has_x);
    let mana_sources_tag = crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
    let mana_sources_spent = game
        .stack
        .iter()
        .rev()
        .find(|entry| entry.is_ability && entry.object_id == source)
        .and_then(|entry| entry.tagged_objects.get(&mana_sources_tag))
        .cloned()
        .unwrap_or_default();
    let event_provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::AbilityActivated);
    let event = TriggerEvent::new_with_provenance(
        AbilityActivatedEvent::new(source, activator, is_mana_ability)
            .with_loyalty_ability(is_loyalty_ability)
            .with_activated_ability(activated_ability)
            .with_activation_cost_has_x(activation_cost_has_x)
            .with_activation_cost_has_tap(activation_cost_has_tap)
            .with_x_value(x_value)
            .with_snapshot(snapshot)
            .with_mana_sources_spent(mana_sources_spent),
        event_provenance,
    );
    queue_triggers_from_event(game, trigger_queue, event, true);
    if is_mana_ability
        && matches!(
            resolve_triggered_mana_abilities_with_dm(game, trigger_queue, decision_maker),
            Err(GameLoopError::MandatoryLoopDraw)
        )
    {
        game.mark_mandatory_loop_draw();
    }
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

pub(super) fn tap_permanent_with_trigger(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    permanent: ObjectId,
) {
    if game.object(permanent).is_some() && !game.is_tapped(permanent) {
        game.tap(permanent);
        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::PermanentTapped);
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new_with_provenance(
                crate::events::PermanentTappedEvent::new(permanent),
                event_provenance,
            ),
            true,
        );
    }
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
    for entry in game.take_pending_trigger_entries() {
        trigger_queue.add(entry);
    }

    let mut one_or_more_zone_changes_seen = HashSet::new();
    loop {
        let pending_events = game.take_pending_trigger_events();
        if pending_events.is_empty() {
            break;
        }
        let batch_lki_events = simultaneous_rule_ltb_batch_events(&pending_events);
        let mut pending_events = pending_events.into_iter().peekable();
        while let Some(event) = pending_events.next() {
            if let Some(batch) = event.simultaneous_batch()
                && matches!(
                    event.kind(),
                    crate::events::EventKind::Damage | crate::events::EventKind::LifeLoss
                )
            {
                let mut simultaneous = vec![event];
                while pending_events
                    .peek()
                    .is_some_and(|next| next.simultaneous_batch() == Some(batch))
                {
                    simultaneous.push(
                        pending_events
                            .next()
                            .expect("peeked simultaneous event should still be present"),
                    );
                }
                queue_triggers_for_simultaneous_events(game, trigger_queue, simultaneous.clone());
                for event in &simultaneous {
                    for trigger in crate::triggers::check_delayed_triggers(game, event) {
                        trigger_queue.add(trigger);
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
                for source_id in source_ids {
                    game.return_exiled_for_source_leave(source_id);
                }
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
    }
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
            let mut legal_targets =
                crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
                    game, &spec, caster, source_id, None, view,
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
) {
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
                        );
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

    if let Some((first, second)) = exchange_control_target_specs(effect) {
        for spec in [first, relaxed_exchange_later_target_spec(&second)] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let profile = crate::effects::TargetSelectionProfile {
                spec: &spec,
                chooser: None,
                description: "target",
                min_targets: 1,
                max_targets: Some(1),
                count_value: None,
                distribution_value: None,
                distribution_min_per_target: 1,
                reuse_policy: crate::effects::TargetReusePolicy::AlwaysDeclareNew,
            };
            declare_target(&profile, declared_targets);
            let legal_targets = compute_legal_targets(game, &spec, caster, source_id);
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
                    min_targets: 1,
                    max_targets: Some(1),
                    distinct_player_group: None,
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
        let mut legal_targets = compute_legal_targets(game, extracted.spec, caster, source_id);
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
                link_relative_player_target_to_prior_requirement(extracted.spec, requirements);
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
                distribution_value: extracted.distribution_value.cloned(),
                distribution_min_per_target: extracted.distribution_min_per_target,
            });
        }
    }
}

fn relative_target_player_exclusion_base(filter: &PlayerFilter) -> Option<&PlayerFilter> {
    filter.relative_target_exclusion_base()
}

fn link_relative_player_target_to_prior_requirement(
    spec: &ChooseSpec,
    requirements: &mut [TargetRequirement],
) -> Option<usize> {
    let ChooseSpec::Player(filter) = spec.base() else {
        return None;
    };
    relative_target_player_exclusion_base(filter)?;
    let prior_index = requirements
        .iter()
        .rposition(|requirement| matches!(requirement.spec.base(), ChooseSpec::Player(_)))?;
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
        let legal_targets = compute_legal_targets(game, &spec, caster, source_id);
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

pub(super) fn specialize_iterated_player_choose_spec(
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
        } => PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player: Box::new(specialize_iterated_player_filter(compared, player)),
            filter: Box::new(specialize_iterated_player_object_filter(filter, player)),
        },
        PlayerFilter::ControlsMost { filter } => PlayerFilter::ControlsMost {
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

    if let Some((first, second)) = exchange_control_target_specs(effect) {
        let mut count = 0;
        for spec in [first, second] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let profile = crate::effects::TargetSelectionProfile {
                spec: &spec,
                chooser: None,
                description: "target",
                min_targets: 1,
                max_targets: Some(1),
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

        match applicable.as_slice() {
            [] => selected.extend(segment.default_effects.iter().flat_map(|effect| {
                cast_time_selected_effects_from_effect(game, effect, caster, Some(source_id))
            })),
            [branch] => {
                if effects_have_new_cast_time_target_selection(&branch.replacement_effects)
                    || !effects_have_cast_time_target_selection(&segment.default_effects)
                {
                    selected.extend(branch.replacement_effects.iter().flat_map(|effect| {
                        cast_time_selected_effects_from_effect(
                            game,
                            effect,
                            caster,
                            Some(source_id),
                        )
                    }));
                } else {
                    selected.extend(segment.default_effects.iter().flat_map(|effect| {
                        cast_time_selected_effects_from_effect(
                            game,
                            effect,
                            caster,
                            Some(source_id),
                        )
                    }));
                }
            }
            [branch, ..] => {
                if effects_have_new_cast_time_target_selection(&branch.replacement_effects)
                    || !effects_have_cast_time_target_selection(&segment.default_effects)
                {
                    selected.extend(branch.replacement_effects.iter().flat_map(|effect| {
                        cast_time_selected_effects_from_effect(
                            game,
                            effect,
                            caster,
                            Some(source_id),
                        )
                    }));
                } else {
                    selected.extend(segment.default_effects.iter().flat_map(|effect| {
                        cast_time_selected_effects_from_effect(
                            game,
                            effect,
                            caster,
                            Some(source_id),
                        )
                    }));
                }
            }
        }
    }

    selected
}

fn cast_time_selected_effects_from_effect(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Effect> {
    let Some(source_id) = source_id else {
        return vec![effect.clone()];
    };
    let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() else {
        return vec![effect.clone()];
    };
    if condition_depends_on_chosen_target(&conditional.condition) {
        return vec![effect.clone()];
    }

    let condition_result = crate::condition_eval::evaluate_condition_cast_time(
        game,
        &conditional.condition,
        caster,
        source_id,
    );
    let selected_branch = if condition_result {
        &conditional.if_true
    } else {
        &conditional.if_false
    };

    selected_branch
        .iter()
        .flat_map(|inner| {
            cast_time_selected_effects_from_effect(game, inner, caster, Some(source_id))
        })
        .collect()
}

fn condition_depends_on_chosen_target(condition: &crate::effect::Condition) -> bool {
    use crate::effect::Condition;

    matches!(
        condition,
        Condition::TargetIsTapped
            | Condition::TargetIsAttacking
            | Condition::TargetIsBlocked
            | Condition::TargetWasKicked
            | Condition::TargetSpellCastOrderThisTurn(_)
            | Condition::TargetSpellControllerIsPoisoned
            | Condition::TargetSpellManaSpentToCastAtLeast { .. }
            | Condition::YouControlMoreCreaturesThanTargetSpellController
            | Condition::TargetHasGreatestPowerAmongCreatures
            | Condition::TargetManaValueLteColorsSpentToCastThisSpell
    )
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

pub(crate) fn extract_target_requirements_from_program_with_modes(
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
pub(super) fn extract_target_requirements_with_modes(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
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

pub(crate) fn spell_has_legal_targets_with_modes_and_view(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
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
            chosen_modes,
            &mut consumed_modal_selection,
            &mut declared_targets,
            true,
            view,
        ) {
            return false;
        }
    }
    true
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
    attacking_player: Option<PlayerId>,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<Target> {
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
        | PlayerFilter::ControlsMost { .. } => {
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

fn stack_entry_contains_exchange_control(game: &GameState, entry: &StackEntry) -> bool {
    let effects = if let Some(effects) = &entry.ability_effects {
        effects.clone()
    } else if let Some(obj) = game.object(entry.object_id) {
        get_effects_for_stack_entry(game, entry, obj)
    } else {
        crate::resolution::ResolutionProgram::default()
    };

    effects
        .all_effects()
        .iter()
        .any(|effect| effect_contains_exchange_control(effect))
}

fn exchange_control_target_still_targetable(
    game: &GameState,
    entry: &StackEntry,
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

    let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent()));
    compute_legal_targets_with_source_snapshot_and_view(
        game,
        &spec,
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
}

pub(super) fn stack_entry_validation_target_specs(
    game: &GameState,
    entry: &StackEntry,
) -> Vec<ChooseSpec> {
    let effects = if let Some(effects) = &entry.ability_effects {
        effects.clone()
    } else if let Some(obj) = game.object(entry.object_id) {
        get_effects_for_stack_entry(game, entry, obj)
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
    specs
}

pub(super) fn validate_stack_entry_targets(
    game: &GameState,
    entry: &StackEntry,
) -> (
    Vec<ResolvedTarget>,
    Vec<crate::game_state::TargetAssignment>,
    bool,
) {
    let view = crate::derived_view::DerivedGameView::new(game);
    validate_stack_entry_targets_with_view(game, entry, &view)
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

pub(super) fn choose_spec_with_damaged_player_from_event(
    spec: &crate::target::ChooseSpec,
    event: Option<&TriggerEvent>,
) -> crate::target::ChooseSpec {
    let Some(player) = damaged_player_from_event(event) else {
        return spec.clone();
    };
    let mut spec = spec.clone();
    replace_damaged_player_choose_spec(&mut spec, player);
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

fn specialize_target_player_relation(filter: &mut crate::target::PlayerFilter, player: PlayerId) {
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            *filter = PlayerFilter::Specific(player);
        }
        PlayerFilter::Target(inner)
        | PlayerFilter::AliasedTarget(inner)
        | PlayerFilter::WasDealtDamageBySourceThisGame { base: inner }
        | PlayerFilter::LostLifeThisTurn { base: inner }
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { base: inner, .. }
        | PlayerFilter::HasMoreLifeThanYou { base: inner }
        | PlayerFilter::MaxSpeed { base: inner, .. } => {
            specialize_target_player_relation(inner, player);
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, .. } => {
            specialize_target_player_relation(base, player);
        }
        PlayerFilter::Excluding { base, excluded } => {
            specialize_target_player_relation(base, player);
            specialize_target_player_relation(excluded, player);
        }
        _ => {}
    }
}

fn specialize_target_player_relation_in_object_filter(
    filter: &mut crate::target::ObjectFilter,
    player: PlayerId,
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
        specialize_target_player_relation(player_filter, player);
    }
    if let Some(constraint) = &mut filter.counters_put_on_this_turn {
        specialize_target_player_relation(&mut constraint.source_controller, player);
    }
    for nested in &mut filter.any_of {
        specialize_target_player_relation_in_object_filter(nested, player);
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
        specialize_target_player_relation_in_object_filter(nested, player);
    }
}

fn specialize_target_player_relation_in_choose_spec(
    spec: &mut crate::target::ChooseSpec,
    player: PlayerId,
) {
    use crate::target::ChooseSpec;

    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            specialize_target_player_relation_in_choose_spec(spec, player);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            specialize_target_player_relation_in_object_filter(filter, player);
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            specialize_target_player_relation_in_object_filter(object_filter, player);
            specialize_target_player_relation(player_filter, player);
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            specialize_target_player_relation(filter, player);
        }
        _ => {}
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
                        .filter(|object| object.has_card_type(CardType::Planeswalker))
                        .and_then(|_| view.current_controller(*object)),
                })
        })
}

pub(super) fn validate_stack_entry_targets_with_view(
    game: &GameState,
    entry: &StackEntry,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> (
    Vec<ResolvedTarget>,
    Vec<crate::game_state::TargetAssignment>,
    bool,
) {
    if entry.targets.is_empty() {
        return (Vec::new(), Vec::new(), false);
    }

    if !entry.target_assignments.is_empty() {
        let mut valid_targets = Vec::new();
        let mut valid_assignments = Vec::with_capacity(entry.target_assignments.len());
        let mut invalid_count = 0usize;
        let contains_exchange_control = stack_entry_contains_exchange_control(game, entry);

        for (assignment_index, assignment) in entry.target_assignments.iter().enumerate() {
            let resolved_spec = choose_spec_with_damaged_player_from_event(
                &assignment.spec,
                entry.triggering_event.as_ref(),
            );
            let mut resolved_spec = choose_spec_for_resolution_target_validation(&resolved_spec);
            if let Some(player) =
                prior_player_or_planeswalker_target(game, entry, assignment_index, view)
            {
                specialize_target_player_relation_in_choose_spec(&mut resolved_spec, player);
            }
            let legal_targets = compute_legal_targets_with_source_snapshot_and_view(
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
            );
            let legal_targets = if entry.defending_player.is_some() {
                compute_legal_targets_with_tagged_objects_combat_context_and_view(
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
                    entry.defending_player,
                    combat_attacking_player_for_entry(game, entry),
                    view,
                )
            } else {
                legal_targets
            };

            let start = valid_targets.len();
            for target in &entry.targets[assignment.range.clone()] {
                if legal_targets.contains(target)
                    || (contains_exchange_control
                        && exchange_control_target_still_targetable(game, entry, target, view))
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
        return (valid_targets, valid_assignments, all_invalid);
    }

    let validation_specs = stack_entry_validation_target_specs(game, entry);
    let legal_target_sets: Vec<Vec<Target>> = validation_specs
        .iter()
        .map(|spec| {
            let resolved_spec =
                choose_spec_with_damaged_player_from_event(spec, entry.triggering_event.as_ref());
            let resolved_spec = choose_spec_for_resolution_target_validation(&resolved_spec);
            if entry.defending_player.is_some() {
                return compute_legal_targets_with_tagged_objects_combat_context_and_view(
                    game,
                    &resolved_spec,
                    entry.controller,
                    Some(entry.object_id),
                    entry.source_snapshot.as_ref(),
                    None,
                    entry.defending_player,
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
    (valid_targets, Vec::new(), all_invalid)
}
