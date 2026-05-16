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
pub fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        // Target wrapper - check the inner spec
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            requires_target_selection(inner)
        }
        // These require target selection during casting
        ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::Player(_)
        | ChooseSpec::Object(_) => true,
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

    if let Some(cast) = event.downcast::<crate::events::spells::SpellCastEvent>() {
        game.consume_temporary_spell_ability_grants_for_spell(cast.spell, cast.caster);
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

pub(super) fn target_events_from_targets(
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
    provenance: ProvNodeId,
) -> Vec<TriggerEvent> {
    targets
        .iter()
        .filter_map(|target| {
            let Target::Object(target_id) = target else {
                return None;
            };
            Some(TriggerEvent::new_with_provenance(
                BecomesTargetedEvent::new(*target_id, source, source_controller, by_ability),
                provenance,
            ))
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
                game.controller_of(obj) != committer
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
    let event_provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::AbilityActivated);
    let event = TriggerEvent::new_with_provenance(
        AbilityActivatedEvent::new(source, activator, is_mana_ability).with_snapshot(snapshot),
        event_provenance,
    );
    queue_triggers_from_event(game, trigger_queue, event, true);
    if is_mana_ability {
        resolve_triggered_mana_abilities_with_dm(game, trigger_queue, decision_maker);
    }
}

pub(super) fn queue_mana_ability_event_for_action(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
    action: &ManaPipPaymentAction,
    activator: PlayerId,
) {
    if let ManaPipPaymentAction::ActivateManaAbility { source_id, .. } = action {
        queue_ability_activated_event(
            game,
            trigger_queue,
            decision_maker,
            *source_id,
            activator,
            true,
            None,
        );
    }
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
    action: &ManaPipPaymentAction,
) {
    let ManaPipPaymentAction::PayViaAlternative {
        permanent_id,
        effect,
    } = action
    else {
        return;
    };

    let contribution = KeywordPaymentContribution {
        permanent_id: *permanent_id,
        effect: *effect,
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
fn simultaneous_sba_ltb_batch_events(pending_events: &[TriggerEvent]) -> Vec<TriggerEvent> {
    use crate::events::cause::CauseType;
    use crate::events::zones::ZoneChangeEvent;

    let mut batch_events: Vec<TriggerEvent> = Vec::new();

    for event in pending_events {
        let Some(zone_change) = event.downcast::<ZoneChangeEvent>() else {
            continue;
        };
        if zone_change.from != crate::zone::Zone::Battlefield
            || zone_change.to == crate::zone::Zone::Battlefield
            || zone_change.cause.cause_type != CauseType::StateBasedAction
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
        let mut merged_event = TriggerEvent::new_with_provenance(merged_zone_change, provenance);
        if let Some(source_snapshot) = batch_events[index].source_snapshot().cloned() {
            merged_event = merged_event.with_source_snapshot(source_snapshot);
        }
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
    let mut one_or_more_zone_changes_seen = HashSet::new();
    loop {
        let pending_events = game.take_pending_trigger_events();
        if pending_events.is_empty() {
            break;
        }
        let batch_lki_events = simultaneous_sba_ltb_batch_events(&pending_events);
        for event in pending_events {
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
    if let Some(exchange) = effect.downcast_ref::<crate::effects::ExchangeControlEffect>() {
        if exchange.permanent1 != exchange.permanent2 {
            return Some((exchange.permanent1.clone(), exchange.permanent2.clone()));
        }
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
        ChooseSpec::WithCount(inner, count) => ChooseSpec::WithCount(
            Box::new(relaxed_exchange_later_target_spec(inner)),
            count.clone(),
        ),
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(
            Box::new(relaxed_exchange_later_target_spec(inner)),
            count.clone(),
            value.clone(),
        ),
        _ => spec.clone(),
    }
}

#[derive(Clone)]
pub(super) struct DeclaredTarget {
    spec: ChooseSpec,
}

fn declare_target(profile: &ExtractedTarget<'_>, declared: &mut Vec<DeclaredTarget>) {
    if profile.reuse_policy == crate::effects::TargetReusePolicy::AlwaysDeclareNew
        || !declared
            .iter()
            .any(|declared| target_spec_reuses_declared_target(profile.spec, &declared.spec))
    {
        declared.push(DeclaredTarget {
            spec: profile.spec.clone(),
        });
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

fn target_spec_reuses_declared_target(candidate: &ChooseSpec, declared: &ChooseSpec) -> bool {
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
    declared: &[DeclaredTarget],
) -> bool {
    profile.reuse_policy != crate::effects::TargetReusePolicy::AlwaysDeclareNew
        && declared
            .iter()
            .any(|declared| target_spec_reuses_declared_target(profile.spec, &declared.spec))
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

        for mode_idx in chosen_modes {
            let Some(mode) = modal.modes.get(*mode_idx) else {
                return false;
            };

            if !modal.allow_repeated_modes && !seen_modes.insert(*mode_idx) {
                return false;
            }

            let mut mode_consumed_modal_selection = false;
            if !mode.effects.iter().all(|effect| {
                spell_effect_has_legal_targets_internal_with_preview_mode_selection(
                    game,
                    effect,
                    caster,
                    source_id,
                    None,
                    &mut mode_consumed_modal_selection,
                    declared_targets,
                    require_full_selection,
                    view,
                )
            }) {
                return false;
            };
            selected_count += 1;
        }

        return if require_full_selection {
            selected_count >= min_modes && selected_count <= max_modes
        } else {
            selected_count <= max_modes
        };
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
        // For "any number" effects, we can cast even with no legal targets.
        if extracted.min_targets == 0 {
            return true;
        }
        let legal_targets = crate::targeting::compute_legal_targets_with_tagged_objects_with_view(
            game,
            extracted.spec,
            caster,
            source_id,
            None,
            view,
        );
        return legal_targets.len() >= extracted.min_targets;
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
            for mode_idx in chosen_modes {
                if let Some(mode) = modal.modes.get(*mode_idx) {
                    for inner in &mode.effects {
                        extract_target_requirements_from_effect_internal(
                            game,
                            inner,
                            caster,
                            source_id,
                            None,
                            consumed_modal_selection,
                            declared_targets,
                            requirements,
                        );
                    }
                }
            }
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
                description: "target",
                min_targets: 1,
                max_targets: Some(1),
                count_value: None,
                reuse_policy: crate::effects::TargetReusePolicy::AlwaysDeclareNew,
            };
            declare_target(&profile, declared_targets);
            let legal_targets = compute_legal_targets(game, &spec, caster, source_id);
            if !legal_targets.is_empty() {
                requirements.push(TargetRequirement {
                    spec,
                    legal_targets,
                    description: "target".to_string(),
                    min_targets: 1,
                    max_targets: Some(1),
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
        let legal_targets = compute_legal_targets(game, extracted.spec, caster, source_id);
        let (min_targets, max_targets) =
            resolved_target_bounds(game, &extracted, caster, source_id);
        // For "any number" effects (min_targets == 0), we can cast even with no legal targets.
        // For required targets (min_targets > 0), we need at least min_targets legal targets.
        let has_enough_targets = min_targets == 0 || legal_targets.len() >= min_targets;
        if has_enough_targets {
            requirements.push(TargetRequirement {
                spec: extracted.spec.clone(),
                legal_targets,
                description: extracted.description.to_string(),
                min_targets,
                max_targets,
            });
        }
    }
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
            description: extracted.description,
            min_targets: extracted.min_targets,
            max_targets: extracted.max_targets,
            count_value: extracted.count_value,
            reuse_policy: extracted.reuse_policy,
        };
        if profile_reuses_declared_target(&profile, declared_targets) {
            return;
        }
        declare_target(&profile, declared_targets);
        let legal_targets = compute_legal_targets(game, &spec, caster, source_id);
        let (min_targets, max_targets) = resolved_target_bounds(game, &profile, caster, source_id);
        let has_enough_targets = min_targets == 0 || legal_targets.len() >= min_targets;
        if has_enough_targets {
            requirements.push(TargetRequirement {
                spec,
                legal_targets,
                description: extracted.description.to_string(),
                min_targets,
                max_targets,
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

fn specialize_iterated_player_choose_spec(spec: &ChooseSpec, player: PlayerId) -> ChooseSpec {
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
            count.clone(),
        ),
        ChooseSpec::WithCountValue(inner, count, value) => ChooseSpec::WithCountValue(
            Box::new(specialize_iterated_player_choose_spec(inner, player)),
            count.clone(),
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
    filter.attached_to_player = filter
        .attached_to_player
        .as_ref()
        .map(|attached_to_player| specialize_iterated_player_filter(attached_to_player, player));
    filter.entered_battlefield_controller = filter
        .entered_battlefield_controller
        .as_ref()
        .map(|controller| specialize_iterated_player_filter(controller, player));
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
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            PlayerFilter::CardsInHandAtLeastMoreThanYou {
                base: Box::new(specialize_iterated_player_filter(base, player)),
                count: *count,
            }
        }
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
    if let Some(modal) = effect.modal_effect_spec() {
        let modes_for_this_modal = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };

        return modes_for_this_modal
            .into_iter()
            .flatten()
            .filter_map(|mode_idx| modal.modes.get(*mode_idx))
            .map(|mode| {
                mode.effects
                    .iter()
                    .map(|inner| {
                        count_target_selection_slots_from_effect_internal(
                            inner,
                            None,
                            consumed_modal_selection,
                            declared_targets,
                        )
                    })
                    .sum::<usize>()
            })
            .sum();
    }

    if let Some((first, second)) = exchange_control_target_specs(effect) {
        let mut count = 0;
        for spec in [first, second] {
            if !requires_target_selection(&spec) {
                continue;
            }
            let profile = crate::effects::TargetSelectionProfile {
                spec: &spec,
                description: "target",
                min_targets: 1,
                max_targets: Some(1),
                count_value: None,
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
            [] => selected.extend(segment.default_effects.iter().cloned()),
            [branch] => {
                if effects_have_new_cast_time_target_selection(&branch.replacement_effects)
                    || !effects_have_cast_time_target_selection(&segment.default_effects)
                {
                    selected.extend(branch.replacement_effects.iter().cloned());
                } else {
                    selected.extend(segment.default_effects.iter().cloned());
                }
            }
            [branch, ..] => {
                if effects_have_new_cast_time_target_selection(&branch.replacement_effects)
                    || !effects_have_cast_time_target_selection(&segment.default_effects)
                {
                    selected.extend(branch.replacement_effects.iter().cloned());
                } else {
                    selected.extend(segment.default_effects.iter().cloned());
                }
            }
        }
    }

    selected
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
        PlayerFilter::Target(inner) => player_filter_references_previous_target_tag(inner),
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
        PlayerFilter::Opponent => player_id != controller,
        PlayerFilter::Active => game.turn.active_player == player_id,
        PlayerFilter::Teammate => false, // In 2-player games, no teammates
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
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, count } => {
            if !player_matches_filter_with_combat(player_id, base, game, controller, combat) {
                return false;
            }
            let candidate_hand = game.player(player_id).map(|p| p.hand.len()).unwrap_or(0);
            let your_hand = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
            candidate_hand >= your_hand.saturating_add(*count as usize)
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
        PlayerFilter::Target(_) => {
            // Target filters are resolved through targeting, not direct matching
            true
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_matches_filter_with_combat(player_id, base, game, controller, combat)
                && !player_matches_filter_with_combat(player_id, excluded, game, controller, combat)
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
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            replace_damaged_player_filter(filter, player);
        }
        _ => {}
    }
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

        for assignment in &entry.target_assignments {
            let resolved_spec = choose_spec_with_damaged_player_from_event(
                &assignment.spec,
                entry.triggering_event.as_ref(),
            );
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
                Target::Object(obj_id) => game
                    .object(*obj_id)
                    .is_some_and(|obj| obj.zone == Zone::Battlefield || obj.zone == Zone::Stack),
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
