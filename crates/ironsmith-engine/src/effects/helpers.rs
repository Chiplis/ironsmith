//! Shared helper functions for effect execution.
//!
//! This module contains utility functions used by multiple effect implementations:
//! - Value resolution (X, counts, power/toughness, etc.)
//! - Player filter resolution
//! - Target finding and validation

use crate::filter::{ObjectFilterExt as _, player_filter_matches_game};
use std::collections::{HashMap, HashSet};

use crate::cost::OptionalCostsPaid;
use crate::decisions::context::ViewCardsContext;
use crate::decisions::{make_decision, specs::ChooseObjectsSpec};
use crate::effect::{
    EffectMetric, EffectMetricSource, EffectOutcome, EventValueSpec, OutcomeStatus,
    PriorEffectMetricQuery, Value,
};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
use crate::events::life::LifeGainEvent;
use crate::events::life::LifeLossEvent;
use crate::events::other::{CounterPlacedEvent, KeywordActionEvent, MarkersChangedEvent};
use crate::events::zones::ZoneChangeEvent;
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object_query::candidate_ids_for_filter;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, FilterContext, ObjectRef, PlayerFilter};
use crate::triggers::AttackEventTarget;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) mod value_eval;
pub(crate) use value_eval::resolve_damage_history_for_comparison;

#[cfg(test)]
#[path = "helpers/original_destination_tests.rs"]
mod original_destination_tests;
#[cfg(test)]
#[path = "helpers/source_number_tests.rs"]
mod source_number_tests;

// ============================================================================
// Tagged Object Resolution
// ============================================================================

/// Emit card-view events for hidden-zone objects before their identities are
/// exposed through a decision prompt or temporary inspection window.
pub(crate) fn view_hidden_candidate_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    viewer: PlayerId,
    candidates: &[ObjectId],
    description: impl Into<String>,
    public: bool,
) {
    let description = description.into();
    let entitled_viewers = game.private_information_viewers_for(viewer, Zone::Library);
    game.hydrate_verified_library_replay_view(candidates, &entitled_viewers, public);
    let already_publicly_revealed = if public {
        ctx.get_tagged_all(crate::effects::PUBLIC_REVEALED_TAG)
            .map(|snapshots| {
                snapshots
                    .iter()
                    .map(|snapshot| snapshot.object_id)
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default()
    } else {
        HashSet::new()
    };
    // Ordered map: view/crypto requirement order must match on every peer.
    let mut grouped: std::collections::BTreeMap<(PlayerId, Zone), Vec<ObjectId>> =
        std::collections::BTreeMap::new();
    for &id in candidates {
        if already_publicly_revealed.contains(&id) {
            continue;
        }
        let Some(object) = game.object(id) else {
            continue;
        };
        if !object.zone.is_hidden() && game.hidden_card_info(id).is_none() {
            continue;
        }
        grouped
            .entry((object.owner, object.zone))
            .or_default()
            .push(id);
    }

    for ((subject, zone), cards) in grouped {
        if cards.is_empty() {
            continue;
        }
        if public {
            for viewer_idx in 0..game.players.len() {
                let public_viewer = PlayerId::from_index(viewer_idx as u8);
                let view_ctx = ViewCardsContext::new(
                    public_viewer,
                    subject,
                    Some(ctx.source),
                    zone,
                    description.clone(),
                )
                .with_public(true);
                ctx.decision_maker
                    .view_cards(game, public_viewer, &cards, &view_ctx);
            }
            for card_id in cards {
                if let Some(object) = game.object(card_id) {
                    ctx.tag_object(
                        crate::effects::PUBLIC_REVEALED_TAG,
                        ObjectSnapshot::from_object(object, game),
                    );
                }
            }
        } else {
            for entitled_viewer in game.private_information_viewers_for(viewer, zone) {
                let view_ctx = ViewCardsContext::new(
                    entitled_viewer,
                    subject,
                    Some(ctx.source),
                    zone,
                    description.clone(),
                );
                ctx.decision_maker
                    .view_cards(game, entitled_viewer, &cards, &view_ctx);
            }
            // Persist the rules player's entitlement, not the controller's
            // temporary derived access. CR 722.4 makes that access follow the
            // current player-control effect.
            ctx.remember_face_down_exile_viewers(&cards, viewer);
        }
    }
}

/// Resolve the current `ObjectId` for a tagged snapshot, following through
/// zone changes via `stable_id` when the snapshot's `object_id` is stale.
///
/// Zone changes create a new `ObjectId` (Magic rule 400.7). Tag snapshots
/// captured before the move still carry the old id. This helper tries the
/// snapshot's `object_id` first; when that object no longer exists it falls
/// back to `stable_id`, but only where the resolving instruction may find
/// the new object (see [`tagged_object_follow_permitted`]).
///
/// Use this only for effects that need to **physically locate** an object in
/// order to move it (e.g. `MoveToZoneEffect`). Effects that read
/// characteristics from a tag should use the snapshot's `object_id` directly
/// to preserve last-known-information semantics.
pub(crate) fn resolve_tagged_object_id(
    game: &GameState,
    ctx: &ExecutionContext,
    snapshot: &ObjectSnapshot,
) -> Option<ObjectId> {
    // A tag naming an ability on the stack by its own stack id ("that spell
    // or ability") keeps naming that entry, not the source's current object.
    if game.stack_ability_entry(snapshot.object_id).is_some() {
        return Some(snapshot.object_id);
    }
    // Zone changes create a fresh ObjectId while retaining the stable
    // identity. Prefer that indexed current object when a tag snapshot is
    // stale and this resolution may find it; the old object record may
    // remain available for LKI queries.
    if let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id)
        && current_id != snapshot.object_id
        && tagged_object_follow_permitted(ctx, snapshot, current_id)
    {
        return Some(current_id);
    }
    game.object(snapshot.object_id).map(|_| snapshot.object_id)
}

/// Context-free variant of [`resolve_tagged_object_id`] for value readers
/// that only project a tagged snapshot's current zone.
pub(crate) fn resolve_tagged_object_id_unscoped(
    game: &GameState,
    snapshot: &ObjectSnapshot,
) -> Option<ObjectId> {
    if game.stack_ability_entry(snapshot.object_id).is_some() {
        return Some(snapshot.object_id);
    }
    if let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id)
        && current_id != snapshot.object_id
    {
        return Some(current_id);
    }
    if game.object(snapshot.object_id).is_some() {
        return Some(snapshot.object_id);
    }
    game.find_object_by_stable_id(snapshot.stable_id)
}

/// Whether a resolving instruction may treat `current_id` — the object the
/// tagged card is now — as the object `snapshot` recorded.
///
/// CR 400.7: an object that changes zones becomes a new object. The
/// exceptions a resolving spell or ability relies on are:
/// - it moved the object itself during this resolution (CR 400.7j);
/// - it triggered on that very move and looks in the zone the object went
///   to (CR 400.7e, 603.6c, 603.10);
/// - its cost moved the object (CR 400.7j). Costs are paid before the spell
///   or ability is put on the stack, and `push_to_stack` re-points its tags at
///   the objects those payments created, so no following is needed later.
///
/// Everything else — a delayed trigger's object (CR 603.7c), a triggered
/// ability's object that moved again while it waited on the stack, an
/// object that left and returned — is a new object the ability can't find.
pub(crate) fn tagged_object_follow_permitted(
    ctx: &ExecutionContext,
    snapshot: &ObjectSnapshot,
    current_id: ObjectId,
) -> bool {
    if let Some(discard) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::other::CardDiscardedEvent>())
    {
        if ctx
            .resolution_object_id_floor
            .is_some_and(|floor| current_id.0 >= floor.0)
        {
            return true;
        }
        return discard.destinations.iter().any(|receipt| {
            receipt.object == Some(current_id)
                && (receipt.card == snapshot.object_id
                    || receipt.object == Some(snapshot.object_id))
        });
    }
    if ctx.triggering_event.as_ref().is_some_and(|event| {
        matches!(
            event.kind(),
            crate::events::EventKind::Transformed
                | crate::events::EventKind::Mutated
                | crate::events::EventKind::TurnedFaceUp
        ) || event
            .downcast::<crate::events::KeywordActionEvent>()
            .is_some_and(|action| action.action == crate::events::KeywordActionKind::Renown)
    }) {
        // A status/characteristic change did not move its participant. Only
        // an explicit move made by this resolution can introduce a successor.
        return current_id == snapshot.object_id
            || ctx
                .resolution_object_id_floor
                .is_some_and(|floor| current_id.0 >= floor.0);
    }
    let Some(floor) = ctx.resolution_object_id_floor else {
        return true;
    };
    if current_id.0 >= floor.0 {
        return true;
    }
    // A spell or activated ability: its cost moves were pinned when it was
    // put on the stack, so a card that moved since is a new object.
    let Some(event) = ctx.triggering_event.as_ref() else {
        return false;
    };
    // An "enters" trigger names the permanent that entered; one that has
    // left the battlefield since is a new object (CR 400.7).
    if let Some(entered) = event.downcast::<crate::events::EnterBattlefieldEvent>() {
        return current_id == entered.object;
    }
    if let Some(zone_change) = event.downcast::<crate::events::ZoneChangeEvent>() {
        let names_moved_object = zone_change.objects.contains(&snapshot.object_id)
            || zone_change.result_objects.contains(&snapshot.object_id)
            || zone_change
                .snapshots
                .iter()
                .chain(zone_change.snapshot.iter())
                .any(|moved| moved.stable_id == snapshot.stable_id);
        if !names_moved_object {
            return false;
        }
        // Only the object the triggering move created, in the zone it went
        // to first. A card that has since moved again is a new object.
        return if zone_change.result_objects.is_empty() {
            zone_change.objects.contains(&current_id)
        } else {
            zone_change.result_objects.contains(&current_id)
        };
    }
    // Combat, tap-state, attachment and phasing observations do not move
    // their participants to new zones. Merely naming an attacker/blocker is
    // no permission to follow a blink that happened before resolution.
    // Genuine in-resolution moves still use the floor permission above.
    if matches!(
        event.kind(),
        crate::events::EventKind::CreatureAttacked
            | crate::events::EventKind::CreatureAttackedAndUnblocked
            | crate::events::EventKind::CreatureBlocked
            | crate::events::EventKind::CreatureBecameBlocked
            | crate::events::EventKind::PermanentTapped
            | crate::events::EventKind::PermanentUntapped
            | crate::events::EventKind::ObjectBecameAttached
            | crate::events::EventKind::ObjectBecameUnattached
            | crate::events::EventKind::PermanentPhasedIn
            | crate::events::EventKind::PermanentPhasedOut
    ) {
        return false;
    }
    // Other legacy movement events (for example sacrifice) may lack an
    // explicit destination receipt. Discard uses its exact receipt above.
    event.object_id() == Some(snapshot.object_id)
        || event
            .inner()
            .snapshots()
            .iter()
            .any(|named| named.stable_id == snapshot.stable_id)
}

/// An `IsTaggedObject` filter constraint matches the tagged card by stable id
/// too. Under a resolution, a card the tag names only by stable id (it has
/// moved since) matches only where `tagged_object_follow_permitted` allows the
/// resolving instruction to follow it (CR 400.7).
fn tagged_constraints_follow_permitted(
    ctx: &ExecutionContext,
    filter: &crate::filter::ObjectFilter,
    id: ObjectId,
    object: &crate::object::Object,
) -> bool {
    if ctx.resolution_object_id_floor.is_none() {
        return true;
    }
    filter.tagged_constraints.iter().all(|constraint| {
        if !matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                | crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
        ) {
            return true;
        }
        let Some(snapshots) = ctx.get_tagged_all(constraint.tag.as_str()) else {
            return true;
        };
        snapshots.iter().any(|snapshot| snapshot.object_id == id)
            || snapshots.iter().any(|snapshot| {
                snapshot.stable_id == object.stable_id
                    && tagged_object_follow_permitted(ctx, snapshot, id)
            })
            || !snapshots
                .iter()
                .any(|snapshot| snapshot.stable_id == object.stable_id)
    })
}

/// Re-point tagged snapshots at the objects their cards are now, where the
/// current resolution may find them, before those tags outlive it (a delayed
/// or reflexive triggered ability). Snapshots it may not follow keep their
/// recorded object, which a later resolution won't find either
/// (CR 603.7c, 400.7).
pub(crate) fn pin_tagged_objects_to_current(
    game: &GameState,
    ctx: &ExecutionContext,
    tagged_objects: &mut HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
) {
    for (tag, snapshots) in tagged_objects.iter_mut() {
        if tag.as_str().starts_with("__paid_departure__")
            || matches!(
                ironsmith_core::tag::SacrificeCostTag::parse(tag),
                Some(ironsmith_core::tag::SacrificeCostTag::OriginalResult(_))
            )
            || tag.as_str().starts_with("__pre_move_history__")
        {
            continue;
        }
        for snapshot in snapshots.iter_mut() {
            if let Some(current) = game.find_object_by_stable_id(snapshot.stable_id)
                && current != snapshot.object_id
                && tagged_object_follow_permitted(ctx, snapshot, current)
                && let Some(object) = game.object(current)
            {
                snapshot.object_id = current;
                snapshot.zone = object.zone;
            }
        }
    }
}

/// CR400.7f authorizes only the Aura's first actual battlefield-to-graveyard
/// receipt after its enchanted permanent left. A physical-card lookup cannot
/// distinguish that object from a later exile/return incarnation.
fn aura_source_graveyard_incarnation(
    game: &GameState,
    ctx: &ExecutionContext,
    trigger: &crate::events::ZoneChangeEvent,
) -> Option<ObjectId> {
    if trigger.from != Zone::Battlefield {
        return None;
    }
    let attached_sources = trigger.object_tags.get("attached_source");
    let snapshot = ctx
        .source_snapshot
        .as_ref()
        .filter(|snapshot| snapshot.object_id == ctx.source)
        .or_else(|| {
            attached_sources.and_then(|sources| {
                sources
                    .iter()
                    .find(|snapshot| snapshot.object_id == ctx.source)
            })
        })?;
    if snapshot.zone != Zone::Battlefield
        || !snapshot.subtypes.contains(&crate::types::Subtype::Aura)
    {
        return None;
    }
    let was_attached = snapshot
        .attached_to
        .as_ref()
        .and_then(|target| target.object_id())
        .is_some_and(|host| trigger.objects.contains(&host))
        || attached_sources.is_some_and(|sources| {
            sources.iter().any(|source| {
                source.object_id == ctx.source && source.stable_id == snapshot.stable_id
            })
        });
    if !was_attached {
        return None;
    }
    let Some(transition) = game
        .turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .filter_map(|record| record.event.downcast::<crate::events::ZoneChangeEvent>())
        .find(|event| event.from == Zone::Battlefield && event.objects.contains(&ctx.source))
    else {
        game.record_token_resource_failure(&ExecutionError::IncompleteEvidence(
            "attached Aura return requires its exact first battlefield departure receipt".into(),
        ));
        return None;
    };
    if transition.to != Zone::Graveyard
        || transition.cause.cause_type != crate::events::cause::CauseType::StateBasedAction
    {
        return None;
    }
    if transition.result_objects.is_empty() {
        game.record_token_resource_failure(&ExecutionError::IncompleteEvidence(
            "attached Aura return requires its exact graveyard destination mapping".into(),
        ));
        return None;
    }
    let destination = transition.result_objects.iter().copied().find(|id| {
        game.object(*id).is_some_and(|object| {
            object.zone == Zone::Graveyard
                && object.owner == snapshot.owner
                && object.stable_id == snapshot.stable_id
        })
    });
    if destination.is_none()
        && (transition
            .result_objects
            .iter()
            .all(|id| game.object(*id).is_some())
            || transition
                .result_objects
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|object| {
                    object.stable_id == snapshot.stable_id && object.owner != snapshot.owner
                }))
    {
        game.record_token_resource_failure(&ExecutionError::IncompleteEvidence(
            "attached Aura destination mapping contradicts its retained identity or owner".into(),
        ));
    }
    // A recorded successor that no longer exists changed incarnation; that
    // is a complete no-return result, unlike a contradictory live mapping.
    destination
}

pub(crate) fn resolve_source_object_id(
    game: &GameState,
    ctx: &ExecutionContext,
) -> Option<ObjectId> {
    if game.object(ctx.source).is_some() {
        return Some(ctx.source);
    }
    // Self-discard triggers can find exactly the public arrival made by
    // that discard, including when scheduling a later return. Never follow a
    // card that left that arrival before the registration resolved.
    if let Some(discard) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::other::CardDiscardedEvent>())
        && discard.card == ctx.source
    {
        return discard
            .destination(ctx.source)
            .filter(|receipt| receipt.zone.is_public())
            .and_then(|receipt| {
                receipt.object.filter(|id| {
                    game.object(*id).is_some_and(|object| {
                        object.zone == receipt.zone
                            && discard
                                .snapshot
                                .as_ref()
                                .is_none_or(|origin| origin.stable_id == object.stable_id)
                    })
                })
            });
    }
    // A zone-change trigger may refer to the new object created by that
    // transition. Its recorded destination identity is authoritative: after
    // another zone change, following the stable card would affect a new object.
    if let Some(event) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::ZoneChangeEvent>())
        && event.objects.contains(&ctx.source)
        && !event.result_objects.is_empty()
    {
        let source_snapshot = ctx.source_snapshot.as_ref().or_else(|| {
            event
                .snapshots
                .iter()
                .find(|snapshot| snapshot.object_id == ctx.source)
        });
        return event.result_objects.iter().copied().find(|id| {
            game.object(*id).is_some_and(|object| {
                object.zone == event.to
                    && source_snapshot
                        .is_some_and(|snapshot| snapshot.stable_id == object.stable_id)
            })
        });
    }
    // For moves that don't start on the battlefield ("When this enters",
    // "when this is put into a graveyard from your library") the event
    // records the destination object itself. If that object is gone it has
    // changed zones again, and the card in its new zone is a new object
    // (CR 400.7): Jadelight Ranger flickered in response to its ETB trigger
    // explores with last-known information, not as the new permanent.
    if let Some(event) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::ZoneChangeEvent>())
        && event.result_objects.is_empty()
        && event.objects.contains(&ctx.source)
    {
        return None;
    }
    if let Some(event) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::ZoneChangeEvent>())
    {
        // Preserve the existing in-resolution movement permission used by
        // tagged references (400.7j). Cost moves were pinned at stack entry;
        // anything created before this resolution cannot use this exception.
        if let Some(floor) = ctx.resolution_object_id_floor
            && let Some(snapshot) = ctx.source_snapshot.as_ref()
            && let Some(current) = game.find_object_by_stable_id(snapshot.stable_id)
            && current.0 >= floor.0
        {
            return Some(current);
        }
        // Source movement in the same event was handled above (400.7e).
        // A different dying object only authorizes the exact Aura/SBA case;
        // never follow arbitrary pre-resolution moves by stable card identity.
        return aura_source_graveyard_incarnation(game, ctx, event);
    }
    // Non-zone-change triggers retain the object that owned the ability.
    // A later independent zone change does not authorize following the same
    // physical card. Explicit moves within a resolution update ctx.source;
    // zone-change triggers use their recorded destination above.
    if ctx
        .triggering_event
        .as_ref()
        .is_some_and(|event| event.downcast::<crate::events::ZoneChangeEvent>().is_none())
    {
        return None;
    }
    ctx.source_snapshot
        .as_ref()
        .and_then(|snapshot| game.find_object_by_stable_id(snapshot.stable_id))
        .filter(|id| {
            ctx.resolution_object_id_floor.map_or_else(
                || {
                    ctx.triggering_event.is_none()
                        && game.object(*id).is_some_and(|object| {
                            matches!(object.zone, Zone::Graveyard | Zone::Exile)
                        })
                },
                |floor| id.0 >= floor.0,
            )
        })
        .or(Some(ctx.source))
}

fn resolve_tagged_players_from_context(
    game: &GameState,
    ctx: &ExecutionContext,
    tag: &crate::tag::TagKey,
) -> Option<Vec<PlayerId>> {
    ctx.get_tagged_players(tag.as_str())
        .cloned()
        .or_else(|| ctx.filter_context(game).tagged_players.get(tag).cloned())
}

// ============================================================================
// Value Resolution
// ============================================================================

/// Get the optional costs paid, preferring context but falling back to source object.
/// This allows ETB triggers to access kick count etc. from the permanent that entered.
pub fn get_optional_costs_paid<'a>(
    game: &'a GameState,
    ctx: &'a ExecutionContext,
) -> &'a OptionalCostsPaid {
    // If context has costs tracked, use those (for spell resolution)
    if !ctx.optional_costs_paid.costs.is_empty()
        || ctx.optional_costs_paid.cast_payment_turn.is_some()
    {
        return &ctx.optional_costs_paid;
    }
    // Otherwise, try to get from the source object (for ETB triggers)
    if let Some(source) = game.object(ctx.source) {
        return &source.optional_costs_paid;
    }
    if let Some(snapshot) = ctx
        .source_snapshot
        .as_ref()
        .filter(|snapshot| snapshot.object_id == ctx.source)
    {
        return &snapshot.optional_costs_paid;
    }
    // Fallback to context (empty)
    &ctx.optional_costs_paid
}

fn effect_metric_memory(
    _game: &GameState,
    outcome: &EffectOutcome,
    source: EffectMetricSource,
) -> Vec<ObjectSnapshot> {
    let outcome = outcome.instruction_result();
    match source {
        EffectMetricSource::AffectedObjects => outcome
            .affected_object_memory()
            .unwrap_or_default()
            .to_vec(),
        EffectMetricSource::ChosenObjects => {
            outcome.chosen_object_memory().unwrap_or_default().to_vec()
        }
        EffectMetricSource::Outcome => outcome
            .result_object_memory()
            .or_else(|| outcome.chosen_object_memory())
            .or_else(|| outcome.affected_object_memory())
            .unwrap_or_default()
            .to_vec(),
    }
}

fn effect_metric_object_count(
    game: &GameState,
    outcome: &EffectOutcome,
    source: EffectMetricSource,
) -> i64 {
    match source {
        EffectMetricSource::Outcome => outcome.as_count().unwrap_or_else(|| {
            let memory = effect_metric_memory(game, outcome, EffectMetricSource::Outcome);
            if !memory.is_empty() {
                memory.len() as i64
            } else {
                outcome.output_objects().len() as i64
            }
        }),
        EffectMetricSource::ChosenObjects => outcome
            .chosen_object_memory()
            .map(|memory| memory.len() as i64)
            .or_else(|| outcome.chosen_objects().map(|ids| ids.len() as i64))
            .unwrap_or(0),
        EffectMetricSource::AffectedObjects => outcome
            .affected_object_memory()
            .map(|memory| memory.len() as i64)
            .or_else(|| outcome.affected_objects().map(|ids| ids.len() as i64))
            .unwrap_or(0),
    }
}

fn resolve_effect_metric(
    game: &GameState,
    ctx: &ExecutionContext,
    effect_id: crate::effect::EffectId,
    source: EffectMetricSource,
    metric: EffectMetric,
) -> Result<i64, ExecutionError> {
    if effect_id == crate::effect::EffectId::ACTIVATION_COUNTER_COST
        && ctx.get_outcome(effect_id).is_none()
    {
        return Err(ExecutionError::IncompleteEvidence(
            "activation counter payment has no completed receipt".into(),
        ));
    }
    // "the other result" of a roll-and-choose die roll (Wild Endeavor) is
    // recorded on the roll itself. When the bound producer is a later
    // instruction, read the nearest earlier roll that recorded one.
    if matches!(metric, EffectMetric::OtherNumber) {
        let other_number = |id: crate::effect::EffectId| {
            ctx.get_outcome(id).and_then(|outcome| {
                outcome.execution_facts.iter().find_map(|fact| match fact {
                    crate::effect::ExecutionFact::OtherNumber(value) => Some(*value as i64),
                    _ => None,
                })
            })
        };
        let found = (0..=effect_id.0)
            .rev()
            .find_map(|id| other_number(crate::effect::EffectId(id)));
        return Ok(found.unwrap_or(0));
    }
    if matches!(
        metric,
        EffectMetric::CoinFlipsTotal
            | EffectMetric::CoinFlipsWon
            | EffectMetric::CoinFlipsLost
            | EffectMetric::CoinHeads
            | EffectMetric::CoinTails
    ) {
        if source != EffectMetricSource::Outcome {
            return Err(ExecutionError::UnresolvableValue(
                "coin metrics require an exact instruction outcome".into(),
            ));
        }
        let outcome = ctx.get_outcome(effect_id).ok_or_else(|| {
            ExecutionError::IncompleteEvidence("coin instruction has no completed receipt".into())
        })?;
        if outcome.status == crate::effect::OutcomeStatus::Declined {
            return Ok(0);
        }
        let results = outcome.coin_flip_results().ok_or_else(|| {
            ExecutionError::IncompleteEvidence(
                "coin instruction outcome has no retained-flip receipt".into(),
            )
        })?;
        return Ok(results
            .iter()
            .filter(|flip| match metric {
                EffectMetric::CoinFlipsTotal => true,
                EffectMetric::CoinFlipsWon => flip.winner == Some(flip.player),
                EffectMetric::CoinFlipsLost => flip.loser == Some(flip.player),
                EffectMetric::CoinHeads => flip.face == ironsmith_core::CoinFace::Heads,
                EffectMetric::CoinTails => flip.face == ironsmith_core::CoinFace::Tails,
                _ => unreachable!("guarded coin metric"),
            })
            .count() as i64);
    }
    // A metric over an instruction that never ran counts nothing.
    let Some(outcome) = ctx.get_outcome(effect_id) else {
        return Ok(0);
    };

    let outcome = outcome.instruction_result();
    let object_memory = || effect_metric_memory(game, outcome, source);

    let resolved = match metric {
        EffectMetric::CoinFlipsTotal
        | EffectMetric::CoinFlipsWon
        | EffectMetric::CoinFlipsLost
        | EffectMetric::CoinHeads
        | EffectMetric::CoinTails => unreachable!("coin metrics handled before generic outcomes"),
        EffectMetric::Count => effect_metric_object_count(game, outcome, source),
        EffectMetric::ChosenCount => {
            effect_metric_object_count(game, outcome, EffectMetricSource::ChosenObjects)
        }
        EffectMetric::AffectedCount => {
            effect_metric_object_count(game, outcome, EffectMetricSource::AffectedObjects)
        }
        EffectMetric::LifeLost => outcome
            .events_of_type::<LifeLossEvent>()
            .map(|event| event.amount as i64)
            .sum(),
        EffectMetric::LifeGained => outcome
            .events_of_type::<LifeGainEvent>()
            .map(|event| event.amount as i64)
            .sum(),
        EffectMetric::DamageDealtCappedByRecipient => {
            if source != EffectMetricSource::Outcome {
                return Err(ExecutionError::UnresolvableValue(
                    "capped damage requires an instruction outcome".into(),
                ));
            }
            return resolve_capped_damage_result(game, outcome);
        }
        EffectMetric::DamageDealt => outcome
            .events_of_type::<DamageEvent>()
            .map(|event| event.amount as i64)
            .sum(),
        EffectMetric::ExcessDamage => outcome
            .execution_facts
            .iter()
            .filter_map(|fact| match fact {
                crate::effect::ExecutionFact::ExcessDamage(value) => Some(*value as i64),
                _ => None,
            })
            .sum(),
        EffectMetric::DamagePrevented => {
            let receipts = outcome
                .execution_facts
                .iter()
                .filter_map(|fact| match fact {
                    crate::effect::ExecutionFact::PreventedDamageReceipt { amount, .. } => {
                        Some(i64::from(*amount))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if receipts.is_empty() {
                outcome
                    .events_of_type::<crate::events::DamagePreventedEvent>()
                    .map(|event| i64::from(event.amount))
                    .sum()
            } else {
                receipts.into_iter().sum()
            }
        }
        EffectMetric::FirstPower => object_memory()
            .into_iter()
            .find_map(|memory| memory.power.map(i64::from))
            .unwrap_or(0),
        EffectMetric::FirstToughness => object_memory()
            .into_iter()
            .find_map(|memory| memory.toughness.map(i64::from))
            .unwrap_or(0),
        EffectMetric::FirstManaValue => object_memory()
            .into_iter()
            .map(|memory| i64::from(memory.mana_value()))
            .next()
            .unwrap_or(0),
        EffectMetric::TotalPower => object_memory()
            .into_iter()
            .map(|memory| i64::from(memory.power.unwrap_or(0)))
            .sum(),
        EffectMetric::TotalToughness => object_memory()
            .into_iter()
            .map(|memory| i64::from(memory.toughness.unwrap_or(0)))
            .sum(),
        EffectMetric::TotalManaValue => object_memory()
            .into_iter()
            .map(|memory| i64::from(memory.mana_value()))
            .sum(),
        EffectMetric::GreatestPower => object_memory()
            .into_iter()
            .filter_map(|memory| memory.power.map(i64::from))
            .max()
            .unwrap_or(0),
        EffectMetric::GreatestToughness => object_memory()
            .into_iter()
            .filter_map(|memory| memory.toughness.map(i64::from))
            .max()
            .unwrap_or(0),
        EffectMetric::GreatestManaValue => object_memory()
            .into_iter()
            .map(|memory| i64::from(memory.mana_value()))
            .max()
            .unwrap_or(0),
        EffectMetric::ColorsAmong => object_memory()
            .into_iter()
            .fold(crate::color::ColorSet::COLORLESS, |colors, memory| {
                colors.union(memory.colors)
            })
            .count() as i64,
        EffectMetric::CardTypesAmong => {
            let mut card_types = std::collections::HashSet::new();
            for memory in object_memory() {
                card_types.extend(memory.card_types);
            }
            card_types.len() as i64
        }
        EffectMetric::GreatestPlayerCount => outcome
            .player_counts()
            .and_then(|counts| counts.iter().map(|(_, count)| *count).max())
            .unwrap_or(0),
        EffectMetric::IteratedPlayerCount => {
            let Some(player_id) = ctx.iteration.iterated_player else {
                return Ok(0);
            };
            outcome
                .player_counts()
                .map(|counts| {
                    counts
                        .iter()
                        .filter_map(|(count_player, count)| {
                            (*count_player == player_id).then_some(*count)
                        })
                        .sum()
                })
                .unwrap_or(0)
        }
        EffectMetric::PlayersWithPositiveCount => outcome
            .player_counts()
            .map(|counts| counts.iter().filter(|(_, count)| *count > 0).count() as i64)
            .unwrap_or(0),
        EffectMetric::NameStickerUniqueVowels => outcome
            .execution_facts
            .iter()
            .find_map(|fact| match fact {
                crate::effect::ExecutionFact::AppliedNameSticker { name, .. } => {
                    Some(crate::game_state::name_sticker_unique_vowels(name) as i64)
                }
                _ => None,
            })
            .unwrap_or(0),
        EffectMetric::OtherNumber => outcome
            .execution_facts
            .iter()
            .find_map(|fact| match fact {
                crate::effect::ExecutionFact::OtherNumber(value) => Some(*value as i64),
                _ => None,
            })
            .unwrap_or(0),
    };

    Ok(resolved)
}

fn resolve_capped_damage_result(
    game: &GameState,
    outcome: &EffectOutcome,
) -> Result<i64, ExecutionError> {
    let original = outcome.instruction_result();
    let mut amount = original
        .events_of_type::<DamageEvent>()
        .map(|event| u128::from(event.amount))
        .sum::<u128>();
    if amount == 0 {
        return Ok(0);
    }
    let recipients = original
        .execution_facts
        .iter()
        .filter_map(|fact| match fact {
            crate::effect::ExecutionFact::DamageRecipientBefore(receipt) => Some(receipt),
            _ => None,
        })
        .collect::<Vec<_>>();
    let [recipient] = recipients.as_slice() else {
        return Err(ExecutionError::UnresolvableValue(
            "capped damage requires one exact original-recipient receipt".into(),
        ));
    };
    match recipient {
        crate::effect::DamageRecipientBefore::Player { life, .. } => {
            amount = amount.min((*life).max(0) as u128)
        }
        crate::effect::DamageRecipientBefore::Object {
            object,
            was_creature,
            loyalty,
        } => {
            if let Some(loyalty) = loyalty {
                amount = amount.min(u128::from(*loyalty));
            }
            if *was_creature {
                // CR 608.2h: the printed creature-toughness cap is current
                // information, unlike the explicitly pre-damage life/loyalty.
                let toughness = if game
                    .object(*object)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    && !game.is_phased_out(*object)
                {
                    let checked = game
                        .continuous_query_snapshot()
                        .map_err(ExecutionError::ContinuousDiscovery)?;
                    let frame = checked
                        .try_current_characteristics(*object)
                        .map_err(ExecutionError::ContinuousDiscovery)?
                        .ok_or_else(|| {
                            ExecutionError::UnresolvableValue(
                                "original damaged creature has no available current frame".into(),
                            )
                        })?;
                    if frame.card_types.contains(&CardType::Creature) {
                        frame.toughness.ok_or_else(|| {
                            ExecutionError::UnresolvableValue(
                                "original damaged creature has no current toughness evidence"
                                    .into(),
                            )
                        })?
                    } else {
                        0
                    }
                } else {
                    let snapshot = game.source_last_known_snapshot(*object).ok_or_else(|| {
                        ExecutionError::UnresolvableValue(
                            "original damaged creature has no exact departure LKI".into(),
                        )
                    })?;
                    if snapshot.card_types.contains(&CardType::Creature) {
                        snapshot.toughness.ok_or_else(|| {
                            ExecutionError::UnresolvableValue(
                                "original damaged creature has no retained toughness evidence"
                                    .into(),
                            )
                        })?
                    } else {
                        0
                    }
                };
                amount = amount.min(toughness.max(0) as u128);
            }
        }
    }
    crate::events::damage::checked_damage_count(amount, "recipient-capped damage result")
        .map(i64::from)
}

fn resolve_prior_effect_metric(
    game: &GameState,
    ctx: &ExecutionContext,
    effect_id: crate::effect::EffectId,
    query: &PriorEffectMetricQuery,
) -> Result<i64, ExecutionError> {
    // A movement's affected memory describes its source LKI. A destination
    // result instead reads its exact original arrivals, before any additions
    // can move them again or add unrelated arrivals of their own.
    let destination_memory = if let Some(destination) = query.original_destination {
        if query.source != EffectMetricSource::AffectedObjects {
            return Err(ExecutionError::UnresolvableValue(
                "an original destination query requires arrival memory".into(),
            ));
        }
        let memory = ctx
            .get_outcome(effect_id)
            .and_then(|outcome| {
                let facts = &outcome.instruction_result().execution_facts;
                let receipts: Vec<_> = facts
                    .iter()
                    .filter_map(|fact| match fact {
                        crate::effect::ExecutionFact::OriginalZoneMoveCards(cards) => Some(cards),
                        _ => None,
                    })
                    .collect();
                (!receipts.is_empty()).then(|| {
                    receipts
                        .into_iter()
                        .flatten()
                        .filter(|card| card.zone == destination)
                        .cloned()
                        .collect::<Vec<_>>()
                })
            })
            .ok_or_else(|| {
                ExecutionError::IncompleteEvidence(
                    "an original destination query has no completed movement receipt".into(),
                )
            })?;
        Some(memory)
    } else {
        None
    };
    if effect_id == crate::effect::EffectId::ACTIVATION_COUNTER_COST
        && ctx.get_outcome(effect_id).is_none()
    {
        return Err(ExecutionError::IncompleteEvidence(
            "activation counter payment has no completed receipt".into(),
        ));
    }
    if matches!(
        query.metric,
        EffectMetric::CoinFlipsTotal
            | EffectMetric::CoinFlipsWon
            | EffectMetric::CoinFlipsLost
            | EffectMetric::CoinHeads
            | EffectMetric::CoinTails
    ) && (query.filter.is_some() || query.player.is_some())
    {
        return Err(ExecutionError::UnresolvableValue(
            "coin receipts do not accept object or player-memory filters".into(),
        ));
    }
    if let Some(reference) = query.color_choice {
        let ironsmith_core::ColorChoiceReference::Effect(color_id) = reference else {
            return Err(ExecutionError::UnresolvableValue(
                "unbound local color choice".into(),
            ));
        };
        if query.action != Some(ironsmith_core::PriorEffectAction::Revealed)
            || query.source != EffectMetricSource::AffectedObjects
            || query.metric != EffectMetric::Count
            || query.player.is_some()
            || query.counter_type.is_some()
            || query.original_destination.is_some()
            || query.filter.as_ref()
                != Some(&crate::target::ObjectFilter::default().of_chosen_color())
        {
            return Err(ExecutionError::UnresolvableValue(
                "local color count requires an exact revealed-card set".into(),
            ));
        }
        let color_outcome = ctx.get_outcome(color_id).ok_or_else(|| {
            ExecutionError::IncompleteEvidence("local color choice has no completed receipt".into())
        })?;
        let colors = color_outcome
            .instruction_result()
            .execution_facts
            .iter()
            .filter_map(|fact| match fact {
                crate::effect::ExecutionFact::ChosenColor(color) => Some(*color),
                _ => None,
            })
            .collect::<Vec<_>>();
        let [color] = colors.as_slice() else {
            return Err(ExecutionError::IncompleteEvidence(
                "local color choice receipt is missing or ambiguous".into(),
            ));
        };
        let reveal_outcome = ctx.get_outcome(effect_id).ok_or_else(|| {
            ExecutionError::IncompleteEvidence("hand reveal has no completed receipt".into())
        })?;
        // The shared reveal owner records the exact producer's action set,
        // including an explicitly empty completed reveal. Legacy native
        // receipts remain readable, but unrelated additions are never borrowed.
        let cards = if let Some(cards) = crate::effects::outcome_recording::action_objects(
            reveal_outcome,
            ironsmith_core::PriorEffectAction::Revealed,
            None,
        ) {
            cards
        } else {
            let reveals = reveal_outcome
                .instruction_result()
                .execution_facts
                .iter()
                .filter_map(|fact| match fact {
                    crate::effect::ExecutionFact::RevealedCards(cards) => Some(cards),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let [cards] = reveals.as_slice() else {
                return Err(ExecutionError::IncompleteEvidence(
                    "hand reveal receipt is missing or ambiguous".into(),
                ));
            };
            (*cards).clone()
        };
        return i64::try_from(
            cards
                .iter()
                .filter(|card| card.colors.contains(*color))
                .count(),
        )
        .map_err(|_| {
            ExecutionError::UnresolvableValue("revealed card count exceeds supported range".into())
        });
    }
    if query.action == Some(ironsmith_core::PriorEffectAction::ChosenNumber) {
        if query.source != EffectMetricSource::Outcome
            || query.metric != EffectMetric::Count
            || query.filter.is_some()
            || query.player.is_some()
            || query.counter_type.is_some()
        {
            return Err(ExecutionError::UnresolvableValue(
                "a chosen-number query requires its exact numeric decision".into(),
            ));
        }
        let outcome = ctx.get_outcome(effect_id).ok_or_else(|| {
            ExecutionError::IncompleteEvidence("numeric decision has no completed receipt".into())
        })?;
        let mut choices = outcome
            .execution_facts
            .iter()
            .filter_map(|fact| match fact {
                crate::effect::ExecutionFact::ChosenNumber(number) => Some(*number),
                _ => None,
            });
        let number = choices.next().ok_or_else(|| {
            ExecutionError::IncompleteEvidence(
                "numeric decision outcome has no chosen-number fact".into(),
            )
        })?;
        if choices.next().is_some() || outcome.as_count() != Some(i64::from(number)) {
            return Err(ExecutionError::IncompleteEvidence(
                "numeric decision receipt is ambiguous or inconsistent".into(),
            ));
        }
        return Ok(i64::from(number));
    }
    if query.filter.is_none() && query.player.is_none() && destination_memory.is_none() {
        return resolve_effect_metric(game, ctx, effect_id, query.source, query.metric);
    }

    if query.metric == EffectMetric::DamageDealtCappedByRecipient {
        return Err(ExecutionError::UnresolvableValue(
            "recipient-capped damage does not accept a memory filter".into(),
        ));
    }

    let Some(outcome) = ctx.get_outcome(effect_id) else {
        return Ok(0);
    };
    let outcome = outcome.instruction_result();
    let filter_ctx = ctx.filter_context(game);
    let selected_players = query
        .player
        .as_ref()
        .map(|player| resolve_player_filter_to_list(game, player, &filter_ctx, ctx))
        .transpose()?;

    if query.action == Some(ironsmith_core::PriorEffectAction::Drawn)
        && query.filter.is_none()
        && matches!(
            query.metric,
            EffectMetric::Count | EffectMetric::AffectedCount
        )
    {
        if let Some(objects) = crate::effects::outcome_recording::action_objects(
            outcome,
            ironsmith_core::PriorEffectAction::Drawn,
            selected_players.as_deref(),
        ) {
            return Ok(objects.len() as i64);
        }
        return outcome
            .events_of_type::<crate::events::CardsDrawnEvent>()
            .filter(|event| {
                selected_players
                    .as_ref()
                    .is_none_or(|players| players.contains(&event.player))
            })
            .try_fold(0i64, |sum, event| {
                sum.checked_add(i64::from(event.amount())).ok_or_else(|| {
                    ExecutionError::UnresolvableValue("draw metric exceeds supported range".into())
                })
            });
    }

    if query.source == EffectMetricSource::Outcome
        && matches!(
            query.metric,
            EffectMetric::LifeGained | EffectMetric::LifeLost
        )
    {
        if query.filter.is_some() {
            return Err(ExecutionError::UnresolvableValue(
                "life metrics cannot apply an object filter".into(),
            ));
        }
        let accepts = |player| {
            selected_players
                .as_ref()
                .is_none_or(|players| players.contains(&player))
        };
        let mut amounts = outcome.events.iter().filter_map(|event| {
            if query.metric == EffectMetric::LifeGained {
                event
                    .downcast::<LifeGainEvent>()
                    .filter(|life| accepts(life.player))
                    .map(|life| life.amount)
            } else {
                event
                    .downcast::<LifeLossEvent>()
                    .filter(|life| accepts(life.player))
                    .map(|life| life.amount)
            }
        });
        return amounts.try_fold(0i64, |sum, amount| {
            Some(i64::from(amount))
                .and_then(|amount| sum.checked_add(amount))
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue("life metric exceeds supported range".into())
                })
        });
    }

    let action_memory = if query.source == EffectMetricSource::AffectedObjects {
        query.action.and_then(|action| {
            crate::effects::outcome_recording::action_objects(
                outcome,
                action,
                selected_players.as_deref(),
            )
        })
    } else {
        None
    };
    let has_action_memory = action_memory.is_some();
    let mut memory = if let Some(memory) = destination_memory {
        memory
    } else if let Some(memory) = action_memory {
        memory
    } else if let Some(selected_players) = selected_players.as_ref()
        && let Some(partitions) = outcome.player_affected_object_memory()
    {
        partitions
            .iter()
            .filter(|(player, _)| selected_players.contains(player))
            .flat_map(|(_, memory)| memory.iter().cloned())
            .collect::<Vec<_>>()
    } else {
        effect_metric_memory(game, outcome, query.source)
    };

    if let Some(selected_players) = selected_players.as_ref()
        && !has_action_memory
        && outcome.player_affected_object_memory().is_none()
    {
        memory.retain(|object| selected_players.contains(&object.controller));
    }
    if let Some(filter) = query.filter.as_ref() {
        // The producer's affected set already establishes "this way"; the
        // captured memory keeps whatever zone the producer saw (library for
        // a mill, graveyard for exile-from-graveyard), so an authored or
        // defaulted zone on the counted noun ("creature card exiled this
        // way" carries the battlefield default) must not reject it.
        let mut filter = filter.clone();
        filter.zone = None;
        for branch in &mut filter.any_of {
            branch.zone = None;
        }
        memory.retain(|object| filter.matches_snapshot(object, &filter_ctx, game));
    }

    let resolved = match query.metric {
        EffectMetric::Count | EffectMetric::ChosenCount | EffectMetric::AffectedCount => {
            memory.len() as i64
        }
        EffectMetric::FirstPower => memory
            .iter()
            .find_map(|object| object.power.map(i64::from))
            .unwrap_or(0),
        EffectMetric::FirstToughness => memory
            .iter()
            .find_map(|object| object.toughness.map(i64::from))
            .unwrap_or(0),
        EffectMetric::FirstManaValue => memory
            .first()
            .map_or(0, |object| i64::from(object.mana_value())),
        EffectMetric::TotalPower => memory
            .iter()
            .map(|object| i64::from(object.power.unwrap_or(0)))
            .sum(),
        EffectMetric::TotalToughness => memory
            .iter()
            .map(|object| i64::from(object.toughness.unwrap_or(0)))
            .sum(),
        EffectMetric::TotalManaValue => memory
            .iter()
            .map(|object| i64::from(object.mana_value()))
            .sum(),
        EffectMetric::GreatestPower => memory
            .iter()
            .filter_map(|object| object.power.map(i64::from))
            .max()
            .unwrap_or(0),
        EffectMetric::GreatestToughness => memory
            .iter()
            .filter_map(|object| object.toughness.map(i64::from))
            .max()
            .unwrap_or(0),
        EffectMetric::GreatestManaValue => memory
            .iter()
            .map(|object| i64::from(object.mana_value()))
            .max()
            .unwrap_or(0),
        EffectMetric::ColorsAmong => memory
            .iter()
            .fold(crate::color::ColorSet::COLORLESS, |colors, object| {
                colors.union(object.colors)
            })
            .count() as i64,
        EffectMetric::CardTypesAmong => memory
            .iter()
            .flat_map(|object| object.card_types.iter().copied())
            .collect::<HashSet<_>>()
            .len() as i64,
        _ => resolve_effect_metric(game, ctx, effect_id, query.source, query.metric)?,
    };
    Ok(resolved)
}

fn normalize_count_as_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn count_as_names_match(lhs: &str, rhs: &str) -> bool {
    lhs.eq_ignore_ascii_case(rhs) || normalize_count_as_name(lhs) == normalize_count_as_name(rhs)
}

fn source_spell_name_for_count_as(game: &GameState, ctx: &ExecutionContext<'_>) -> Option<String> {
    if let Some(object) = game.object(ctx.source) {
        return (object.zone == Zone::Stack).then(|| object.name.to_string());
    }

    let snapshot = ctx.source_snapshot.as_ref()?;
    (snapshot.zone == Zone::Stack).then(|| snapshot.name.to_string())
}

fn count_as_card_named_for_spell_effect_bonus(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext<'_>,
    filter_ctx: &crate::target::FilterContext,
) -> usize {
    let Some(required_name) = filter.name.as_deref() else {
        return 0;
    };
    let Some(source_name) = source_spell_name_for_count_as(game, ctx) else {
        return 0;
    };

    game.object_ids_in_deterministic_order()
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| !filter.matches_non_recursive(object, filter_ctx, game))
        .filter(|object| {
            object.abilities.iter().any(|ability| {
                if !ability.functions_in(&object.zone) {
                    return false;
                }
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    return false;
                };
                let Some(spec) = static_ability.count_as_card_named_for_spell_effect_spec() else {
                    return false;
                };
                count_as_names_match(source_name.as_str(), spec.spell_name.as_str())
                    && count_as_names_match(required_name, spec.counted_name.as_str())
            })
        })
        .filter(|object| {
            let mut counted_object = (*object).clone();
            counted_object.name = required_name.to_string().into();
            filter.matches_non_recursive(&counted_object, filter_ctx, game)
        })
        .count()
}

fn source_exiled_link_count(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext<'_>,
    filter_ctx: &crate::target::FilterContext,
) -> Option<i32> {
    let uses_source_exiled_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
    });
    if !uses_source_exiled_tag {
        return None;
    }
    if ctx.get_tagged_all(crate::tag::SOURCE_EXILED_TAG).is_some() {
        return None;
    }

    Some(
        game.get_exiled_with_source_links(ctx.source)
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|object| filter.matches(object, filter_ctx, game))
            .count() as i32,
    )
}

/// Return the size of the largest creature-type cohort in `subtype_sets`.
///
/// One object can contribute to several cohorts when it has several creature
/// types, but contributes at most once to any one cohort. Noncreature subtypes
/// do not participate.
pub(crate) fn greatest_shared_creature_type_count<I, J>(subtype_sets: I) -> i32
where
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = Subtype>,
{
    let mut counts: HashMap<Subtype, i32> = HashMap::new();
    for subtypes in subtype_sets {
        let mut types_on_object = HashSet::new();
        for subtype in subtypes {
            if subtype.is_creature_type() && types_on_object.insert(subtype) {
                *counts.entry(subtype).or_default() += 1;
            }
        }
    }
    counts.into_values().max().unwrap_or(0)
}

fn greatest_shared_creature_type_count_for_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext,
    filter_ctx: &FilterContext,
) -> i32 {
    let subtype_sets = if let Some(snapshots) = value_tagged_snapshots_for_filter(game, filter, ctx)
    {
        snapshots
            .iter()
            .filter(|snapshot| {
                value_tagged_snapshot_matches_filter(game, filter, filter_ctx, snapshot)
            })
            .map(|snapshot| snapshot.subtypes.clone())
            .collect::<Vec<_>>()
    } else {
        value_candidate_ids_for_filter(game, filter, ctx)
            .into_iter()
            .filter_map(|id| game.object(id).map(|object| (id, object)))
            .filter(|(_, object)| filter.matches(object, filter_ctx, game))
            .filter_map(|(id, _)| game.current_subtypes(id))
            .collect::<Vec<_>>()
    };
    greatest_shared_creature_type_count(subtype_sets)
}

/// Unlocked doors on a Room (CR 709.5): none until a door unlocks, two once
/// fully unlocked, otherwise one. Non-Rooms have no doors.
pub(crate) fn room_unlocked_door_count(game: &GameState, object: &crate::object::Object) -> i32 {
    if !object.subtypes.contains(&crate::types::Subtype::Room)
        || game.room_has_no_unlocked_door(object.id)
    {
        0
    } else if game.is_room_fully_unlocked(object.id) {
        2
    } else {
        1
    }
}

/// Resolve a Value to a concrete i32.
/// Resolve a numeric quantity without truncating prior instruction counts.
pub fn resolve_value_wide(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i64, ExecutionError> {
    value_eval::resolve_wide(
        value,
        &value_eval::EvaluationContext::execution_context(game, ctx),
    )
}

/// Convert a nonnegative instruction quantity into an unsigned event field.
pub fn resolve_nonnegative_u32(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<u32, ExecutionError> {
    u32::try_from(resolve_value_wide(game, value, ctx)?.max(0)).map_err(|_| {
        ExecutionError::UnresolvableValue(
            "resolved quantity exceeds the unsigned event range".into(),
        )
    })
}

/// Resolve a requested quantity for an operation bounded by available resources.
/// Clamp in the wide domain before narrowing the resulting event quantity.
pub fn resolve_bounded_nonnegative_u32(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
    available: u32,
) -> Result<u32, ExecutionError> {
    let quantity = resolve_value_wide(game, value, ctx)?
        .max(0)
        .min(i64::from(available));
    u32::try_from(quantity).map_err(|_| {
        ExecutionError::InternalError(
            "bounded quantity exceeded its unsigned resource range".into(),
        )
    })
}

pub fn resolve_value(
    game: &GameState,
    value: &Value,
    ctx: &ExecutionContext,
) -> Result<i32, ExecutionError> {
    value_eval::resolve(
        value,
        &value_eval::EvaluationContext::execution_context(game, ctx),
    )
}

/// Resolve the player affected by the most recent damage effect in the
/// current resolution path.  References such as "that player" after a spell
/// deals damage use the prior effect's result, not the spell's triggering
/// event (which is usually absent for an ordinary spell on the stack).
fn prior_effect_damaged_player(ctx: &ExecutionContext) -> Option<PlayerId> {
    let mut outcomes = ctx
        .effect_outcomes
        .iter()
        .filter(|(id, _)| **id != crate::effect::EffectId::TAGGED_COUNT)
        .collect::<Vec<_>>();
    outcomes.sort_unstable_by_key(|(id, _)| std::cmp::Reverse(id.0));
    outcomes
        .into_iter()
        .find_map(|(_, outcome)| {
            outcome
                .events_of_type::<DamageEvent>()
                .find_map(|event| match event.target {
                    DamageTarget::Player(player_id) => Some(player_id),
                    _ => None,
                })
        })
        .or_else(|| {
            ctx.get_tagged_players("__it__")
                .and_then(|players| players.last().copied())
        })
}

/// Current attachment of a live source, or its exact departure receipt.
/// A live but unattached source never revives an earlier attachment.
pub(crate) fn source_attachment_target_with_lki(
    game: &GameState,
    source: ObjectId,
    retained: Option<&ObjectSnapshot>,
) -> Option<crate::object::AttachmentTarget> {
    if let Some(source) = game.object(source) {
        return source.attached_to;
    }
    game.source_departure_snapshot(source)
        .or_else(|| retained.filter(|snapshot| snapshot.object_id == source))
        .and_then(|snapshot| snapshot.attached_to)
}

fn object_lki_snapshot<'a>(
    ctx: &'a ExecutionContext<'_>,
    object_id: ObjectId,
) -> Option<&'a ObjectSnapshot> {
    ctx.source_snapshot
        .as_ref()
        .filter(|snapshot| snapshot.object_id == object_id)
        .or_else(|| ctx.target_snapshots.get(&object_id))
        .or_else(|| {
            ctx.tagged_objects
                .values()
                .flatten()
                .find(|snapshot| snapshot.object_id == object_id)
        })
}

fn tagged_snapshots_for_choose_spec<'a>(
    ctx: &'a ExecutionContext<'_>,
    spec: &ChooseSpec,
) -> Option<&'a [ObjectSnapshot]> {
    match spec.base() {
        ChooseSpec::Tagged(tag) => ctx.get_tagged_all(tag).map(Vec::as_slice),
        _ => None,
    }
}

fn snapshot_counter_total(
    snapshot: &ObjectSnapshot,
    counter_type: &Option<crate::object::CounterType>,
) -> i64 {
    if let Some(counter_type) = counter_type {
        snapshot.counters.get(counter_type).copied().unwrap_or(0) as i64
    } else {
        snapshot.counters.values().map(|count| *count as i64).sum()
    }
}

fn latest_tagged_lki_snapshot<'a>(
    game: &'a GameState,
    tagged_snapshot: &ObjectSnapshot,
) -> Option<&'a ObjectSnapshot> {
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .rev()
        .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
        .flat_map(|event| event.snapshots())
        .find(|snapshot| {
            // CR 400.7: a later incarnation of the same physical card is not
            // this tagged object. Authorized movement links update the tag
            // separately; LKI lookup must not invent such permission.
            snapshot.zone == tagged_snapshot.zone && snapshot.object_id == tagged_snapshot.object_id
        })
}

/// The last-known snapshot recorded when `object_id` last changed zones this
/// turn (its characteristics as it left, CR 608.2h).
pub(crate) fn latest_zone_change_snapshot_for_object(
    game: &GameState,
    object_id: ObjectId,
) -> Option<ObjectSnapshot> {
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .rev()
        .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
        .flat_map(|event| event.snapshots())
        .find(|snapshot| snapshot.object_id == object_id)
        .cloned()
}

fn source_characteristic_lki_for_execution<'a>(
    game: &'a GameState,
    ctx: &'a ExecutionContext<'_>,
) -> Option<&'a ObjectSnapshot> {
    game.source_characteristic_lki_snapshot(ctx.source, ctx.source_snapshot.as_ref())
}

fn value_candidate_ids_for_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    if value_tagged_snapshots_for_filter(game, filter, ctx).is_none() {
        return candidate_ids_for_filter(game, filter);
    }

    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for constraint in &filter.tagged_constraints {
        let Some(snapshots) = ctx.get_tagged_all(&constraint.tag) else {
            continue;
        };
        for snapshot in snapshots {
            if seen.insert(snapshot.object_id) {
                ids.push(snapshot.object_id);
            }
        }
    }
    ids
}

/// Non-mutating object preview for decision UI metadata.
///
/// This is intentionally narrower than effect resolution: it answers "which
/// visible objects does this structured spec prove are relevant to this
/// option?" without prompting, choosing targets, or applying fallback behavior.
pub(crate) fn preview_object_ids_for_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    let filter_ctx = ctx.filter_context(game);
    let mut ids: Vec<ObjectId> = value_candidate_ids_for_filter(game, filter, ctx)
        .into_iter()
        .filter_map(|id| game.object(id).map(|obj| (id, obj)))
        .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
        .map(|(id, _)| id)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

pub(crate) fn preview_object_ids_for_choose_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Option<Vec<ObjectId>> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => {
            preview_object_ids_for_choose_spec(game, spec, ctx)
        }
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => {
            preview_object_ids_for_choose_spec(game, inner, ctx)
        }
        ChooseSpec::Object(filter)
        | ChooseSpec::ObjectOrPlayer(filter, _)
        | ChooseSpec::All(filter) => Some(preview_object_ids_for_filter(game, filter, ctx)),
        ChooseSpec::SpecificObject(id) => Some(vec![*id]),
        ChooseSpec::Source => resolve_source_object_id(game, ctx).map(|id| vec![id]),
        ChooseSpec::Tagged(tag) => Some(
            ctx.get_tagged_all(tag)
                .map(|tagged| {
                    let mut ids: Vec<ObjectId> = tagged
                        .iter()
                        .filter_map(|snapshot| resolve_tagged_object_id(game, ctx, snapshot))
                        .collect();
                    ids.sort();
                    ids.dedup();
                    ids
                })
                .unwrap_or_default(),
        ),
        ChooseSpec::Iterated => ctx.iteration.iterated_object.map(|id| vec![id]),
        ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::AttackedPlayerOrPlaneswalker
        | ChooseSpec::Player(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::EachPlayer(_) => None,
    }
}

fn count_matching_objects_for_player(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    player_id: PlayerId,
    ctx: &ExecutionContext,
) -> usize {
    let filter_ctx = ctx.filter_context(game);
    value_candidate_ids_for_filter(game, filter, ctx)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|obj| game.controller_of(obj) == player_id)
        .filter(|obj| filter.matches(obj, &filter_ctx, game))
        .count()
}

fn value_tagged_snapshots_for_filter<'a>(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &'a ExecutionContext,
) -> Option<Vec<&'a ObjectSnapshot>> {
    if !crate::object_query::require_captured_public_collections(
        game,
        filter,
        &ctx.filter_context(game),
    ) {
        return Some(Vec::new());
    }
    // A public destination reference reads the exact still-present result,
    // rather than historical types of a card that has moved or turned face down.
    if filter.match_captured_public_destination {
        return None;
    }
    // A leave-the-battlefield event captures each attachment under
    // `attached_source` before state-based actions move unattached Auras to
    // their owners' graveyards.  Counts such as Hateful Eidolon's "each Aura
    // ... that was attached to it" must evaluate those LKI snapshots rather
    // than the attachments' new zone objects.
    if let [constraint] = filter.tagged_constraints.as_slice()
        && constraint.relation == crate::filter::TaggedOpbjectRelation::WasAttachedToTaggedObject
    {
        let tagged_hosts = ctx.get_tagged_all(&constraint.tag)?;
        let attached_snapshots = ctx.get_tagged_all("attached_source")?;
        let mut seen = HashSet::new();
        return Some(
            attached_snapshots
                .iter()
                .filter(|attachment| {
                    tagged_hosts
                        .iter()
                        .any(|host| host.attachments.contains(&attachment.object_id))
                })
                .filter(|attachment| seen.insert(attachment.stable_id))
                .collect(),
        );
    }

    let only_is_tagged_constraints = !filter.tagged_constraints.is_empty()
        && filter.tagged_constraints.iter().all(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
            )
        });
    if !only_is_tagged_constraints {
        return None;
    }

    let mut seen = HashSet::new();
    let mut snapshots = Vec::new();
    for constraint in &filter.tagged_constraints {
        let Some(tagged) = ctx.get_tagged_all(&constraint.tag) else {
            continue;
        };
        for snapshot in tagged {
            if seen.insert(snapshot.object_id) {
                snapshots.push(snapshot);
            }
        }
    }
    Some(snapshots)
}

/// Match a tagged LKI snapshot while honoring an explicitly required current
/// zone.
///
/// Zone-changing tagged effects intentionally preserve the pre-move snapshot
/// so later clauses can still inspect characteristics such as token status and
/// controller. When a value asks for objects in the destination zone, validate
/// that the stable object is currently there and project only its zone onto the
/// LKI snapshot before applying the rest of the filter.
fn value_tagged_snapshot_matches_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    filter_ctx: &crate::filter::FilterContext,
    snapshot: &ObjectSnapshot,
) -> bool {
    if filter.matches_snapshot(snapshot, filter_ctx, game) {
        return true;
    }

    let Some(required_zone) = filter.zone else {
        return false;
    };
    let Some(current_id) = resolve_tagged_object_id_unscoped(game, snapshot) else {
        return false;
    };
    let Some(current) = game.object(current_id) else {
        return false;
    };
    if current.zone != required_zone || snapshot.zone == required_zone {
        return false;
    }

    let mut projected = snapshot.clone();
    projected.zone = required_zone;
    filter.matches_snapshot(&projected, filter_ctx, game)
}

/// Returns the sorted effective power values represented by a filter.
///
/// This is the value-domain counterpart to `Value::DistinctPowers`: callers
/// that must perform one operation for each distinct value need the values,
/// not just their count.
pub(crate) fn distinct_power_values_for_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext,
) -> Vec<i32> {
    let filter_ctx = ctx.filter_context(game);
    let mut powers = HashSet::new();
    if let Some(snapshots) = value_tagged_snapshots_for_filter(game, filter, ctx) {
        for snapshot in snapshots
            .into_iter()
            .filter(|snapshot| filter.matches_snapshot(snapshot, &filter_ctx, game))
        {
            if let Some(power) = snapshot.power {
                powers.insert(power);
            }
        }
    } else {
        for object in value_candidate_ids_for_filter(game, filter, ctx)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|object| filter.matches(object, &filter_ctx, game))
        {
            if let Some(power) = game.calculated_power(object.id).or_else(|| object.power()) {
                powers.insert(power);
            }
        }
    }
    let mut powers = powers.into_iter().collect::<Vec<_>>();
    powers.sort_unstable();
    powers
}

// ============================================================================
// Player Filter Resolution
// ============================================================================

/// Resolve a ChooseSpec to a PlayerId.
///
/// This is the primary way to resolve "which player" from a ChooseSpec.
/// Handles targeting, filters, and special references.
fn attacked_target_from_trigger(ctx: &ExecutionContext) -> Option<AttackEventTarget> {
    let triggering_event = ctx.triggering_event.as_ref()?;
    if let Some(event) = triggering_event.downcast::<CreatureAttackedEvent>() {
        return Some(event.target);
    }
    if let Some(event) = triggering_event.downcast::<CreatureBecameBlockedEvent>() {
        return event.attack_target;
    }
    None
}

pub fn resolve_player_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<PlayerId, ExecutionError> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => resolve_player_from_spec(game, spec, ctx),
        // Target wrapper - look in ctx.targets for a player target
        ChooseSpec::Target(inner) => {
            if let Some(player_id) = matching_player_targets_for_spec(game, spec, ctx).first() {
                return Ok(*player_id);
            }
            if !matching_object_targets_for_spec(game, spec, ctx).is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }
            resolve_player_from_spec(game, inner, ctx)
        }

        // Player filter - delegate to resolve_player_filter
        ChooseSpec::Player(filter) => resolve_player_filter(game, filter, ctx),
        ChooseSpec::PlayerOrPlaneswalker(filter) => {
            if let Some(player_id) = matching_player_targets_for_spec(game, spec, ctx).first() {
                return Ok(*player_id);
            }
            if !matching_object_targets_for_spec(game, spec, ctx).is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }
            resolve_player_filter(game, filter, ctx)
        }
        ChooseSpec::ObjectOrPlayer(_, filter) => {
            if let Some(player_id) = matching_player_targets_for_spec(game, spec, ctx).first() {
                return Ok(*player_id);
            }
            if !matching_object_targets_for_spec(game, spec, ctx).is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }
            resolve_player_filter(game, filter, ctx)
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => match attacked_target_from_trigger(ctx) {
            Some(AttackEventTarget::Player(player_id)) => Ok(player_id),
            Some(AttackEventTarget::Planeswalker(planeswalker_id)) => {
                let planeswalker = game
                    .object(planeswalker_id)
                    .ok_or(ExecutionError::ObjectNotFound(planeswalker_id))?;
                Ok(game.controller_of(planeswalker))
            }
            Some(AttackEventTarget::Battle(battle_id)) => game
                .battle_protector(battle_id)
                .ok_or(ExecutionError::ObjectNotFound(battle_id)),
            // CR 506.4c: it isn't attacking any player or planeswalker.
            Some(AttackEventTarget::Nothing) => Err(ExecutionError::InvalidTarget),
            None => ctx.combat.defending_player.ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "Attacked player/planeswalker not set".to_string(),
                )
            }),
        },

        // Source controller ("you" on a permanent's ability)
        ChooseSpec::SourceController => Ok(ctx.controller),

        // Source owner
        ChooseSpec::SourceOwner => {
            if let Some(obj) = game.object(ctx.source) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.source_snapshot.as_ref() {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(ctx.source))
            }
        }

        // Specific player
        ChooseSpec::SpecificPlayer(id) => Ok(*id),

        // Tagged - not typically used for players, but could be extended
        ChooseSpec::Tagged(_) => Err(ExecutionError::UnresolvableValue(
            "Tagged spec cannot be resolved to a player".to_string(),
        )),

        // EachPlayer - resolve all matching players (returns first for single resolution)
        ChooseSpec::EachPlayer(filter) => resolve_player_filter(game, filter, ctx),

        // WithCount wrapper - delegate to inner spec
        ChooseSpec::WithCount(inner, _) | ChooseSpec::WithCountValue(inner, _, _) => {
            resolve_player_from_spec(game, inner, ctx)
        }

        // Iterated player (in ForEach loops)
        ChooseSpec::Iterated => ctx.iteration.iterated_player.ok_or_else(|| {
            ExecutionError::UnresolvableValue(
                "Iterated player not set (must be inside ForEach loop)".to_string(),
            )
        }),

        // Object specs can't be resolved to players
        ChooseSpec::Object(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source
        | ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::All(_) => Err(ExecutionError::UnresolvableValue(
            "Object spec cannot be resolved to a player".to_string(),
        )),
    }
}

/// "The attacking player": the one captured by the triggering attack, else
/// the active player while combat is under way or when the triggering event
/// is combat damage (CR 506.2: the active player is the attacking player).
fn combat_attacking_player(game: &GameState, ctx: &ExecutionContext) -> Option<PlayerId> {
    ctx.combat.attacking_player.or_else(|| {
        let combat_damage_trigger = ctx.triggering_event.as_ref().is_some_and(|event| {
            event
                .downcast::<DamageEvent>()
                .is_some_and(|damage| damage.is_combat)
                || event.downcast::<crate::CombatDamageEvent>().is_some()
        });
        (combat_damage_trigger || game.turn.phase == crate::game_state::Phase::Combat)
            .then_some(game.turn.active_player)
    })
}

/// Context tag holding the opponent the effect's controller chose for a
/// singular untargeted "an opponent" instruction during this resolution.
pub(crate) const AN_OPPONENT_CHOICE_TAG: &str = "__an_opponent_choice";

/// Sentinel error raised when a singular "an opponent" must be chosen among
/// two or more opponents; [`crate::effects::execute_effect`] asks the
/// controller and retries the instruction once.
pub(crate) const AN_OPPONENT_CHOICE_REQUIRED: &str =
    "Opponent filter requires choosing one of several opponents";

/// The opponents a singular "an opponent" may be, in seat order.
pub(crate) fn an_opponent_choice_candidates(
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<PlayerId> {
    let filter_ctx = ctx.filter_context(game);
    game.players
        .iter()
        .filter(|player| {
            player.id != ctx.controller
                && player.is_in_game()
                && PlayerFilter::Opponent.matches_player(player.id, &filter_ctx)
        })
        .map(|player| player.id)
        .collect()
}

/// Resolve a PlayerFilter to a concrete PlayerId.
pub fn resolve_player_filter(
    game: &GameState,
    spec: &PlayerFilter,
    ctx: &ExecutionContext,
) -> Result<PlayerId, ExecutionError> {
    let player = (|| match spec {
        PlayerFilter::You => Ok(ctx.controller),
        PlayerFilter::EffectController => Ok(ctx.controller),
        PlayerFilter::Any => {
            // "Any" player needs resolution from targets or defaults to controller
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            Ok(ctx.controller)
        }
        PlayerFilter::NotYou => {
            let filter_ctx = ctx.filter_context(game);
            for player in game.players.iter() {
                if player.id != ctx.controller
                    && player.is_in_game()
                    && PlayerFilter::NotYou.matches_player(player.id, &filter_ctx)
                {
                    return Ok(player.id);
                }
            }
            Err(ExecutionError::UnresolvableValue(
                "NotYou filter requires another in-game player".to_string(),
            ))
        }
        PlayerFilter::Opponent => {
            let filter_ctx = ctx.filter_context(game);
            // Replacement payloads may carry the affected player as context,
            // even when this instruction asks for a different player. Reuse
            // only a live candidate satisfying the authored player filter.
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target
                    && game.player(*id).is_some_and(|player| player.is_in_game())
                    && spec.matches_player(*id, &filter_ctx)
                {
                    return Ok(*id);
                }
            }
            let opponents = game
                .players
                .iter()
                .filter(|player| {
                    player.id != ctx.controller
                        && player.is_in_game()
                        && PlayerFilter::Opponent.matches_player(player.id, &filter_ctx)
                })
                .map(|player| player.id)
                .collect::<Vec<_>>();
            if let [opponent] = opponents.as_slice() {
                return Ok(*opponent);
            }
            // A singular untargeted "an opponent" is chosen by the effect's
            // controller as the instruction is performed. The choice made
            // earlier in this resolution is reused so "that player" stays the
            // same opponent.
            if let Some(chosen) = ctx
                .get_tagged_players(AN_OPPONENT_CHOICE_TAG)
                .and_then(|players| players.first().copied())
                .filter(|chosen| opponents.contains(chosen))
            {
                return Ok(chosen);
            }
            if opponents.len() >= 2 {
                return Err(ExecutionError::UnresolvableValue(
                    AN_OPPONENT_CHOICE_REQUIRED.to_string(),
                ));
            }
            Err(ExecutionError::UnresolvableValue(
                "Opponent filter requires a targeted player".to_string(),
            ))
        }
        PlayerFilter::Teammate => {
            let filter_ctx = ctx.filter_context(game);
            // Replacement payloads may carry the affected player as context,
            // even when this instruction asks for a different player. Reuse
            // only a live candidate satisfying the authored player filter.
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target
                    && game.player(*id).is_some_and(|player| player.is_in_game())
                    && spec.matches_player(*id, &filter_ctx)
                {
                    return Ok(*id);
                }
            }
            let teammates = game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && PlayerFilter::Teammate.matches_player(player.id, &filter_ctx)
                })
                .map(|player| player.id)
                .collect::<Vec<_>>();
            if let [teammate] = teammates.as_slice() {
                return Ok(*teammate);
            }
            Err(ExecutionError::UnresolvableValue(
                "Teammate filter requires a targeted player".to_string(),
            ))
        }
        PlayerFilter::PlayerToYourLeft => game
            .closest_in_game_player_to_left_matching(ctx.controller, |_| true)
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "there is no in-game player to the effect controller's left".to_string(),
                )
            }),
        PlayerFilter::PlayerToYourRight => game
            .closest_in_game_player_to_right_matching(ctx.controller, |_| true)
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "there is no in-game player to the effect controller's right".to_string(),
                )
            }),
        PlayerFilter::Attacking => combat_attacking_player(game, ctx).ok_or_else(|| {
            ExecutionError::UnresolvableValue("AttackingPlayer not set".to_string())
        }),
        PlayerFilter::DamagedPlayer => {
            if let Some(triggering_event) = &ctx.triggering_event
                && let Some(damage_event) = triggering_event.downcast::<DamageEvent>()
                && let DamageTarget::Player(player_id) = damage_event.target
            {
                return Ok(player_id);
            }
            ctx.get_tagged_players("damaged_player")
                .and_then(|players| players.first().copied())
                .or_else(|| prior_effect_damaged_player(ctx))
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "DamagedPlayer requires a player damage event".to_string(),
                    )
                })
        }
        PlayerFilter::Target(_) => {
            let targets = if ctx.targets.is_empty() && !ctx.targets_are_cost_choices {
                ctx.announced_targets.as_deref().unwrap_or(&ctx.targets)
            } else {
                &ctx.targets
            };
            for target in targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            if !ctx.targets_are_cost_choices {
                if let Some(player) = ctx
                    .announced_targets
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .find_map(|target| match target {
                        ResolvedTarget::Player(player) => Some(*player),
                        _ => None,
                    })
                {
                    return Ok(player);
                }
            }
            Err(ExecutionError::InvalidTarget)
        }
        PlayerFilter::AliasedTarget(inner) => {
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            let filter_ctx = ctx.filter_context(game);
            ctx.get_tagged_players(crate::tag::DELAYED_TARGET_PLAYERS_TAG)
                .and_then(|players| {
                    players
                        .iter()
                        .copied()
                        .find(|player| inner.matches_player(*player, &filter_ctx))
                })
                .ok_or(ExecutionError::InvalidTarget)
        }
        PlayerFilter::Excluding { .. } => {
            let filter_ctx = ctx.filter_context(game);
            let mut players = resolve_player_filter_to_list(game, spec, &filter_ctx, ctx)?;
            players
                .drain(..)
                .next()
                .ok_or_else(|| ExecutionError::UnresolvableValue("No matching players".to_string()))
        }
        PlayerFilter::Specific(id) => Ok(*id),
        PlayerFilter::MostLifeTied
        | PlayerFilter::LowestLifeTied
        | PlayerFilter::CastCardTypeThisTurn(_)
        | PlayerFilter::AttackedBySourceThisTurn
        | PlayerFilter::WasDealtDamageBySourceThisGame { .. }
        | PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. }
        | PlayerFilter::LostLifeThisTurn { .. }
        | PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. }
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. }
        | PlayerFilter::OpponentWithMoreControlledObjectsThan { .. }
        | PlayerFilter::ControlsMost { .. }
        | PlayerFilter::ControlsFewestTied { .. }
        | PlayerFilter::OpponentOf(_)
        | PlayerFilter::MaxSpeed { .. }
        | PlayerFilter::MostCardsInHand => {
            let filter_ctx = ctx.filter_context(game);
            let mut players = resolve_player_filter_to_list(game, spec, &filter_ctx, ctx)?;
            players
                .drain(..)
                .next()
                .ok_or_else(|| ExecutionError::UnresolvableValue("No matching players".to_string()))
        }
        PlayerFilter::ChosenPlayer => ctx
            .combat
            .chosen_player
            .or_else(|| game.chosen_player(ctx.source))
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "ChosenPlayer requires a previously chosen player".to_string(),
                )
            }),
        PlayerFilter::TaggedPlayer(tag) => resolve_tagged_players_from_context(game, ctx, tag)
            .and_then(|players| players.first().copied())
            .ok_or_else(|| {
                if tag.as_str() == ironsmith_core::tag::DAMAGE_SOURCE_CONTROLLER_TAG {
                    return ExecutionError::IncompleteEvidence(
                        "missing damage-time source controller".into(),
                    );
                }
                ExecutionError::UnresolvableValue(format!(
                    "TaggedPlayer requires a tagged player for '{tag}'"
                ))
            }),
        PlayerFilter::ControllerOf(object_ref) | PlayerFilter::AliasedControllerOf(object_ref) => {
            resolve_controller_of(game, ctx, object_ref)
        }
        PlayerFilter::OwnerOf(object_ref) | PlayerFilter::AliasedOwnerOf(object_ref) => {
            resolve_owner_of(game, ctx, object_ref)
        }
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            for target in &ctx.targets {
                if let ResolvedTarget::Player(id) = target {
                    return Ok(*id);
                }
            }
            resolve_controller_of(game, ctx, &ObjectRef::Target)
        }
        PlayerFilter::Active => game
            .singular_active_player(ctx.combat.chosen_player)
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue("There is no active player".to_string())
            }),
        PlayerFilter::Defending => match ctx.defending_players(game)?.as_slice() {
            [player] => Ok(*player),
            [] => Err(ExecutionError::InvalidTarget),
            _ => Err(ExecutionError::UnresolvedPlayerDecision {
                player: ctx.controller,
                decision: "choose the defending player",
            }),
        },
        PlayerFilter::IteratedPlayer => ctx
            .iteration
            .iterated_player
            .or_else(|| {
                ctx.get_tagged_players("__it__")
                    .and_then(|players| players.first().copied())
            })
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "IteratedPlayer not set (must be inside ForEachOpponent/ForEachPlayer)"
                        .to_string(),
                )
            }),
    })()?;

    if game.source_snapshot_is_exempt_from_range(Some(ctx.source), ctx.source_snapshot.as_ref())
        || game.player_is_within_range(ctx.controller, player)
    {
        Ok(player)
    } else {
        Err(ExecutionError::OutOfRange)
    }
}

/// Resolve the player who is instructed to make a choice. CR 801.5c is
/// intentionally confined to chooser resolution: it must not make an
/// otherwise out-of-range player eligible for the effect itself.
pub fn resolve_player_filter_as_chooser(
    game: &GameState,
    spec: &PlayerFilter,
    ctx: &ExecutionContext,
) -> Result<PlayerId, ExecutionError> {
    let filter_ctx = ctx.filter_context(game);
    if filter_ctx.players_in_range.is_none() {
        return resolve_player_filter(game, spec, ctx);
    }

    let candidates = resolve_player_filter_to_list(game, spec, &filter_ctx, ctx)?;
    if !candidates.is_empty() {
        return resolve_player_filter(game, spec, ctx);
    }

    let mut unrestricted_ctx = filter_ctx;
    unrestricted_ctx.players_in_range = None;
    let appropriate = resolve_player_filter_to_list(game, spec, &unrestricted_ctx, ctx)?;
    game.closest_in_game_player_to_left_matching(ctx.controller, |candidate| {
        appropriate.contains(&candidate)
    })
    .ok_or_else(|| {
        ExecutionError::UnresolvableValue(
            "no appropriate player can make the required choice".to_string(),
        )
    })
}

fn resolve_controller_of(
    game: &GameState,
    ctx: &ExecutionContext,
    object_ref: &ObjectRef,
) -> Result<PlayerId, ExecutionError> {
    match object_ref {
        ObjectRef::FilterCandidate => Err(ExecutionError::InvalidTarget),
        ObjectRef::Target => {
            let target_id = match find_target_object(&ctx.targets) {
                Ok(id) => id,
                Err(error) => {
                    return delayed_captured_target_snapshot(ctx)
                        .map(|snapshot| snapshot.controller)
                        .ok_or(error);
                }
            };
            if let Some(obj) = game.object(target_id) {
                Ok(game.controller_of(obj))
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }
        ObjectRef::Specific(object_id) => {
            if let Some(obj) = game.object(*object_id) {
                Ok(game.controller_of(obj))
            } else if let Some(snapshot) = object_lki_snapshot(ctx, *object_id) {
                Ok(snapshot.controller)
            } else {
                Err(ExecutionError::ObjectNotFound(*object_id))
            }
        }
        ObjectRef::Tagged(tag) => {
            if let Some(snapshot) = ctx.get_tagged(tag) {
                // The tagged object is still that same object on the
                // battlefield or stack: its controller is its current one
                // (a reanimated creature is controlled by the Aura's
                // controller, not the graveyard card's owner). Once it has
                // left, the snapshot's last known controller stands
                // (CR 608.2h).
                let exact_attack_participant = ctx
                    .triggering_event
                    .as_ref()
                    .and_then(|event| event.downcast::<crate::events::CreatureAttackedEvent>())
                    .is_some_and(|attack| attack.attacker == snapshot.object_id);
                let live_id = if exact_attack_participant {
                    // A declaration names this incarnation. A later card with
                    // the same stable identity is not the attacking creature.
                    Some(snapshot.object_id)
                } else {
                    resolve_tagged_object_id(game, ctx, snapshot)
                };
                let live_controller = live_id
                    .and_then(|id| game.object(id))
                    .filter(|object| {
                        matches!(
                            object.zone,
                            crate::zone::Zone::Battlefield | crate::zone::Zone::Stack
                        ) && (object.id == snapshot.object_id || object.zone != snapshot.zone)
                    })
                    .map(|object| game.controller_of(object));
                let departure_controller = if exact_attack_participant {
                    game.source_departure_snapshot(snapshot.object_id)
                        .map(|departed| departed.controller)
                } else {
                    ctx.triggering_event
                        .as_ref()
                        .filter(|event| {
                            matches!(
                                event.kind(),
                                crate::events::EventKind::PermanentTapped
                                    | crate::events::EventKind::PermanentUntapped
                                    | crate::events::EventKind::ObjectBecameAttached
                                    | crate::events::EventKind::ObjectBecameUnattached
                                    | crate::events::EventKind::PermanentPhasedIn
                                    | crate::events::EventKind::PermanentPhasedOut
                            )
                        })
                        .and_then(|_| {
                            latest_zone_change_snapshot_for_object(game, snapshot.object_id)
                        })
                        .map(|departed| departed.controller)
                };
                if exact_attack_participant {
                    return live_controller.or(departure_controller).ok_or_else(||
                        ExecutionError::IncompleteEvidence(
                            "attacking creature's controller requires its exact live incarnation or departure receipt".into()));
                }
                Ok(live_controller
                    .or(departure_controller)
                    .unwrap_or(snapshot.controller))
            } else if matches!(tag.as_str(), "triggering" | "it" | "__it__")
                && let Some(attack) = ctx
                    .triggering_event
                    .as_ref()
                    .and_then(|event| event.downcast::<crate::events::CreatureAttackedEvent>())
            {
                // The trigger may be stacked after its exact attacker left.
                // Its event identity still owns this reference even if no
                // presentation tag could be populated during stacking.
                game.controller_of_id(attack.attacker)
                    .or_else(|| game.source_departure_snapshot(attack.attacker)
                        .map(|snapshot| snapshot.controller))
                    .ok_or_else(|| ExecutionError::IncompleteEvidence(
                        "attacking creature's controller requires its exact live incarnation or departure receipt".into()))
            } else if let Some(player) = ctx
                .get_tagged_players(tag.as_str())
                .and_then(|players| players.first().copied())
            {
                // "That permanent's controller or that player": the tagged
                // result was a player (damage dealt to a player target).
                Ok(player)
            } else if matches!(tag.as_str(), "enchanted" | "equipped")
                && let Some(crate::object::AttachmentTarget::Object(host)) = game
                    .object(ctx.source)
                    .and_then(|source| source.attached_to)
                && let Some(host_object) = game.object(host)
            {
                // "enchanted creature's controller" resolves through the
                // source's attachment, not an explicitly bound tag.
                Ok(game.controller_of(host_object))
            } else if tag.as_str().starts_with("damaged")
                && let [target] = ctx.targets.as_slice()
            {
                // The damage to the single target was prevented, so the
                // damage result bound nothing; the reference is still that
                // target ("that permanent's controller or that player").
                match target {
                    ResolvedTarget::Player(player) => Ok(*player),
                    ResolvedTarget::Object(object_id) => game
                        .object(*object_id)
                        .map(|object| game.controller_of(object))
                        .or_else(|| {
                            ctx.target_snapshots
                                .get(object_id)
                                .map(|snapshot| snapshot.controller)
                        })
                        .ok_or(ExecutionError::ObjectNotFound(*object_id)),
                }
            } else if tag.as_str() == crate::tag::SOURCE_OBJECT_TAG {
                // "This permanent's controller": the ability's own source.
                Ok(game
                    .object(ctx.source)
                    .map(|source| game.controller_of(source))
                    .unwrap_or(ctx.controller))
            } else {
                Err(ExecutionError::TagNotFound(tag.to_string()))
            }
        }
    }
}

fn resolve_owner_of(
    game: &GameState,
    ctx: &ExecutionContext,
    object_ref: &ObjectRef,
) -> Result<PlayerId, ExecutionError> {
    match object_ref {
        ObjectRef::FilterCandidate => Err(ExecutionError::InvalidTarget),
        ObjectRef::Target => {
            let target_id = match find_target_object(&ctx.targets) {
                Ok(id) => id,
                Err(error) => {
                    // "Put up to four target cards from a player's graveyard
                    // ... That player ...": with no card chosen, the player
                    // whose graveyard was named was chosen on its own.
                    return delayed_captured_target_snapshot(ctx)
                        .map(|snapshot| snapshot.owner)
                        .or_else(|| {
                            ctx.get_tagged_players(crate::tag::TARGET_GRAVEYARD_PLAYER_TAG)
                                .and_then(|players| players.first().copied())
                        })
                        .ok_or(error);
                }
            };
            if let Some(obj) = game.object(target_id) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.target_snapshots.get(&target_id) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(target_id))
            }
        }
        ObjectRef::Specific(object_id) => {
            if let Some(obj) = game.object(*object_id) {
                Ok(obj.owner)
            } else if let Some(snapshot) = ctx.target_snapshots.get(object_id) {
                Ok(snapshot.owner)
            } else {
                Err(ExecutionError::ObjectNotFound(*object_id))
            }
        }
        ObjectRef::Tagged(tag) => {
            if let Some(snapshot) = ctx.get_tagged(tag) {
                Ok(snapshot.owner)
            } else if tag.as_str() == crate::tag::SOURCE_OBJECT_TAG {
                // "This artifact's owner": the ability's own source.
                game.object(ctx.source)
                    .map(|source| source.owner)
                    .or_else(|| ctx.source_snapshot.as_ref().map(|snapshot| snapshot.owner))
                    .ok_or(ExecutionError::ObjectNotFound(ctx.source))
            } else {
                Err(ExecutionError::TagNotFound(tag.to_string()))
            }
        }
    }
}

// ============================================================================
// Target Finding
// ============================================================================

/// Find the first object target in the targets list.
/// A delayed triggered ability has no targets of its own chosen by the
/// scheduling ability; when it was registered, that ability's object targets
/// were captured as `targeted_<index>` snapshots. "That player"/"its owner"
/// references to the scheduling ability's target read them from there
/// ("... target cards from a player's graveyard ... That player draws a card
/// at the beginning of the next turn's upkeep").
fn delayed_captured_target_snapshot<'a>(
    ctx: &'a ExecutionContext<'_>,
) -> Option<&'a crate::snapshot::ObjectSnapshot> {
    ctx.get_tagged("targeted_0")
}

pub fn find_target_object(targets: &[ResolvedTarget]) -> Result<ObjectId, ExecutionError> {
    for target in targets {
        if let ResolvedTarget::Object(id) = target {
            return Ok(*id);
        }
    }
    Err(ExecutionError::InvalidTarget)
}

/// Resolve a [`ChooseSpec`] to a single object id.
///
/// This supports non-target references (e.g. `Source`, `Tagged`, `Iterated`)
/// in addition to classic `ctx.targets`-backed target specs.
/// If multiple objects resolve, this returns the first one to preserve established
/// single-target executor behavior.
pub fn resolve_single_object_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<ObjectId, ExecutionError> {
    resolve_objects_from_spec(game, spec, ctx)?
        .into_iter()
        .next()
        .ok_or(ExecutionError::InvalidTarget)
}

/// Resolve a [`ChooseSpec`] to a single object or player target.
pub fn resolve_single_target_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<ResolvedTarget, ExecutionError> {
    if let Ok(object_id) = resolve_single_object_from_spec(game, spec, ctx) {
        return Ok(ResolvedTarget::Object(object_id));
    }

    resolve_players_from_spec(game, spec, ctx)?
        .into_iter()
        .next()
        .map(ResolvedTarget::Player)
        .ok_or(ExecutionError::InvalidTarget)
}

/// Find the first player target in the targets list.
pub fn find_target_player(targets: &[ResolvedTarget]) -> Result<PlayerId, ExecutionError> {
    for target in targets {
        if let ResolvedTarget::Player(id) = target {
            return Ok(*id);
        }
    }
    Err(ExecutionError::InvalidTarget)
}

/// Normalize object selections returned by a decision maker.
///
/// This guarantees:
/// - at most `required` objects are returned,
/// - every object is from `candidates`,
/// - there are no duplicates,
/// - if fewer than `required` valid selections were provided, the remainder is
///   filled deterministically from `candidates` order.
pub fn normalize_object_selection(
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    required: usize,
) -> Vec<ObjectId> {
    let mut selected = Vec::with_capacity(required);

    for id in chosen {
        if selected.len() == required {
            break;
        }
        if candidates.contains(&id) && !selected.contains(&id) {
            selected.push(id);
        }
    }

    if selected.len() < required {
        for &id in candidates {
            if selected.len() == required {
                break;
            }
            if !selected.contains(&id) {
                selected.push(id);
            }
        }
    }

    selected
}

fn matching_object_targets_for_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    if !ctx.target_assignments.is_empty() {
        let assigned: Vec<ObjectId> = ctx
            .target_assignments
            .iter()
            .filter(|assignment| {
                assignment.spec == *spec
                    || assignment.spec.base() == spec.base()
                    || crate::targeting::target_spec_matches_chooser_assignment(
                        spec,
                        &assignment.spec,
                    )
            })
            .flat_map(|assignment| ctx.targets[assignment.range.clone()].iter())
            .filter_map(|target| match target {
                ResolvedTarget::Object(id) => Some(*id),
                ResolvedTarget::Player(_) => None,
            })
            .collect();
        if !assigned.is_empty() {
            return assigned;
        }
    }

    ctx.targets
        .iter()
        .filter_map(|target| {
            let ResolvedTarget::Object(id) = target else {
                return None;
            };
            validate_target(game, target, spec, ctx).then_some(*id)
        })
        .collect()
}

fn matching_player_targets_for_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Vec<PlayerId> {
    if !ctx.target_assignments.is_empty() {
        let assigned: Vec<PlayerId> = ctx
            .target_assignments
            .iter()
            .filter(|assignment| {
                assignment.spec == *spec
                    || assignment.spec.base() == spec.base()
                    || crate::targeting::target_spec_matches_chooser_assignment(
                        spec,
                        &assignment.spec,
                    )
            })
            .flat_map(|assignment| ctx.targets[assignment.range.clone()].iter())
            .filter_map(|target| match target {
                ResolvedTarget::Player(id) => Some(*id),
                ResolvedTarget::Object(_) => None,
            })
            .collect();
        if !assigned.is_empty() {
            return assigned;
        }
    }

    ctx.targets
        .iter()
        .filter_map(|target| {
            let ResolvedTarget::Player(id) = target else {
                return None;
            };
            validate_target(game, target, spec, ctx).then_some(*id)
        })
        .collect()
}

// ============================================================================
// Target Validation
// ============================================================================

/// Validate that a resolved target matches a target spec.
pub fn validate_target(
    game: &GameState,
    target: &ResolvedTarget,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> bool {
    // An ability on the stack is named by its own id; match the filter on
    // its source's characteristics for that particular stack object.
    if let ResolvedTarget::Object(id) = target
        && let Some(entry) = game.stack_ability_entry(*id)
    {
        let mut spec = spec;
        while let ChooseSpec::Target(inner)
        | ChooseSpec::SurfaceHinted { spec: inner, .. }
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) = spec
        {
            spec = inner;
        }
        let ChooseSpec::Object(filter) = spec else {
            return false;
        };
        let Some(source) = game.object(entry.object_id) else {
            return false;
        };
        let mut filter_ctx = ctx.filter_context(game);
        filter_ctx.stack_entry = Some(*id);
        return filter.matches(source, &filter_ctx, game);
    }
    let filter_ctx = ctx.filter_context(game);
    let range_exempt =
        game.source_snapshot_is_exempt_from_range(Some(ctx.source), ctx.source_snapshot.as_ref());
    let within_range = match target {
        ResolvedTarget::Object(id) => game.object(*id).map_or_else(
            || {
                ctx.target_snapshots.get(id).is_some_and(|snapshot| {
                    range_exempt
                        || game.snapshot_is_within_range(ctx.controller, snapshot, Some(ctx.source))
                })
            },
            |_| range_exempt || game.object_is_within_range(ctx.controller, *id, Some(ctx.source)),
        ),
        ResolvedTarget::Player(id) => {
            range_exempt || game.player_is_within_range(ctx.controller, *id)
        }
    };
    if !within_range {
        return false;
    }

    match (target, spec) {
        // Selection wrappers do not change target legality.
        (
            _,
            ChooseSpec::Target(inner)
            | ChooseSpec::SurfaceHinted { spec: inner, .. }
            | ChooseSpec::WithCount(inner, _)
            | ChooseSpec::WithCountValue(inner, _, _),
        ) => validate_target(game, target, inner, ctx),
        (ResolvedTarget::Object(id), ChooseSpec::Object(filter)) => {
            let filter_ctx_for_candidate = || {
                let mut candidate_ctx = filter_ctx.clone();
                candidate_ctx
                    .target_objects
                    .retain(|snapshot| snapshot.object_id != *id);
                if let Some(object) = game.object(*id) {
                    candidate_ctx
                        .target_objects
                        .retain(|snapshot| snapshot.stable_id != object.stable_id);
                } else if let Some(snapshot) = ctx.target_snapshots.get(id) {
                    candidate_ctx
                        .target_objects
                        .retain(|target| target.stable_id != snapshot.stable_id);
                }
                candidate_ctx
            };
            if let Some(obj) = game.object(*id) {
                filter.matches(obj, &filter_ctx_for_candidate(), game)
            } else if let Some(snapshot) = ctx.target_snapshots.get(id) {
                filter.matches_snapshot(snapshot, &filter_ctx_for_candidate(), game)
            } else {
                false
            }
        }
        (ResolvedTarget::Player(id), ChooseSpec::Player(filter)) => {
            player_filter_matches_game(filter, *id, game, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::ObjectOrPlayer(filter, _)) => {
            let mut candidate_ctx = filter_ctx.clone();
            candidate_ctx
                .target_objects
                .retain(|snapshot| snapshot.object_id != *id);
            if let Some(object) = game.object(*id) {
                candidate_ctx
                    .target_objects
                    .retain(|snapshot| snapshot.stable_id != object.stable_id);
                filter.matches(object, &candidate_ctx, game)
            } else if let Some(snapshot) = ctx.target_snapshots.get(id) {
                candidate_ctx
                    .target_objects
                    .retain(|target| target.stable_id != snapshot.stable_id);
                filter.matches_snapshot(snapshot, &candidate_ctx, game)
            } else {
                false
            }
        }
        (ResolvedTarget::Player(id), ChooseSpec::ObjectOrPlayer(_, filter)) => {
            player_filter_matches_game(filter, *id, game, &filter_ctx)
        }
        (ResolvedTarget::Player(id), ChooseSpec::PlayerOrPlaneswalker(filter)) => {
            player_filter_matches_game(filter, *id, game, &filter_ctx)
        }
        (ResolvedTarget::Object(id), ChooseSpec::PlayerOrPlaneswalker(_)) => {
            (game.object(*id).is_some() && game.current_has_card_type(*id, CardType::Planeswalker))
                || ctx
                    .target_snapshots
                    .get(id)
                    .is_some_and(|snapshot| snapshot.card_types.contains(&CardType::Planeswalker))
        }
        (ResolvedTarget::Object(id), ChooseSpec::AnyTarget) => {
            game.object(*id).is_some() || ctx.target_snapshots.contains_key(id)
        }
        (ResolvedTarget::Player(id), ChooseSpec::AnyTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
        }
        (ResolvedTarget::Object(id), ChooseSpec::AnyOtherTarget) => {
            game.object(*id).is_some_and(|obj| obj.id != ctx.source)
                || ctx
                    .target_snapshots
                    .get(id)
                    .is_some_and(|snapshot| snapshot.object_id != ctx.source)
        }
        (ResolvedTarget::Player(id), ChooseSpec::AnyOtherTarget) => {
            game.player(*id).is_some_and(|p| p.is_in_game())
        }
        (ResolvedTarget::Object(id), ChooseSpec::SpecificObject(expected)) => id == expected,
        (ResolvedTarget::Player(id), ChooseSpec::SpecificPlayer(expected)) => id == expected,
        _ => false,
    }
}

// ============================================================================
// Selection Resolution
// ============================================================================

fn resolve_primary_object_from_value_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<ObjectId, ExecutionError> {
    let objects = resolve_objects_from_spec(game, spec, ctx)?;
    objects
        .first()
        .copied()
        .ok_or(ExecutionError::InvalidTarget)
}

/// Result shaping policy for applying operations to selected objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectApplyResultPolicy {
    /// Return `Count(applied_count)`.
    CountApplied,
    /// Return `Resolved` when at least one object was selected, else `TargetInvalid`.
    ///
    /// This preserves single-target semantics used by effects that resolve even when
    /// a selected object is no longer present or no state change occurred.
    SingleTargetResolvedOrInvalid,
}

/// Summary from applying an operation across selected objects.
#[derive(Debug)]
pub struct ObjectApplyResult {
    pub selected_count: usize,
    pub applied_count: usize,
    pub outcome: EffectOutcome,
}

/// Whether a tagged relation names the tagged objects themselves (the
/// candidates *are* the tagged set) rather than relating other objects to it.
pub(crate) fn tagged_relation_names_members(
    relation: crate::filter::TaggedOpbjectRelation,
) -> bool {
    matches!(
        relation,
        crate::filter::TaggedOpbjectRelation::IsTaggedObject
            | crate::filter::TaggedOpbjectRelation::SameObjectId
            | crate::filter::TaggedOpbjectRelation::SameStableId
            | crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
    )
}

fn filter_names_tagged_members(filter: &crate::filter::ObjectFilter) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| tagged_relation_names_members(constraint.relation))
}

fn candidate_object_ids_for_filter(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    let filter_ctx = ctx.filter_context(game);
    candidate_ids_for_filter(game, filter)
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
        .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
        .map(|(id, _)| id)
        .collect()
}

pub fn resolve_objects_for_effect(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
) -> Result<Vec<ObjectId>, ExecutionError> {
    resolve_objects_for_effect_with_choice_description(game, ctx, spec, None)
}

pub fn resolve_objects_for_effect_with_choice_description(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    choice_description: Option<String>,
) -> Result<Vec<ObjectId>, ExecutionError> {
    if !spec.is_target()
        && let ChooseSpec::Object(filter) = spec.base()
        && filter.source
    {
        // An explicitly qualified source is a reference, never a choice from
        // the named zone. Reuse the exact source/death-arrival resolver and
        // preserve its zone and owner predicates instead of scanning by name
        // or stable identity after another zone change.
        return resolve_objects_from_spec(game, spec, ctx);
    }
    if let ChooseSpec::Object(filter) | ChooseSpec::All(filter) = spec.base()
        && !crate::object_query::require_captured_public_collections(
            game,
            filter,
            &ctx.filter_context(game),
        )
    {
        return Err(ExecutionError::IncompleteEvidence(
            "public destination reference requires its producer collection".into(),
        ));
    }
    if !spec.is_target()
        && let ChooseSpec::Object(filter) = spec.base()
    {
        if !ctx.targets.is_empty()
            && matches!(spec.base(), ChooseSpec::Object(_))
            && !matches!(spec, ChooseSpec::WithCount(_, _))
            && filter.tagged_constraints.is_empty()
            && let Ok(objects) = resolve_objects_from_spec(game, spec, ctx)
            && !objects.is_empty()
        {
            return Ok(objects);
        }

        // Membership constraints ("that creature", "those cards") name the
        // tagged objects themselves; there is nothing to choose. Relational
        // constraints ("another creature", "a creature that shares a type
        // with it") only narrow an ordinary choice among matching objects.
        if filter_names_tagged_members(filter)
            && !matches!(
                spec,
                ChooseSpec::WithCount(..) | ChooseSpec::WithCountValue(..)
            )
        {
            return resolve_objects_from_spec(game, spec, ctx);
        }

        let count = spec.count();
        let resolved_dynamic_count = if count.is_dynamic_x() {
            if let Some(count_value) = spec.count_value() {
                Some(resolve_value(game, count_value, ctx)?.max(0) as usize)
            } else {
                None
            }
        } else {
            None
        };
        let mut candidates = candidate_object_ids_for_filter(game, filter, ctx);
        // A choice among hidden hand cards must look the same on every peer:
        // the owner knows which cards match, peers hold placeholders. Keep the
        // placeholders choosable and never skip, auto-pick, or fail the choice
        // because of the local filter result (see
        // `game_state::hidden_hand_choices`).
        let hidden_filter_ctx = ctx.filter_context(game);
        // A random pick among qualifying hand cards: every peer must draw it
        // from the same set, before any bound or early exit reads its size.
        if count.is_random() && filter.zone == Some(crate::zone::Zone::Hand) {
            let hand_ids = game.all_hand_card_ids();
            if !game.settle_hidden_hand_random_pool(
                &mut *ctx.decision_maker,
                ctx.source,
                filter,
                &hidden_filter_ctx,
                &hand_ids,
                &mut candidates,
            ) {
                return Ok(Vec::new());
            }
        }
        let hidden_hand_choice =
            !count.is_random() && game.hidden_hand_choice_for_filter(filter, &hidden_filter_ctx);
        if hidden_hand_choice {
            for id in game.hidden_hand_placeholder_candidates(
                filter,
                &hidden_filter_ctx,
                game.all_hand_card_ids(),
            ) {
                if !candidates.contains(&id) {
                    candidates.push(id);
                }
            }
        }
        if candidates.is_empty() && !hidden_hand_choice {
            if count.min == 0 || count.is_random() || resolved_dynamic_count.is_some() {
                return Ok(Vec::new());
            }
            return Err(ExecutionError::InvalidTarget);
        }
        if resolved_dynamic_count == Some(0) {
            return Ok(Vec::new());
        }

        let (min, max) = if count.is_dynamic_x() {
            let x = if let Some(x) = resolved_dynamic_count {
                x
            } else {
                ctx.x_value.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("X value not set".to_string())
                })? as usize
            };
            if count.is_up_to_dynamic_x() {
                (0, x.min(candidates.len()))
            } else if spec.count_value().is_some() || count.is_random() {
                let bounded = x.min(candidates.len());
                (bounded, bounded)
            } else if x > candidates.len() && !hidden_hand_choice {
                return Err(ExecutionError::InvalidTarget);
            } else {
                (x.min(candidates.len()), x.min(candidates.len()))
            }
        } else {
            (
                count.min.min(candidates.len()),
                count.max.unwrap_or(candidates.len()),
            )
        };

        if count.is_random() {
            game.shuffle_slice(&mut candidates);
            if filter.distinct_mana_values {
                candidates =
                    normalize_chosen_distinct_mana_values(game, candidates, &[], min, max, false);
            } else {
                candidates.truncate(max);
            }
            if filter.one_per_card_type {
                candidates = normalize_chosen_one_per_card_type(
                    game,
                    candidates,
                    &[],
                    min,
                    max,
                    false,
                    &filter.card_types,
                );
            }
            if candidates.len() < min {
                return Err(ExecutionError::InvalidTarget);
            }
            return Ok(candidates);
        }

        if candidates.len() < min && !hidden_hand_choice {
            return Err(ExecutionError::InvalidTarget);
        }

        if ctx.targets_are_cost_choices && !ctx.targets.is_empty() {
            let mut chosen = Vec::new();
            for target in &ctx.targets {
                if let ResolvedTarget::Object(object_id) = target
                    && candidates.contains(object_id)
                    && !chosen.contains(object_id)
                {
                    chosen.push(*object_id);
                }
            }
            if chosen.len() >= min && chosen.len() <= max {
                if !filter.one_per_card_type && !filter.distinct_mana_values {
                    return Ok(chosen);
                }
                let normalized = if filter.distinct_mana_values {
                    normalize_chosen_distinct_mana_values(
                        game,
                        chosen.clone(),
                        &candidates,
                        min,
                        max,
                        false,
                    )
                } else {
                    chosen.clone()
                };
                let normalized = if filter.one_per_card_type {
                    normalize_chosen_one_per_card_type(
                        game,
                        normalized,
                        &candidates,
                        min,
                        max,
                        false,
                        &filter.card_types,
                    )
                } else {
                    normalized
                };
                if normalized.len() == chosen.len() {
                    return Ok(normalized);
                }
                return Err(ExecutionError::InvalidTarget);
            }
            if !chosen.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }
        }

        if candidates.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.zone.is_hidden())
        }) {
            let description = choice_description
                .clone()
                .unwrap_or_else(|| format!("Choose {}", filter.description()));
            let choosing_player = ctx.iteration.iterated_player.unwrap_or(ctx.controller);
            view_hidden_candidate_objects(
                game,
                ctx,
                choosing_player,
                &candidates,
                description,
                false,
            );
        }

        if candidates.len() == 1 && min == 1 && max == 1 && !hidden_hand_choice {
            return Ok(candidates);
        }

        let description =
            choice_description.unwrap_or_else(|| format!("Choose {}", filter.description()));
        let choosing_player = ctx.iteration.iterated_player.unwrap_or(ctx.controller);
        let mut choice_spec = ChooseObjectsSpec::new(
            ctx.source,
            description.clone(),
            candidates.clone(),
            min,
            Some(max),
        );
        if hidden_hand_choice {
            choice_spec = choice_spec
                .allow_partial_completion()
                .require_explicit_choice();
        }
        let chosen: Vec<ObjectId> = make_decision(
            game,
            ctx.decision_maker,
            choosing_player,
            Some(ctx.source),
            choice_spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }

        if hidden_hand_choice {
            // No fill-to-minimum and no identity-dependent normalization: both
            // would rewrite the choice differently on peers holding
            // placeholders. Chosen placeholders are validated once opened.
            let chosen = normalize_objects_for_count(chosen, &candidates, 0, max);
            game.record_hidden_identity_obligations(
                &chosen,
                filter,
                &hidden_filter_ctx,
                &description,
            );
            return Ok(chosen);
        }
        let chosen = normalize_objects_for_count(chosen, &candidates, min, max);
        let chosen = if filter.distinct_mana_values {
            normalize_chosen_distinct_mana_values(game, chosen, &candidates, min, max, true)
        } else {
            chosen
        };
        let chosen = if filter.one_per_card_type {
            normalize_chosen_one_per_card_type(
                game,
                chosen,
                &candidates,
                min,
                max,
                true,
                &filter.card_types,
            )
        } else {
            chosen
        };
        if chosen.len() < min {
            return Err(ExecutionError::InvalidTarget);
        }
        return Ok(chosen);
    }

    // A bounded choice from an already tagged set is still a new choice; the
    // count wrapper must not disappear merely because the inner tagged spec
    // resolves to every remembered object. This is used by exact partitions
    // such as "return two of the cards ... and put the rest ...".
    if !spec.is_target()
        && let ChooseSpec::WithCount(inner, count) = spec
        && matches!(inner.base(), ChooseSpec::Tagged(_))
        && !count.dynamic_x
        && !count.random
    {
        let candidates = resolve_objects_from_spec(game, inner, ctx)?;
        let min = count.min.min(candidates.len());
        let max = count.max.unwrap_or(candidates.len()).min(candidates.len());
        if candidates.len() < count.min || max < min {
            return Err(ExecutionError::InvalidTarget);
        }
        if candidates.len() == min && min == max {
            return Ok(candidates);
        }
        let choosing_player = ctx.iteration.iterated_player.unwrap_or(ctx.controller);
        let description = choice_description.unwrap_or_else(|| "Choose cards".to_string());
        let chosen = make_decision(
            game,
            ctx.decision_maker,
            choosing_player,
            Some(ctx.source),
            ChooseObjectsSpec::new(ctx.source, description, candidates.clone(), min, Some(max)),
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        let chosen = normalize_objects_for_count(chosen, &candidates, min, max);
        if chosen.len() < min {
            return Err(ExecutionError::InvalidTarget);
        }
        return Ok(chosen);
    }

    resolve_objects_from_spec(game, spec, ctx)
}

pub fn resolve_single_object_for_effect(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
) -> Result<ObjectId, ExecutionError> {
    resolve_objects_for_effect(game, ctx, spec)?
        .into_iter()
        .next()
        .ok_or(ExecutionError::InvalidTarget)
}

/// Resolve the object an effect acts *from* — a damage source, "it deals
/// damage" rebinding — last-known-information first.
///
/// A tagged object that has left its zone (bounced, destroyed, or its
/// graveyard card exiled after a dies trigger) still deals the damage its
/// ability describes, using the characteristics it last had
/// (CR 608.2h, 113.7a). The live lookup only decides *which* object id the
/// source is when the tagged card may still be followed; the tagged snapshot
/// is authoritative for the characteristics, and it alone suffices when no
/// live object remains.
///
/// A pending or empty source choice has no binding. Locked references may
/// still resolve from tagged last known information after departure.
pub(crate) fn resolve_effect_source_with_lki(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
) -> Option<(ObjectId, Option<ObjectSnapshot>)> {
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    let selected = resolve_single_object_for_effect(game, ctx, spec).ok();
    if ctx.decision_maker.awaiting_choice()
        || (selected.is_none() && !source_binding_is_locked(spec))
    {
        return None;
    }
    retain_effect_source_lki(game, ctx, spec, selected)
}

/// Immutable source specifications refer to announced/locked objects. Share
/// the same LKI precedence with the chooser-bearing live resolver.
pub(crate) fn resolve_effect_source_from_spec_with_lki(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
) -> Option<(ObjectId, Option<ObjectSnapshot>)> {
    let selected = resolve_single_object_from_spec(game, spec, ctx).ok();
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    retain_effect_source_lki(game, ctx, spec, selected)
}

/// A bounded selection from tagged objects is a new source choice, even
/// though its base specification names an existing collection. Keep the same
/// capability contract for live LKI fallback and prepared source adapters.
pub(crate) fn source_binding_is_locked(spec: &ChooseSpec) -> bool {
    if spec.is_target() {
        return true;
    }
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => source_binding_is_locked(spec),
        ChooseSpec::WithCount(inner, _) | ChooseSpec::WithCountValue(inner, _, _)
            if matches!(inner.base(), ChooseSpec::Tagged(_)) =>
        {
            false
        }
        _ => matches!(
            spec.base(),
            ChooseSpec::Source
                | ChooseSpec::SpecificObject(_)
                | ChooseSpec::Tagged(_)
                | ChooseSpec::Iterated
        ),
    }
}

fn retain_effect_source_lki(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
    selected: Option<ObjectId>,
) -> Option<(ObjectId, Option<ObjectSnapshot>)> {
    let live = selected.filter(|id| game.object(*id).is_some());
    let tagged = match spec.base() {
        ChooseSpec::Tagged(tag) => {
            let history = ctx.get_tagged_all(format!("__pre_move_history__{}", tag.as_str()));
            let current = ctx.get_tagged_all(tag);
            let snapshots = history
                .into_iter()
                .flatten()
                .chain(current.into_iter().flatten());
            match selected {
                Some(id) => snapshots
                    .filter(|snapshot| {
                        snapshot.object_id == id
                            || game
                                .object(id)
                                .is_some_and(|object| object.stable_id == snapshot.stable_id)
                    })
                    .next(),
                None => snapshots.into_iter().next(),
            }
            .cloned()
        }
        _ => None,
    };
    let departed = tagged
        .as_ref()
        .filter(|snapshot| {
            game.object(snapshot.object_id)
                .is_none_or(|object| object.zone != snapshot.zone)
        })
        .map(|snapshot| {
            latest_tagged_lki_snapshot(game, snapshot)
                .unwrap_or(snapshot)
                .clone()
        });
    match (live, tagged) {
        // The tagged object left the zone it was tagged in before this
        // resolution: its new incarnation is a new object (CR 400.7), so the
        // source is the object as it last existed, not the card it became.
        // A card this resolution moved itself (CR 400.7j) is still followed.
        (Some(id), Some(snapshot))
            if id != snapshot.object_id
                && departed.is_some()
                && ctx
                    .resolution_object_id_floor
                    .is_none_or(|floor| id.0 < floor.0) =>
        {
            let lki = departed.unwrap_or(snapshot);
            Some((lki.object_id, Some(lki)))
        }
        (Some(id), tagged) => Some((id, tagged)),
        (None, Some(snapshot)) => {
            let lki = departed.unwrap_or(snapshot);
            Some((lki.object_id, Some(lki)))
        }
        (None, None) => None,
    }
}

/// The tagged snapshot a characteristic reader must use instead of a live
/// object: the tagged object has left the zone it was tagged in (CR 608.2h).
pub(crate) fn tagged_lki_when_object_left<'a>(
    game: &'a GameState,
    ctx: &'a ExecutionContext<'_>,
    spec: &ChooseSpec,
) -> Option<&'a ObjectSnapshot> {
    let ChooseSpec::Tagged(tag) = spec.base() else {
        return None;
    };
    if let Some(snapshot) = ctx.get_tagged(format!("__paid_departure__{}", tag.as_str())) {
        return Some(snapshot);
    }
    let snapshot = ctx.get_tagged(tag)?;
    game.object(snapshot.object_id)
        .is_none_or(|object| object.zone != snapshot.zone)
        .then(|| latest_tagged_lki_snapshot(game, snapshot).unwrap_or(snapshot))
}

fn normalize_objects_for_count(
    mut chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
) -> Vec<ObjectId> {
    let mut normalized = Vec::new();
    for id in chosen.drain(..) {
        if normalized.len() == max {
            break;
        }
        if candidates.contains(&id) && !normalized.contains(&id) {
            normalized.push(id);
        }
    }

    if normalized.len() < min {
        for id in candidates {
            if normalized.len() >= min {
                break;
            }
            if !normalized.contains(id) {
                normalized.push(*id);
            }
        }
    }

    normalized
}

fn card_type_assignment_exists(
    game: &GameState,
    chosen: &[ObjectId],
    slot_types: &[CardType],
) -> bool {
    fn assign_card(
        game: &GameState,
        id: ObjectId,
        slot_types: &[CardType],
        visited_types: &mut HashSet<CardType>,
        assigned: &mut HashMap<CardType, ObjectId>,
    ) -> bool {
        let card_types = game
            .current_card_types(id)
            .or_else(|| game.object(id).map(|object| object.card_types.to_vec()))
            .unwrap_or_default();
        for card_type in card_types {
            // "one card of each permanent type": only the listed types are
            // slots (a kindred card's kindred type isn't a permanent type).
            if !slot_types.is_empty() && !slot_types.contains(&card_type) {
                continue;
            }
            if !visited_types.insert(card_type) {
                continue;
            }
            let previous = assigned.get(&card_type).copied();
            if previous.is_none()
                || previous.is_some_and(|previous| {
                    assign_card(game, previous, slot_types, visited_types, assigned)
                })
            {
                assigned.insert(card_type, id);
                return true;
            }
        }
        false
    }

    let mut assigned = HashMap::new();
    for &id in chosen {
        if !assign_card(game, id, slot_types, &mut HashSet::new(), &mut assigned) {
            return false;
        }
    }
    true
}

/// Normalize a selection so every chosen card can occupy a different
/// card-type slot. Multitype cards are reassigned through bipartite matching,
/// allowing (for example) an artifact creature and a creature card to occupy
/// the artifact and creature slots respectively.
pub(crate) fn normalize_chosen_one_per_card_type(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
    slot_types: &[CardType],
) -> Vec<ObjectId> {
    let mut normalized = Vec::new();
    for id in chosen {
        if normalized.len() >= max || normalized.contains(&id) {
            continue;
        }
        normalized.push(id);
        if !card_type_assignment_exists(game, &normalized, slot_types) {
            normalized.pop();
        }
    }

    if fill_to_min && normalized.len() < min {
        for &id in candidates {
            if normalized.len() >= min || normalized.len() >= max || normalized.contains(&id) {
                continue;
            }
            normalized.push(id);
            if !card_type_assignment_exists(game, &normalized, slot_types) {
                normalized.pop();
            }
        }
    }

    normalized
}

pub(crate) fn normalize_chosen_distinct_mana_values(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
) -> Vec<ObjectId> {
    let mana_value = |id: ObjectId| {
        game.object(id).map(|object| {
            object
                .mana_cost
                .as_ref()
                .map_or(0, |cost| cost.mana_value())
        })
    };
    let mut used = HashSet::new();
    let mut normalized = Vec::new();
    for id in chosen {
        if normalized.len() >= max {
            break;
        }
        if mana_value(id).is_some_and(|value| used.insert(value)) {
            normalized.push(id);
        }
    }
    if fill_to_min && normalized.len() < min {
        for id in candidates {
            if normalized.len() >= min || normalized.len() >= max {
                break;
            }
            if mana_value(*id).is_some_and(|value| used.insert(value)) {
                normalized.push(*id);
            }
        }
    }
    normalized
}

/// Resolve objects from `spec`, apply an operation per object, and shape the result.
/// CR 603.10a / 603.6c: objects that one instruction moves out of a zone move
/// simultaneously, so every zone-change event of that instruction looks back
/// at the same pre-event trigger sources. A leaves-the-battlefield observer
/// removed together with other permanents (Angelic Sleuth under Farewell)
/// sees all of them leave, not only the ones processed before it.
///
/// Returns whether this call pinned the look-back; pass the result to
/// [`end_simultaneous_zone_change_lookback`]. An enclosing pin (a sacrifice
/// batch, state-based actions) is reused.
pub(crate) fn begin_simultaneous_zone_change_lookback(game: &mut GameState) -> bool {
    if game.simultaneous_event_lookback().is_some()
        || ![
            crate::events::EventKind::ZoneChange,
            crate::events::EventKind::Destroy,
            crate::events::EventKind::Sacrifice,
        ]
        .into_iter()
        .any(|kind| game.may_have_triggered_abilities_for_event_kind(kind))
    {
        return false;
    }
    let lookback = game.trigger_source_lookback_snapshots();
    game.set_simultaneous_event_lookback(Some(lookback));
    true
}

pub(crate) fn end_simultaneous_zone_change_lookback(game: &mut GameState, pinned: bool) {
    if pinned {
        game.set_simultaneous_event_lookback(None);
    }
}

pub fn apply_to_selected_objects(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    result_policy: ObjectApplyResultPolicy,
    apply: impl FnMut(&mut GameState, &mut ExecutionContext, ObjectId) -> Result<bool, ExecutionError>,
) -> Result<ObjectApplyResult, ExecutionError> {
    apply_to_selected_objects_with_choice_description(game, ctx, spec, result_policy, None, apply)
}

pub fn apply_to_selected_objects_with_choice_description(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    result_policy: ObjectApplyResultPolicy,
    choice_description: Option<String>,
    mut apply: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        ObjectId,
    ) -> Result<bool, ExecutionError>,
) -> Result<ObjectApplyResult, ExecutionError> {
    apply_to_selected_objects_with_prepared(
        game,
        ctx,
        spec,
        result_policy,
        choice_description,
        None,
        apply,
    )
}

pub(crate) fn apply_to_selected_objects_with_prepared(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    result_policy: ObjectApplyResultPolicy,
    choice_description: Option<String>,
    prepared: Option<Vec<ObjectId>>,
    mut apply: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        ObjectId,
    ) -> Result<bool, ExecutionError>,
) -> Result<ObjectApplyResult, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(ObjectApplyResult {
            selected_count: 0,
            applied_count: 0,
            outcome: EffectOutcome::count(0),
        });
    }
    crate::effects::composition::execute_result_checkpoint_transaction(game, ctx, |game, ctx| {
        let objects = if let Some(objects) = prepared {
            objects
        } else {
            resolve_objects_for_effect_with_choice_description(game, ctx, spec, choice_description)?
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(ObjectApplyResult {
                selected_count: 0,
                applied_count: 0,
                outcome: EffectOutcome::count(0),
            });
        }
        let selected_count = objects.len();
        let mut applied_count = 0usize;

        for object_id in objects {
            let applied = apply(game, ctx, object_id)?;
            // A callback can suspend inside a replacement program. Do not invoke
            // later object callbacks or report an uncommitted successful prefix.
            if ctx.decision_maker.awaiting_choice() {
                return Ok(ObjectApplyResult {
                    selected_count: 0,
                    applied_count: 0,
                    outcome: EffectOutcome::count(0),
                });
            }
            if applied {
                applied_count += 1;
            }
        }

        let outcome = match result_policy {
            ObjectApplyResultPolicy::CountApplied => EffectOutcome::count(applied_count as i32),
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid => {
                if selected_count > 0 {
                    EffectOutcome::resolved()
                } else {
                    EffectOutcome::target_invalid()
                }
            }
        };

        Ok(ObjectApplyResult {
            selected_count,
            applied_count,
            outcome,
        })
    })
}

/// Apply a single-target object operation using `ctx.targets` semantics.
///
/// This preserves the common single-target behavior:
/// - first object target is used,
/// - `None` means success (`Resolved`),
/// - `Some(result)` means short-circuit with that result,
/// - no object targets means `TargetInvalid`.
pub fn apply_single_target_object_from_context(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut apply: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        ObjectId,
    ) -> Result<Option<OutcomeStatus>, ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    for target in ctx.targets.clone() {
        if let ResolvedTarget::Object(object_id) = target {
            if let Some(status) = apply(game, ctx, object_id)? {
                return Ok(EffectOutcome::from_status(status));
            }
            return Ok(EffectOutcome::resolved());
        }
    }

    Ok(EffectOutcome::target_invalid())
}

/// Apply a single-target object operation using `spec` to pick the matching
/// object from `ctx.targets`.
pub fn apply_single_target_object_from_spec(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    mut apply: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        ObjectId,
    ) -> Result<Option<OutcomeStatus>, ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    let object_id = match resolve_single_object_from_spec(game, spec, ctx) {
        Ok(object_id) => object_id,
        Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
        Err(err) => return Err(err),
    };

    if let Some(status) = apply(game, ctx, object_id)? {
        return Ok(EffectOutcome::from_status(status));
    }

    Ok(EffectOutcome::resolved())
}

/// Resolve a ChooseSpec to a list of ObjectIds.
///
/// For targeted/chosen specs, returns the objects from ctx.targets.
/// For All specs, filters objects on the battlefield.
/// For Source, returns the source object.
/// For Iterated, returns the current iterated object.
pub fn resolve_objects_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<Vec<ObjectId>, ExecutionError> {
    if let ChooseSpec::Object(filter) | ChooseSpec::All(filter) = spec.base()
        && !crate::object_query::require_captured_public_collections(
            game,
            filter,
            &ctx.filter_context(game),
        )
    {
        return Err(ExecutionError::IncompleteEvidence(
            "public destination reference requires its producer collection".into(),
        ));
    }
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => resolve_objects_from_spec(game, spec, ctx),
        // Target wrapper - handle special cases then fall back to ctx.targets
        ChooseSpec::Target(inner) => {
            // Handle special cases where target is embedded in the spec
            match inner.base() {
                ChooseSpec::SpecificObject(id) => {
                    return Ok(vec![*id]);
                }
                ChooseSpec::Source => {
                    return resolve_source_object_id(game, ctx)
                        .map(|id| vec![id])
                        .ok_or(ExecutionError::InvalidTarget);
                }
                ChooseSpec::Tagged(tag) => {
                    let tagged = ctx
                        .get_tagged_all(tag)
                        .ok_or_else(|| ExecutionError::TagNotFound(tag.to_string()))?;
                    let objects: Vec<ObjectId> = tagged
                        .iter()
                        .filter_map(|snapshot| resolve_tagged_object_id(game, ctx, snapshot))
                        .collect();
                    if objects.is_empty() {
                        return Err(ExecutionError::InvalidTarget);
                    }
                    return Ok(objects);
                }
                _ => {}
            }

            let objects = matching_object_targets_for_spec(game, spec, ctx);

            if objects.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            Ok(objects)
        }
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _) => {
            if inner.is_target() {
                let objects = match resolve_objects_from_spec(game, inner, ctx) {
                    Ok(objects) => objects,
                    Err(ExecutionError::InvalidTarget) => {
                        if count.min == 0 && ctx.targets.is_empty() {
                            Vec::new()
                        } else {
                            return Err(ExecutionError::InvalidTarget);
                        }
                    }
                    Err(err) => return Err(err),
                };
                if objects.len() < count.min {
                    return Err(ExecutionError::InvalidTarget);
                }
                if let Some(max) = count.max
                    && objects.len() > max
                {
                    return Err(ExecutionError::InvalidTarget);
                }
                return Ok(objects);
            }

            if let ChooseSpec::Object(filter) = inner.base() {
                let filter_ctx = ctx.filter_context(game);
                let mut objects: Vec<ObjectId> = candidate_ids_for_filter(game, filter)
                    .iter()
                    .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect();

                let resolved_dynamic_count = if count.is_dynamic_x() {
                    if let Some(count_value) = spec.count_value() {
                        Some(resolve_value(game, count_value, ctx)?.max(0) as usize)
                    } else {
                        None
                    }
                } else {
                    None
                };

                if objects.is_empty() {
                    if resolved_dynamic_count.is_some() {
                        return Ok(objects);
                    }
                    return Err(ExecutionError::InvalidTarget);
                }

                let max = if count.is_dynamic_x() {
                    resolved_dynamic_count.unwrap_or_else(|| {
                        ctx.x_value
                            .map(|x| x as usize)
                            .or(count.max)
                            .unwrap_or(objects.len())
                    })
                } else {
                    count.max.unwrap_or(objects.len())
                };
                if count.is_random() {
                    game.shuffle_slice(&mut objects);
                }
                objects.truncate(max);
                if objects.len() < count.min {
                    return Err(ExecutionError::InvalidTarget);
                }
                return Ok(objects);
            }

            resolve_objects_from_spec(game, inner, ctx)
        }

        // Object filter (non-targeted choice) - generally supplied via previous selection,
        // but some tags only effects resolve from tagged objects and filters.
        ChooseSpec::Object(filter) if filter.source => {
            let Some(id) = resolve_source_object_id(game, ctx) else {
                return Ok(Vec::new());
            };
            let mut filter_ctx = ctx.filter_context(game);
            filter_ctx.source = Some(id);
            Ok(game
                .object(id)
                .filter(|object| filter.matches(object, &filter_ctx, game))
                .map(|_| vec![id])
                .unwrap_or_default())
        }
        ChooseSpec::Object(filter) => {
            if filter.tagged_constraints.is_empty() {
                let objects: Vec<ObjectId> = ctx
                    .targets
                    .iter()
                    .filter_map(|t| {
                        if let ResolvedTarget::Object(id) = t
                            && validate_target(game, t, spec, ctx)
                        {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();

                if objects.is_empty() {
                    return Err(ExecutionError::InvalidTarget);
                }

                return Ok(objects);
            }

            let filter_ctx = ctx.filter_context(game);
            let mut tagged_candidates = Vec::new();
            for constraint in &filter.tagged_constraints {
                // Only membership constraints name candidates; a relational
                // one (IsNotTaggedObject, SharesCardType, ...) must not limit
                // the pool to the very objects it compares against.
                if !tagged_relation_names_members(constraint.relation) {
                    continue;
                }
                if let Some(snapshots) = ctx.get_tagged_all(&constraint.tag) {
                    for snapshot in snapshots {
                        if let Some(object_id) = resolve_tagged_object_id(game, ctx, snapshot)
                            && !tagged_candidates.contains(&object_id)
                        {
                            tagged_candidates.push(object_id);
                        }
                    }
                }
            }
            // A tagged object this resolution can't find anymore (it changed
            // zones, CR 400.7) mustn't come back through the filter scan,
            // which matches tagged objects by stable identity.
            let tagged_objects_unfindable = tagged_candidates.is_empty()
                && filter.tagged_constraints.iter().any(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::IsTaggedObject
                            | crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
                    ) && ctx
                        .get_tagged_all(&constraint.tag)
                        .is_some_and(|snapshots| !snapshots.is_empty())
                });
            let candidate_ids = if tagged_objects_unfindable {
                Vec::new()
            } else if tagged_candidates.is_empty() {
                candidate_ids_for_filter(game, filter)
            } else {
                tagged_candidates
            };
            let objects: Vec<ObjectId> = candidate_ids
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
                .map(|obj| obj.id)
                .collect();

            if objects.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            Ok(objects)
        }

        ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::ObjectOrPlayer(_, _)
        | ChooseSpec::PlayerOrPlaneswalker(_) => {
            let objects: Vec<ObjectId> = ctx
                .targets
                .iter()
                .filter_map(|t| {
                    if let ResolvedTarget::Object(id) = t {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();

            if objects.is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            Ok(objects)
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            match attacked_target_from_trigger(ctx) {
                Some(AttackEventTarget::Planeswalker(object_id))
                | Some(AttackEventTarget::Battle(object_id)) => return Ok(vec![object_id]),
                Some(AttackEventTarget::Player(_)) | Some(AttackEventTarget::Nothing) | None => {}
            }
            Err(ExecutionError::InvalidTarget)
        }

        // All matching - filter battlefield
        ChooseSpec::All(filter) => {
            let filter_ctx = ctx.filter_context(game);
            // "Reveal the top seven cards ... put all cards with that name
            // among them into your hand": a zone-less filter drawn from a
            // tagged collection names those objects wherever they are, not
            // only permanents.
            let candidates = if filter.zone.is_none() && filter_names_tagged_members(filter) {
                let mut ids = Vec::new();
                for constraint in &filter.tagged_constraints {
                    if !tagged_relation_names_members(constraint.relation) {
                        continue;
                    }
                    for snapshot in ctx.get_tagged_all(&constraint.tag).into_iter().flatten() {
                        // Only objects still where the collection was
                        // tagged; a tagged permanent that left keeps the
                        // ordinary battlefield-only reading.
                        if let Some(id) = resolve_tagged_object_id(game, ctx, snapshot)
                            && game
                                .object(id)
                                .is_some_and(|object| object.zone == snapshot.zone)
                            && !ids.contains(&id)
                        {
                            ids.push(id);
                        }
                    }
                }
                ids
            } else {
                candidate_ids_for_filter(game, filter)
            };
            let objects: Vec<ObjectId> = candidates
                .iter()
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
                .filter(|(id, obj)| tagged_constraints_follow_permitted(ctx, filter, *id, obj))
                .map(|(id, _)| id)
                .collect();

            Ok(objects)
        }

        // Source reference
        ChooseSpec::Source => resolve_source_object_id(game, ctx)
            .map(|id| vec![id])
            .ok_or(ExecutionError::InvalidTarget),

        // Specific object
        ChooseSpec::SpecificObject(id) => Ok(vec![*id]),

        // Tagged objects
        ChooseSpec::Tagged(tag) => {
            let Some(tagged) = ctx.get_tagged_all(tag) else {
                return Ok(Vec::new());
            };
            Ok(tagged
                .iter()
                .filter_map(|snapshot| resolve_tagged_object_id(game, ctx, snapshot))
                .collect())
        }

        // Iterated object (ForEach loops)
        ChooseSpec::Iterated => ctx
            .iteration
            .iterated_object
            .map(|id| vec![id])
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "Iterated object not set (must be inside ForEach loop)".to_string(),
                )
            }),

        // Player specs can't be resolved to objects
        ChooseSpec::Player(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::EachPlayer(_) => Err(ExecutionError::UnresolvableValue(
            "Player spec cannot be resolved to objects".to_string(),
        )),
    }
}

/// Resolve a ChooseSpec to a list of PlayerIds.
///
/// For targeted/chosen player specs, returns the players from ctx.targets.
/// For EachPlayer specs, filters players in the game.
/// For SourceController, returns the controller.
/// For Iterated, returns the current iterated player.
pub fn resolve_players_from_spec(
    game: &GameState,
    spec: &ChooseSpec,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => resolve_players_from_spec(game, spec, ctx),
        // Target/WithCount wrappers - delegate to inner
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => {
            let players = matching_player_targets_for_spec(game, spec, ctx);

            if !players.is_empty() {
                return Ok(players);
            }
            if !matching_object_targets_for_spec(game, spec, ctx).is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            // A targeted object spec with no chosen target ("up to two target
            // creatures" with none chosen, or a target that became illegal)
            // refers to no player.
            if matches!(
                inner.base(),
                ChooseSpec::Object(_)
                    | ChooseSpec::SpecificObject(_)
                    | ChooseSpec::Tagged(_)
                    | ChooseSpec::All(_)
                    | ChooseSpec::AnyTarget
                    | ChooseSpec::AnyOtherTarget
            ) {
                return Ok(Vec::new());
            }
            // If no player targets, try to resolve the inner spec
            resolve_players_from_spec(game, inner, ctx)
        }

        // Player filter - resolve to matching players
        ChooseSpec::Player(filter)
        | ChooseSpec::ObjectOrPlayer(_, filter)
        | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            let players = matching_player_targets_for_spec(game, spec, ctx);

            if !players.is_empty() {
                return Ok(players);
            }
            if !matching_object_targets_for_spec(game, spec, ctx).is_empty() {
                return Err(ExecutionError::InvalidTarget);
            }

            // Fall back to filter resolution
            let filter_ctx = ctx.filter_context(game);
            resolve_player_filter_to_list(game, filter, &filter_ctx, ctx)
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => match attacked_target_from_trigger(ctx) {
            Some(AttackEventTarget::Player(player_id)) => Ok(vec![player_id]),
            Some(AttackEventTarget::Planeswalker(planeswalker_id)) => {
                let planeswalker = game
                    .object(planeswalker_id)
                    .ok_or(ExecutionError::ObjectNotFound(planeswalker_id))?;
                Ok(vec![game.controller_of(planeswalker)])
            }
            Some(AttackEventTarget::Battle(battle_id)) => game
                .battle_protector(battle_id)
                .map(|protector| vec![protector])
                .ok_or(ExecutionError::ObjectNotFound(battle_id)),
            // CR 506.4c: it isn't attacking any player or planeswalker.
            Some(AttackEventTarget::Nothing) => Ok(Vec::new()),
            None => {
                if let Some(defending) = ctx.combat.defending_player {
                    Ok(vec![defending])
                } else {
                    Err(ExecutionError::UnresolvableValue(
                        "Attacked player/planeswalker not set".to_string(),
                    ))
                }
            }
        },

        // Each player matching filter
        ChooseSpec::EachPlayer(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let players: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.is_in_game())
                .filter(|p| player_filter_matches_game(filter, p.id, game, &filter_ctx))
                .map(|p| p.id)
                .collect();

            Ok(players)
        }

        // Source controller ("you")
        ChooseSpec::SourceController => Ok(vec![ctx.controller]),

        // Source owner
        ChooseSpec::SourceOwner => {
            if let Some(obj) = game.object(ctx.source) {
                Ok(vec![obj.owner])
            } else if let Some(snapshot) = ctx.source_snapshot.as_ref() {
                Ok(vec![snapshot.owner])
            } else {
                Err(ExecutionError::ObjectNotFound(ctx.source))
            }
        }

        // Specific player
        ChooseSpec::SpecificPlayer(id) => Ok(vec![*id]),

        // Iterated player (ForEach loops)
        ChooseSpec::Iterated => ctx
            .iteration
            .iterated_player
            .map(|id| vec![id])
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "Iterated player not set (must be inside ForEach loop)".to_string(),
                )
            }),

        // "Any target" resolves to the players among the chosen targets
        // (the target-only path, e.g. a damage source that targeted a player).
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => {
            Ok(matching_player_targets_for_spec(game, spec, ctx))
        }

        // Object specs can't be resolved to players
        ChooseSpec::Object(_)
        | ChooseSpec::SpecificObject(_)
        | ChooseSpec::Source
        | ChooseSpec::Tagged(_)
        | ChooseSpec::All(_) => Err(ExecutionError::UnresolvableValue(
            "Object spec cannot be resolved to players".to_string(),
        )),
    }
}

/// Helper to resolve a PlayerFilter to a list of PlayerIds.
pub(crate) fn resolve_player_filter_to_list(
    game: &GameState,
    filter: &PlayerFilter,
    _filter_ctx: &FilterContext,
    ctx: &ExecutionContext,
) -> Result<Vec<PlayerId>, ExecutionError> {
    let mut players = match filter {
        PlayerFilter::You => Ok(vec![ctx.controller]),
        PlayerFilter::EffectController => Ok(vec![ctx.controller]),
        PlayerFilter::Any => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .map(|player| player.id)
            .collect()),
        PlayerFilter::Target(_) => {
            let targets = if ctx.targets.is_empty() && !ctx.targets_are_cost_choices {
                ctx.announced_targets.as_deref().unwrap_or(&ctx.targets)
            } else {
                &ctx.targets
            };
            let players = targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Player(id) => Some(*id),
                    ResolvedTarget::Object(_) => None,
                })
                .collect::<Vec<_>>();
            if players.is_empty() {
                Err(ExecutionError::InvalidTarget)
            } else {
                Ok(players)
            }
        }
        PlayerFilter::AliasedTarget(inner) => {
            let mut players = ctx
                .targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Player(id) => Some(*id),
                    ResolvedTarget::Object(_) => None,
                })
                .collect::<Vec<_>>();
            if players.is_empty()
                && let Some(delayed_players) =
                    ctx.get_tagged_players(crate::tag::DELAYED_TARGET_PLAYERS_TAG)
            {
                let filter_ctx = ctx.filter_context(game);
                players.extend(
                    delayed_players
                        .iter()
                        .copied()
                        .filter(|player| inner.matches_player(*player, &filter_ctx)),
                );
            }
            if players.is_empty() {
                Err(ExecutionError::InvalidTarget)
            } else {
                Ok(players)
            }
        }
        PlayerFilter::NotYou => {
            let others: Vec<PlayerId> = game
                .players
                .iter()
                .filter(|p| p.id != ctx.controller && p.is_in_game())
                .map(|p| p.id)
                .collect();
            Ok(others)
        }
        PlayerFilter::Opponent => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game() && game.are_opponents(ctx.controller, player.id))
            .map(|player| player.id)
            .collect()),
        PlayerFilter::Specific(id) => Ok(vec![*id]),
        PlayerFilter::PlayerToYourLeft | PlayerFilter::PlayerToYourRight => {
            Ok(vec![resolve_player_filter(game, filter, ctx)?])
        }
        PlayerFilter::MostLifeTied => {
            let max_life = game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && _filter_ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|players| players.contains(&player.id))
                })
                .map(|player| player.life)
                .max()
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue("No players are in the game".to_string())
                })?;
            Ok(game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && player.life == max_life
                        && _filter_ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|players| players.contains(&player.id))
                })
                .map(|player| player.id)
                .collect())
        }
        PlayerFilter::LowestLifeTied => {
            let min_life = game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && _filter_ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|players| players.contains(&player.id))
                })
                .map(|player| player.life)
                .min()
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue("No players are in the game".to_string())
                })?;
            Ok(game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && player.life == min_life
                        && _filter_ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|players| players.contains(&player.id))
                })
                .map(|player| player.id)
                .collect())
        }
        PlayerFilter::MostCardsInHand => {
            let max_hand = game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && _filter_ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|players| players.contains(&player.id))
                })
                .map(|player| player.hand.len())
                .max()
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue("No players are in the game".to_string())
                })?;
            let leaders = game
                .players
                .iter()
                .filter(|player| {
                    player.is_in_game()
                        && player.hand.len() == max_hand
                        && _filter_ctx
                            .players_in_range
                            .as_ref()
                            .is_none_or(|players| players.contains(&player.id))
                })
                .map(|player| player.id)
                .collect::<Vec<_>>();
            match leaders.as_slice() {
                [leader] => Ok(vec![*leader]),
                [] => Err(ExecutionError::UnresolvableValue(
                    "MostCardsInHand requires an in-game player".to_string(),
                )),
                _ => Err(ExecutionError::UnresolvableValue(
                    "MostCardsInHand requires a unique player".to_string(),
                )),
            }
        }
        PlayerFilter::CastCardTypeThisTurn(card_type) => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .filter(|player| {
                game.turn_store
                    .turn_history
                    .spell_cast_snapshot_history()
                    .iter()
                    .any(|snapshot| {
                        snapshot.controller == player.id && snapshot.card_types.contains(card_type)
                    })
            })
            .map(|player| player.id)
            .collect()),
        PlayerFilter::AttackedBySourceThisTurn => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .filter(|player| player_filter_matches_game(filter, player.id, game, _filter_ctx))
            .map(|player| player.id)
            .collect()),
        PlayerFilter::WasDealtDamageBySourceThisGame { .. }
        | PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. }
        | PlayerFilter::LostLifeThisTurn { .. } => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .filter(|player| player_filter_matches_game(filter, player.id, game, _filter_ctx))
            .map(|player| player.id)
            .collect()),
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. } => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .filter(|player| player_filter_matches_game(filter, player.id, game, _filter_ctx))
            .map(|player| player.id)
            .collect()),
        PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. }
        | PlayerFilter::OpponentWithMoreControlledObjectsThan { .. }
        | PlayerFilter::ControlsMost { .. }
        | PlayerFilter::ControlsFewestTied { .. } => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .filter(|player| player_filter_matches_game(filter, player.id, game, _filter_ctx))
            .map(|player| player.id)
            .collect()),
        PlayerFilter::OpponentOf(_) | PlayerFilter::MaxSpeed { .. } => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game())
            .filter(|player| player_filter_matches_game(filter, player.id, game, _filter_ctx))
            .map(|player| player.id)
            .collect()),
        PlayerFilter::ChosenPlayer => ctx
            .combat
            .chosen_player
            .or_else(|| game.chosen_player(ctx.source))
            .map(|id| vec![id])
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "ChosenPlayer requires a previously chosen player".to_string(),
                )
            }),
        PlayerFilter::TaggedPlayer(tag) => resolve_tagged_players_from_context(game, ctx, tag)
            .ok_or_else(|| {
                if tag.as_str() == ironsmith_core::tag::DAMAGE_SOURCE_CONTROLLER_TAG {
                    return ExecutionError::IncompleteEvidence(
                        "missing damage-time source controller".into(),
                    );
                }
                ExecutionError::UnresolvableValue(format!(
                    "TaggedPlayer requires a tagged player for '{tag}'"
                ))
            }),
        PlayerFilter::Active => game
            .singular_active_player(ctx.combat.chosen_player)
            .map(|id| vec![id])
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue("There is no active player".to_string())
            }),
        PlayerFilter::Defending => ctx.defending_players(game),
        PlayerFilter::Attacking => combat_attacking_player(game, ctx)
            .map(|id| vec![id])
            .ok_or_else(|| {
                ExecutionError::UnresolvableValue("AttackingPlayer not set".to_string())
            }),
        PlayerFilter::DamagedPlayer => {
            if let Some(triggering_event) = &ctx.triggering_event
                && let Some(damage_event) = triggering_event.downcast::<DamageEvent>()
                && let DamageTarget::Player(player_id) = damage_event.target
            {
                return Ok(vec![player_id]);
            }
            ctx.get_tagged_players("damaged_player")
                .and_then(|players| players.first().copied())
                .or_else(|| prior_effect_damaged_player(ctx))
                .map(|player_id| vec![player_id])
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "DamagedPlayer requires a player damage event".to_string(),
                    )
                })
        }
        PlayerFilter::IteratedPlayer => ctx
            .iteration
            .iterated_player
            .or(_filter_ctx.iterated_player)
            .or_else(|| {
                ctx.get_tagged_players("__it__")
                    .and_then(|players| players.first().copied())
            })
            .map(|id| vec![id])
            .ok_or_else(|| ExecutionError::UnresolvableValue("IteratedPlayer not set".to_string())),
        PlayerFilter::TargetPlayerOrControllerOfTarget => Ok(vec![resolve_player_filter(
            game,
            &PlayerFilter::TargetPlayerOrControllerOfTarget,
            ctx,
        )?]),
        PlayerFilter::Excluding { base, excluded } => {
            let mut base_players = resolve_player_filter_to_list(game, base, _filter_ctx, ctx)?;
            let excluded_players = resolve_player_filter_to_list(game, excluded, _filter_ctx, ctx)?;
            base_players.retain(|id| !excluded_players.contains(id));
            Ok(base_players)
        }
        PlayerFilter::ControllerOf(object_ref) | PlayerFilter::AliasedControllerOf(object_ref) => {
            Ok(vec![resolve_controller_of(game, ctx, object_ref)?])
        }
        PlayerFilter::OwnerOf(object_ref) | PlayerFilter::AliasedOwnerOf(object_ref) => {
            Ok(vec![resolve_owner_of(game, ctx, object_ref)?])
        }
        PlayerFilter::Teammate => Ok(game
            .players
            .iter()
            .filter(|player| player.is_in_game() && game.are_teammates(ctx.controller, player.id))
            .map(|player| player.id)
            .collect()),
    }?;
    if let Some(players_in_range) = &_filter_ctx.players_in_range {
        players.retain(|player| players_in_range.contains(player));
    }
    Ok(players)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::ColorSet;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::effect::ChoiceCount;
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn add_battlefield_permanent(
        game: &mut GameState,
        id_raw: u32,
        name: &str,
        controller: PlayerId,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let card = CardBuilder::new(crate::ids::CardId::from_raw(id_raw), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(card_types)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn add_hand_card(game: &mut GameState, id_raw: u32, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(crate::ids::CardId::from_raw(id_raw), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn add_custom_creature(
        game: &mut GameState,
        id_raw: u32,
        name: &str,
        owner: PlayerId,
        mana_value: u8,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(crate::ids::CardId::from_raw(id_raw), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn add_custom_permanent(
        game: &mut GameState,
        id_raw: u32,
        name: &str,
        owner: PlayerId,
        card_types: Vec<CardType>,
        colors: ColorSet,
    ) -> ObjectId {
        let card = CardBuilder::new(crate::ids::CardId::from_raw(id_raw), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(card_types)
            .color_indicator(colors)
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn distinct_mana_value_selection_rejects_duplicate_values() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let first_two = add_custom_creature(&mut game, 91_001, "First Two", alice, 2, 2, 2);
        let second_two = add_custom_creature(&mut game, 91_002, "Second Two", alice, 2, 2, 2);
        let three = add_custom_creature(&mut game, 91_003, "Three", alice, 3, 3, 3);

        assert_eq!(
            normalize_chosen_distinct_mana_values(
                &game,
                vec![first_two, second_two, three],
                &[],
                0,
                3,
                false,
            ),
            vec![first_two, three]
        );
    }

    fn add_typed_creature(
        game: &mut GameState,
        id_raw: u32,
        name: &str,
        owner: PlayerId,
        subtypes: Vec<Subtype>,
    ) -> ObjectId {
        let card = CardBuilder::new(crate::ids::CardId::from_raw(id_raw), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn metric_value(
        effect_id: crate::effect::EffectId,
        source: EffectMetricSource,
        metric: EffectMetric,
    ) -> Value {
        Value::EffectMetric {
            effect_id,
            source,
            metric,
        }
    }

    #[test]
    fn greatest_shared_creature_type_count_uses_largest_cohort_not_object_total() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();
        add_typed_creature(
            &mut game,
            420,
            "Elf Warrior",
            alice,
            vec![Subtype::Elf, Subtype::Warrior],
        );
        add_typed_creature(
            &mut game,
            421,
            "Elf Druid",
            alice,
            vec![Subtype::Elf, Subtype::Druid],
        );
        add_typed_creature(
            &mut game,
            422,
            "Goblin Warrior",
            alice,
            vec![Subtype::Goblin, Subtype::Warrior],
        );
        add_typed_creature(&mut game, 423, "Opponent Elf", bob, vec![Subtype::Elf]);

        let ctx = ExecutionContext::new_default(source_id, alice);
        let value = Value::GreatestSharedCreatureTypeCount(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
        );

        assert_eq!(
            resolve_value(&game, &value, &ctx).expect("shared creature-type count should resolve"),
            2,
            "Elf and Warrior each form a two-creature cohort; the Goblin and opposing Elf must not inflate it",
        );
    }

    #[test]
    fn effect_metric_resolves_count_from_outcome_chosen_and_affected_memory() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();
        let chosen = add_battlefield_permanent(
            &mut game,
            410,
            "Chosen Creature",
            alice,
            vec![CardType::Creature],
        );
        let affected_a = add_battlefield_permanent(
            &mut game,
            411,
            "Affected Creature A",
            alice,
            vec![CardType::Creature],
        );
        let affected_b = add_battlefield_permanent(
            &mut game,
            412,
            "Affected Creature B",
            alice,
            vec![CardType::Creature],
        );
        let chosen_memory = vec![ObjectSnapshot::from_object_id(&game, chosen).unwrap()];
        let affected_memory = vec![
            ObjectSnapshot::from_object_id(&game, affected_a).unwrap(),
            ObjectSnapshot::from_object_id(&game, affected_b).unwrap(),
        ];
        let effect_id = crate::effect::EffectId(17);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::count(9)
                .with_chosen_object_memory(chosen_memory)
                .with_affected_object_memory(affected_memory),
        );

        assert_eq!(
            resolve_value(
                &game,
                &metric_value(effect_id, EffectMetricSource::Outcome, EffectMetric::Count),
                &ctx,
            )
            .unwrap(),
            9
        );
        assert_eq!(
            resolve_value(
                &game,
                &metric_value(
                    effect_id,
                    EffectMetricSource::ChosenObjects,
                    EffectMetric::ChosenCount,
                ),
                &ctx,
            )
            .unwrap(),
            1
        );
        assert_eq!(
            resolve_value(
                &game,
                &metric_value(
                    effect_id,
                    EffectMetricSource::AffectedObjects,
                    EffectMetric::AffectedCount,
                ),
                &ctx,
            )
            .unwrap(),
            2
        );
    }

    #[test]
    fn effect_metric_resolves_lki_object_stats_after_objects_leave_battlefield() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();
        let creature_a = add_custom_creature(&mut game, 420, "First Creature", alice, 3, 4, 2);
        let creature_b = add_custom_creature(&mut game, 421, "Second Creature", alice, 6, 6, 5);
        let snapshots = [creature_a, creature_b]
            .into_iter()
            .map(|id| {
                let object = game.object(id).expect("creature should exist");
                ObjectSnapshot::from_object_with_calculated_characteristics(object, &game)
            })
            .collect::<Vec<_>>();
        game.move_object_by_effect(creature_a, Zone::Graveyard)
            .expect("first creature should move");
        game.move_object_by_effect(creature_b, Zone::Exile)
            .expect("second creature should move");
        let memory = snapshots.iter().map(Clone::clone).collect::<Vec<_>>();
        let effect_id = crate::effect::EffectId(18);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::count(2).with_affected_object_memory(memory),
        );

        for (metric, expected) in [
            (EffectMetric::FirstPower, 4),
            (EffectMetric::FirstToughness, 2),
            (EffectMetric::FirstManaValue, 3),
            (EffectMetric::TotalPower, 10),
            (EffectMetric::TotalToughness, 7),
            (EffectMetric::TotalManaValue, 9),
            (EffectMetric::GreatestPower, 6),
            (EffectMetric::GreatestToughness, 5),
            (EffectMetric::GreatestManaValue, 6),
        ] {
            assert_eq!(
                resolve_value(
                    &game,
                    &metric_value(effect_id, EffectMetricSource::AffectedObjects, metric),
                    &ctx,
                )
                .unwrap(),
                expected,
                "metric {metric:?} should resolve from stored LKI"
            );
        }
    }

    #[test]
    fn prior_effect_metric_selects_the_iterated_players_object_partition() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();
        let alice_creature = add_custom_creature(&mut game, 422, "Alice Creature", alice, 3, 4, 2);
        let bob_creature = add_custom_creature(&mut game, 423, "Bob Creature", bob, 6, 7, 5);
        let alice_memory = ObjectSnapshot::from_object_id(&game, alice_creature).unwrap();
        let bob_memory = ObjectSnapshot::from_object_id(&game, bob_creature).unwrap();
        let effect_id = crate::effect::EffectId(20);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::count(2)
                .with_affected_object_memory(vec![alice_memory.clone(), bob_memory.clone()])
                .with_player_affected_object_memory(vec![
                    (alice, vec![alice_memory]),
                    (bob, vec![bob_memory]),
                ]),
        );
        let query = crate::effect::PriorEffectMetricQuery::new(
            EffectMetricSource::AffectedObjects,
            EffectMetric::FirstPower,
        )
        .with_player(PlayerFilter::IteratedPlayer);
        let value = Value::PriorEffectMetric { effect_id, query };

        ctx.iteration.iterated_player = Some(alice);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 4);
        ctx.iteration.iterated_player = Some(bob);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 7);
    }

    #[test]
    fn effect_metric_resolves_colors_and_card_types_among_result_memory() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();
        let artifact_creature = add_custom_permanent(
            &mut game,
            430,
            "Blue Artifact Creature",
            alice,
            vec![CardType::Artifact, CardType::Creature],
            ColorSet::BLUE,
        );
        let red_enchantment = add_custom_permanent(
            &mut game,
            431,
            "Red Enchantment",
            alice,
            vec![CardType::Enchantment],
            ColorSet::RED,
        );
        let memory = [artifact_creature, red_enchantment]
            .into_iter()
            .map(|id| ObjectSnapshot::from_object_id(&game, id).unwrap())
            .collect::<Vec<_>>();
        let effect_id = crate::effect::EffectId(19);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::count(2).with_affected_object_memory(memory),
        );

        assert_eq!(
            resolve_value(
                &game,
                &metric_value(
                    effect_id,
                    EffectMetricSource::AffectedObjects,
                    EffectMetric::ColorsAmong,
                ),
                &ctx,
            )
            .unwrap(),
            2
        );
        assert_eq!(
            resolve_value(
                &game,
                &metric_value(
                    effect_id,
                    EffectMetricSource::AffectedObjects,
                    EffectMetric::CardTypesAmong,
                ),
                &ctx,
            )
            .unwrap(),
            3
        );
    }

    #[test]
    fn effect_metric_resolves_life_lost_from_stored_events() {
        use crate::events::life::LifeLossEvent;
        use crate::provenance::ProvNodeId;
        use crate::triggers::TriggerEvent;

        let game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = ObjectId(999);
        let effect_id = crate::effect::EffectId(19);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::resolved().with_events([
                TriggerEvent::new_with_provenance(
                    LifeLossEvent::from_effect(alice, 2),
                    ProvNodeId::default(),
                ),
                TriggerEvent::new_with_provenance(
                    LifeLossEvent::from_effect(bob, 3),
                    ProvNodeId::default(),
                ),
            ]),
        );

        assert_eq!(
            resolve_value(
                &game,
                &metric_value(
                    effect_id,
                    EffectMetricSource::Outcome,
                    EffectMetric::LifeLost
                ),
                &ctx,
            )
            .unwrap(),
            5
        );
    }

    #[test]
    fn effect_metric_sums_numeric_excess_damage_facts() {
        let game = new_test_game();
        let alice = game.players[0].id;
        let source_id = ObjectId(998);
        let effect_id = crate::effect::EffectId(20);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.store_outcome(
            effect_id,
            EffectOutcome::resolved()
                .with_execution_fact(crate::effect::ExecutionFact::ExcessDamage(2))
                .with_execution_fact(crate::effect::ExecutionFact::ExcessDamage(3)),
        );

        assert_eq!(
            resolve_value(
                &game,
                &metric_value(
                    effect_id,
                    EffectMetricSource::Outcome,
                    EffectMetric::ExcessDamage,
                ),
                &ctx,
            )
            .unwrap(),
            5
        );
    }

    #[test]
    fn triggering_object_mana_spent_prefers_spell_cast_snapshot() {
        use crate::events::spells::SpellCastEvent;
        use crate::player::ManaPool;
        use crate::provenance::ProvNodeId;
        use crate::triggers::TriggerEvent;

        let game = new_test_game();
        let alice = game.players[0].id;
        let source_id = ObjectId(997);
        let spell_id = ObjectId(996);
        let mut snapshot = ObjectSnapshot::for_testing(spell_id, alice, "Triggered Spell");
        snapshot.mana_spent_to_cast = ManaPool {
            blue: 2,
            red: 1,
            colorless: 2,
            ..ManaPool::default()
        };
        snapshot.caster_mana_spent_to_cast = Some(2);
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new_with_snapshot(spell_id, alice, Zone::Hand, snapshot),
            ProvNodeId::default(),
        );
        let ctx = ExecutionContext::new_default(source_id, alice).with_triggering_event(event);

        assert_eq!(
            resolve_value(&game, &Value::ManaSpentToCastTriggeringObject, &ctx).unwrap(),
            5
        );
        assert_eq!(
            resolve_value(&game, &Value::CasterManaSpentToCastTriggeringObject, &ctx).unwrap(),
            2
        );
        let mut unknown = ObjectSnapshot::for_testing(spell_id, alice, "Legacy payment");
        unknown.mana_spent_to_cast.colorless = 7;
        let unknown_event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new_with_snapshot(spell_id, alice, Zone::Hand, unknown),
            ProvNodeId::default(),
        );
        let unknown_ctx =
            ExecutionContext::new_default(source_id, alice).with_triggering_event(unknown_event);
        assert!(
            resolve_value(
                &game,
                &Value::CasterManaSpentToCastTriggeringObject,
                &unknown_ctx
            )
            .is_err(),
            "total payment cannot substitute for missing payer evidence"
        );
    }

    #[test]
    fn mana_symbol_spent_value_counts_only_that_symbol_and_composes_with_division() {
        use crate::player::ManaPool;

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = add_hand_card(&mut game, 399, "Colored Payment Spell", alice);
        game.object_mut(source_id)
            .expect("source spell should exist")
            .mana_spent_to_cast = ManaPool {
            blue: 5,
            red: 2,
            ..ManaPool::default()
        };
        let ctx = ExecutionContext::new_default(source_id, alice);
        let blue_pairs = Value::DividedRoundedDown(
            Box::new(Value::ManaSymbolSpentToCastThisSpell {
                symbol: ManaSymbol::Blue,
                reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
            }),
            2,
        );

        assert_eq!(resolve_value(&game, &blue_pairs, &ctx).unwrap(), 2);
    }

    #[test]
    fn mana_value_of_source_uses_lki_after_source_moves_from_expected_zone() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_card = CardBuilder::new(crate::ids::CardId::from_raw(400), "Departing Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source_id).expect("source should exist"),
            &game,
        );
        let moved_source_id = game
            .move_object_by_effect(source_id, Zone::Hand)
            .expect("source should move to hand");
        game.object_mut(moved_source_id)
            .expect("moved source should exist")
            .mana_cost = Some(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]).into());

        let ctx = ExecutionContext::new_default(moved_source_id, alice)
            .with_source_snapshot(source_snapshot);

        assert_eq!(
            resolve_value(
                &game,
                &Value::ManaValueOf(Box::new(ChooseSpec::Source)),
                &ctx
            )
            .expect("source mana value should resolve from LKI"),
            2,
            "608.2h requires source information to use LKI after the source leaves its expected zone"
        );
    }

    #[test]
    fn power_of_source_uses_lki_after_source_moves_by_stable_id() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_card = CardBuilder::new(crate::ids::CardId::from_raw(401), "Departing Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source should exist")
            .add_counters(crate::object::CounterType::PlusOnePlusOne, 3);
        let source_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source_id).expect("source should still exist"),
            &game,
        );
        let moved_source_id = game
            .move_object_by_effect(source_id, Zone::Graveyard)
            .expect("source should move to graveyard");

        assert_ne!(source_id, moved_source_id);
        assert_eq!(
            game.object(moved_source_id)
                .expect("moved source should exist")
                .power(),
            Some(2),
            "zone changes should clear counters from the current object"
        );

        let ctx =
            ExecutionContext::new_default(source_id, alice).with_source_snapshot(source_snapshot);
        assert_eq!(
            resolve_value(&game, &Value::PowerOf(Box::new(ChooseSpec::Source)), &ctx)
                .expect("source power should resolve from LKI"),
            5,
            "608.2h requires PowerOf(Source) to use LKI after the source moved"
        );
    }

    struct SelectIdsDecisionMaker {
        chosen: Vec<ObjectId>,
    }

    impl DecisionMaker for SelectIdsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chosen
                .iter()
                .copied()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    #[test]
    fn test_resolve_fixed_value() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id);

        let value = Value::Fixed(5);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 5);
    }

    #[test]
    fn aggregate_mana_symbols_sum_filtered_objects_at_resolution() {
        fn add_permanent(
            game: &mut GameState,
            id: u32,
            name: &str,
            controller: PlayerId,
            zone: Zone,
            mana_cost: Option<ManaCost>,
        ) {
            let mut builder = CardBuilder::new(crate::ids::CardId::from_raw(id), name)
                .card_types(vec![CardType::Enchantment]);
            if let Some(mana_cost) = mana_cost {
                builder = builder.mana_cost(mana_cost);
            }
            let card = builder.build();
            game.create_object_from_card(&card, controller, zone);
        }

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        add_permanent(
            &mut game,
            9150,
            "Double Green",
            alice,
            Zone::Battlefield,
            Some(ManaCost::from_symbols(vec![
                ManaSymbol::Green,
                ManaSymbol::Green,
            ])),
        );
        add_permanent(
            &mut game,
            9151,
            "Hybrid Green",
            alice,
            Zone::Battlefield,
            Some(ManaCost::from_pips(vec![
                vec![ManaSymbol::Green, ManaSymbol::White],
                vec![ManaSymbol::Generic(2), ManaSymbol::Green],
                vec![ManaSymbol::Green, ManaSymbol::Life(2)],
            ])),
        );
        add_permanent(
            &mut game,
            9152,
            "No Mana Cost",
            alice,
            Zone::Battlefield,
            None,
        );
        add_permanent(
            &mut game,
            9153,
            "Opponent Green",
            bob,
            Zone::Battlefield,
            Some(ManaCost::from_symbols(vec![
                ManaSymbol::Green,
                ManaSymbol::Green,
                ManaSymbol::Green,
            ])),
        );
        add_permanent(
            &mut game,
            9154,
            "Green in Hand",
            alice,
            Zone::Hand,
            Some(ManaCost::from_symbols(vec![
                ManaSymbol::Green,
                ManaSymbol::Green,
            ])),
        );

        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        let value = Value::ManaSymbolsInManaCostOf {
            spec: Box::new(ChooseSpec::All(ObjectFilter::permanent().you_control())),
            color: crate::color::Color::Green,
        };
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 5);
    }

    #[test]
    fn players_who_control_more_respects_the_player_domain() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let charlie = game.players[2].id;

        add_battlefield_permanent(
            &mut game,
            9101,
            "Alice Creature",
            alice,
            vec![CardType::Creature],
        );
        for (id, name) in [(9102, "Bob Creature A"), (9103, "Bob Creature B")] {
            add_battlefield_permanent(&mut game, id, name, bob, vec![CardType::Creature]);
        }
        for (id, name) in [
            (9104, "Charlie Creature A"),
            (9105, "Charlie Creature B"),
            (9106, "Charlie Creature C"),
        ] {
            add_battlefield_permanent(&mut game, id, name, charlie, vec![CardType::Creature]);
        }

        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, alice);
        let creatures = ObjectFilter::creature();

        for (players, expected) in [
            (PlayerFilter::Any, 2),
            (PlayerFilter::Opponent, 2),
            (PlayerFilter::Specific(bob), 1),
        ] {
            let value = Value::PlayersWhoControlMoreThanYou {
                players,
                filter: creatures.clone(),
            };
            assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), expected);
        }

        let at_least_two_more = Value::PlayersWhoControlAtLeastMoreThanYou {
            players: PlayerFilter::Opponent,
            filter: creatures,
            minimum_difference: 2,
        };
        assert_eq!(resolve_value(&game, &at_least_two_more, &ctx).unwrap(), 1);
    }

    #[test]
    fn test_resolve_x_value() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id).with_x(3);

        let value = Value::X;
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 3);
    }

    #[test]
    fn resolve_this_ability_resolved_this_turn_count_for_activated_ability() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = ObjectId(9001);
        let ability_index = 2;
        let ctx =
            ExecutionContext::new_default(source_id, player_id).with_ability_index(ability_index);

        game.record_activated_ability_resolved(source_id, ability_index);
        game.record_activated_ability_resolved(source_id, ability_index);

        assert_eq!(
            resolve_value(&game, &Value::ThisAbilityResolvedThisTurnCount, &ctx).unwrap(),
            2
        );
    }

    #[test]
    fn test_resolve_x_times_value() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id).with_x(3);

        let value = Value::XTimes(2);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 6);
    }

    #[test]
    fn resolve_commander_cast_count_tracks_only_command_zone_casts_for_controller() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();

        let commander_card = CardBuilder::new(crate::ids::CardId::from_raw(9901), "Commander")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        let alice_commander = game.create_object_from_card(&commander_card, alice, Zone::Command);
        game.set_as_commander(alice_commander, alice);
        let bob_commander = game.create_object_from_card(&commander_card, bob, Zone::Command);
        game.set_as_commander(bob_commander, bob);

        let alice_ctx = ExecutionContext::new_default(source_id, alice);
        assert_eq!(
            resolve_value(
                &game,
                &Value::CommanderCastCount(PlayerFilter::You),
                &alice_ctx
            )
            .unwrap(),
            0,
            "players should start with zero command-zone commander casts"
        );

        game.record_commander_cast_from_command_zone(alice_commander);
        game.record_commander_cast_from_command_zone(alice_commander);
        game.record_commander_cast_from_command_zone(bob_commander);

        assert_eq!(
            resolve_value(
                &game,
                &Value::CommanderCastCount(PlayerFilter::You),
                &alice_ctx
            )
            .unwrap(),
            2,
            "your commander-cast count should include only your command-zone casts"
        );

        let bob_ctx = ExecutionContext::new_default(source_id, bob);
        assert_eq!(
            resolve_value(
                &game,
                &Value::CommanderCastCount(PlayerFilter::You),
                &bob_ctx
            )
            .unwrap(),
            1,
            "the value should branch by controller and not leak another player's cast count"
        );
    }

    #[test]
    fn test_resolve_lands_entered_battlefield_this_turn_counts_historical_entries() {
        use crate::events::EnterBattlefieldEvent;
        use crate::filter::ObjectFilter;
        use crate::provenance::ProvNodeId;
        use crate::triggers::TriggerEvent;

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();

        let bob_land_a =
            add_battlefield_permanent(&mut game, 5901, "Bob Land A", bob, vec![CardType::Land]);
        let bob_land_b =
            add_battlefield_permanent(&mut game, 5902, "Bob Land B", bob, vec![CardType::Land]);
        let alice_land =
            add_battlefield_permanent(&mut game, 5903, "Alice Land", alice, vec![CardType::Land]);

        for land_id in [bob_land_a, bob_land_b, alice_land] {
            let event = TriggerEvent::new_with_provenance(
                EnterBattlefieldEvent::new(land_id, Zone::Hand),
                ProvNodeId::default(),
            );
            game.record_turn_history_event(&event);
        }
        game.move_object_by_effect(bob_land_a, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.iteration.iterated_player = Some(bob);

        assert_eq!(
            resolve_value(
                &game,
                &Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                &ctx
            )
            .unwrap(),
            2,
            "historical land-entry counts should include lands that have left the battlefield"
        );

        let mut current_battlefield_filter = ObjectFilter::land();
        current_battlefield_filter.zone = Some(Zone::Battlefield);
        current_battlefield_filter.entered_battlefield_this_turn = true;
        current_battlefield_filter.entered_battlefield_controller =
            Some(PlayerFilter::IteratedPlayer);
        assert_eq!(
            resolve_value(&game, &Value::Count(current_battlefield_filter), &ctx).unwrap(),
            1,
            "the older object filter shape only counts matching lands still on the battlefield"
        );
    }

    #[test]
    fn test_resolve_total_power_for_pure_tagged_filter_outside_battlefield() {
        use crate::filter::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
        use crate::snapshot::ObjectSnapshot;
        use crate::tag::TagKey;

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();

        let bear = CardBuilder::new(crate::ids::CardId::from_raw(5001), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let elf = CardBuilder::new(crate::ids::CardId::from_raw(5002), "Elf")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let bear_id = game.create_object_from_card(&bear, alice, Zone::Graveyard);
        let elf_id = game.create_object_from_card(&elf, alice, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.set_tagged_objects(
            "sacrificed_0",
            vec![
                ObjectSnapshot::from_object(game.object(bear_id).unwrap(), &game),
                ObjectSnapshot::from_object(game.object(elf_id).unwrap(), &game),
            ],
        );

        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Creature);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrificed_0"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        assert_eq!(
            resolve_value(&game, &Value::TotalPower(filter), &ctx).unwrap(),
            3,
            "pure tagged filters should evaluate against their tagged objects, even off the battlefield"
        );
    }

    #[test]
    fn current_tagged_count_excludes_departed_and_returned_new_objects() {
        use crate::snapshot::ObjectSnapshot;
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let card = CardBuilder::new(crate::ids::CardId::new(), "Chosen permanent")
            .card_types(vec![CardType::Artifact])
            .build();
        let chosen = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects(
            "chosen",
            vec![ObjectSnapshot::from_object(
                game.object(chosen).unwrap(),
                &game,
            )],
        );
        let historical = crate::filter::ObjectFilter::tagged("chosen").in_zone(Zone::Battlefield);
        let mut current = historical.clone();
        current.match_current_state = true;
        assert_eq!(
            resolve_value(&game, &Value::Count(current.clone()), &ctx).unwrap(),
            1
        );
        let exiled = game.move_object_by_effect(chosen, Zone::Exile).unwrap();
        assert_eq!(
            resolve_value(&game, &Value::Count(current.clone()), &ctx).unwrap(),
            0
        );
        assert_eq!(
            resolve_value(&game, &Value::Count(historical.clone()), &ctx).unwrap(),
            1
        );
        game.move_object_by_effect(exiled, Zone::Battlefield)
            .unwrap();
        assert_eq!(
            resolve_value(&game, &Value::Count(current), &ctx).unwrap(),
            0,
            "returning the same card creates a new object, not a surviving chosen permanent"
        );
        assert_eq!(
            resolve_value(&game, &Value::Count(historical), &ctx).unwrap(),
            1
        );
    }

    #[test]
    fn chosen_object_power_difference_uses_only_the_tagged_set() {
        use crate::filter::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
        use crate::snapshot::ObjectSnapshot;
        use crate::tag::TagKey;

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();
        let small = add_custom_creature(&mut game, 5005, "Small Choice", alice, 1, 2, 2);
        let large = add_custom_creature(&mut game, 5006, "Large Choice", alice, 1, 5, 5);
        let _unchosen = add_custom_creature(&mut game, 5007, "Unchosen Creature", alice, 1, 11, 11);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.set_tagged_objects(
            "__chosen_objects__",
            vec![
                ObjectSnapshot::from_object(game.object(small).unwrap(), &game),
                ObjectSnapshot::from_object(game.object(large).unwrap(), &game),
            ],
        );

        let mut filter = ObjectFilter::creature();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("__chosen_objects__"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let difference = Value::absolute_difference(
            Value::GreatestPower(filter.clone()),
            Value::LeastPower(filter),
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::Difference);

        assert_eq!(
            resolve_value(&game, &difference, &ctx).unwrap(),
            3,
            "the aggregate must ignore creatures outside the exact chosen set"
        );
    }

    #[test]
    fn test_resolve_count_for_pure_tagged_filter_outside_battlefield() {
        use crate::filter::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
        use crate::snapshot::ObjectSnapshot;
        use crate::tag::TagKey;

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();

        let bear = CardBuilder::new(crate::ids::CardId::from_raw(5003), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let elf = CardBuilder::new(crate::ids::CardId::from_raw(5004), "Elf")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();

        let bear_id = game.create_object_from_card(&bear, alice, Zone::Graveyard);
        let elf_id = game.create_object_from_card(&elf, alice, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.set_tagged_objects(
            "sacrificed_0",
            vec![
                ObjectSnapshot::from_object(game.object(bear_id).unwrap(), &game),
                ObjectSnapshot::from_object(game.object(elf_id).unwrap(), &game),
            ],
        );

        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Creature);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrificed_0"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        assert_eq!(
            resolve_value(&game, &Value::Count(filter), &ctx).unwrap(),
            2,
            "pure tagged filters should count their tagged objects, even off the battlefield"
        );
    }

    #[test]
    fn tagged_count_uses_current_zone_and_preserves_pre_move_characteristics() {
        use crate::filter::TaggedOpbjectRelation;
        use crate::snapshot::ObjectSnapshot;

        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = game.new_object_id();
        let card = CardBuilder::new(crate::ids::CardId::from_raw(5010), "Card Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let token = CardBuilder::new(crate::ids::CardId::from_raw(5011), "Token Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .token()
            .build();
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let token_id = game.create_object_from_card(&token, alice, Zone::Battlefield);
        let snapshots = [card_id, token_id]
            .map(|id| ObjectSnapshot::from_object(game.object(id).unwrap(), &game))
            .to_vec();

        for id in [card_id, token_id] {
            game.move_object_by_effect(id, Zone::Exile)
                .expect("tagged object should move to exile");
        }

        let mut ctx = ExecutionContext::new_default(source_id, alice);
        ctx.set_tagged_objects("exiled_this_way", snapshots);
        let filter = ObjectFilter::creature()
            .nontoken()
            .in_zone(Zone::Exile)
            .match_tagged(
                crate::tag::TagKey::from("exiled_this_way"),
                TaggedOpbjectRelation::IsTaggedObject,
            );

        assert_eq!(
            resolve_value(&game, &Value::Count(filter), &ctx).unwrap(),
            1,
            "the count should verify the current exile zone while retaining LKI token status"
        );
    }

    #[test]
    fn test_resolve_count_for_attacking_iterated_player_or_their_planeswalkers() {
        use crate::card::{CardBuilder, PowerToughness};
        use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
        use crate::ids::CardId;
        use crate::target::ObjectFilter;
        use crate::types::CardType;
        use crate::zone::Zone;

        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let charlie = game.players[2].id;

        let make_creature = |game: &mut GameState, name: &str, controller: PlayerId| {
            let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .build();
            game.create_object_from_card(&card, controller, Zone::Battlefield)
        };

        let attacker_a = make_creature(&mut game, "A", alice);
        let attacker_b = make_creature(&mut game, "B", alice);
        let attacker_c = make_creature(&mut game, "C", alice);

        let planeswalker_card = CardBuilder::new(
            CardId::from_raw(game.new_object_id().0 as u32),
            "Bob Walker",
        )
        .card_types(vec![CardType::Planeswalker])
        .build();
        let bob_planeswalker =
            game.create_object_from_card(&planeswalker_card, bob, Zone::Battlefield);

        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: attacker_a,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: attacker_b,
                    target: AttackTarget::Planeswalker(bob_planeswalker),
                },
                AttackerInfo {
                    creature: attacker_c,
                    target: AttackTarget::Player(charlie),
                },
            ],
            ..Default::default()
        });

        let mut filter = ObjectFilter::creature();
        filter.attacking = true;
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::IteratedPlayer);
        let value = Value::Count(filter);

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.iteration.iterated_player = Some(bob);
        assert_eq!(
            resolve_value(&game, &value, &ctx).unwrap(),
            2,
            "should count attackers attacking Bob or his planeswalker"
        );

        ctx.iteration.iterated_player = Some(charlie);
        assert_eq!(
            resolve_value(&game, &value, &ctx).unwrap(),
            1,
            "should count only attackers attacking Charlie"
        );
    }

    #[test]
    fn constrained_player_filters_do_not_accept_unqualified_context_players() {
        let mut game = new_test_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), bob);
        for targets in [
            vec![bob],
            vec![bob, alice],
            vec![alice, bob],
            vec![PlayerId::from_index(99), alice],
        ] {
            ctx.targets = targets.into_iter().map(ResolvedTarget::Player).collect();
            assert_eq!(
                resolve_player_filter(&game, &PlayerFilter::Opponent, &ctx).unwrap(),
                alice
            );
        }
        ctx.targets = vec![ResolvedTarget::Player(bob)];
        assert_eq!(
            resolve_player_filter(&game, &PlayerFilter::Any, &ctx).unwrap(),
            bob,
            "unconstrained affected-player bindings remain available"
        );
        for target in [alice, bob] {
            ctx.targets = vec![ResolvedTarget::Player(target)];
            assert!(
                matches!(
                    resolve_player_filter(&game, &PlayerFilter::Teammate, &ctx),
                    Err(ExecutionError::UnresolvableValue(_))
                ),
                "no teammate exists in this game"
            );
        }
    }

    #[test]
    fn constrained_opponent_context_retains_multiplayer_choice_and_valid_targets() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), bob);
        ctx.targets = vec![ResolvedTarget::Player(bob)];
        assert!(
            matches!(resolve_player_filter(&game, &PlayerFilter::Opponent, &ctx),
            Err(ExecutionError::UnresolvableValue(ref detail)) if detail == AN_OPPONENT_CHOICE_REQUIRED)
        );
        ctx.set_tagged_players(AN_OPPONENT_CHOICE_TAG, vec![carol]);
        assert_eq!(
            resolve_player_filter(&game, &PlayerFilter::Opponent, &ctx).unwrap(),
            carol
        );
        ctx.targets = vec![ResolvedTarget::Player(bob), ResolvedTarget::Player(alice)];
        assert_eq!(
            resolve_player_filter(&game, &PlayerFilter::Opponent, &ctx).unwrap(),
            alice,
            "a valid explicit context target still takes precedence"
        );
    }

    #[test]
    fn test_resolve_player_filter_you() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id);

        let filter = PlayerFilter::You;
        assert_eq!(
            resolve_player_filter(&game, &filter, &ctx).unwrap(),
            player_id
        );
    }

    #[test]
    fn test_find_target_object_found() {
        let object_id = ObjectId(42);
        let targets = vec![ResolvedTarget::Object(object_id)];
        assert_eq!(find_target_object(&targets).unwrap(), object_id);
    }

    #[test]
    fn test_find_target_object_not_found() {
        let targets = vec![ResolvedTarget::Player(PlayerId(1))];
        assert!(find_target_object(&targets).is_err());
    }

    #[test]
    fn test_resolve_single_object_from_spec_source() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, player_id);

        let resolved = resolve_single_object_from_spec(&game, &ChooseSpec::Source, &ctx).unwrap();
        assert_eq!(resolved, source_id);
    }

    #[test]
    fn counters_on_source_uses_lki_when_source_left_expected_zone() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source = add_battlefield_permanent(
            &mut game,
            5004,
            "Remembered Counterbear",
            alice,
            vec![CardType::Creature],
        );
        game.object_mut(source)
            .expect("source should exist")
            .counters
            .insert(crate::object::CounterType::PlusOnePlusOne, 3);
        let source_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).expect("source should still exist"),
            &game,
        );

        game.remove_object(source);

        let ctx =
            ExecutionContext::new_default(source, alice).with_source_snapshot(source_snapshot);
        assert_eq!(
            resolve_value(
                &game,
                &Value::CountersOnSource(crate::object::CounterType::PlusOnePlusOne),
                &ctx
            )
            .expect("source counter count should resolve from LKI"),
            3,
            "608.2h requires source-referential effects to use LKI if the source is gone"
        );
        assert_eq!(
            resolve_value(
                &game,
                &Value::CountersOn(Box::new(ChooseSpec::Source), None),
                &ctx
            )
            .expect("generic source counter count should resolve from LKI"),
            3,
            "generic source-counter values should use the same source LKI"
        );
    }

    #[test]
    fn test_resolve_tagged_object_helper_follows_zone_changes_to_the_current_object() {
        let mut game = new_test_game();
        let alice = game.players[0].id;

        let creature = CardBuilder::new(crate::ids::CardId::from_raw(5005), "Test Galleon")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![crate::types::Subtype::Vehicle])
            .power_toughness(PowerToughness::fixed(2, 10))
            .build();
        let battlefield_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let snapshot = ObjectSnapshot::from_object(
            game.object(battlefield_id)
                .expect("battlefield object should exist"),
            &game,
        );

        let exile_id = game
            .move_object_by_effect(battlefield_id, Zone::Exile)
            .expect("object should move to exile");
        let returned_id = game
            .move_object_by_effect(exile_id, Zone::Battlefield)
            .expect("object should return to the battlefield");

        assert_eq!(
            resolve_tagged_object_id(
                &game,
                &ExecutionContext::new_default(returned_id, alice),
                &snapshot
            ),
            Some(returned_id),
            "tagged object helper should follow the current object after a round trip"
        );
    }

    #[test]
    fn test_find_target_player_found() {
        let player_id = PlayerId(1);
        let targets = vec![ResolvedTarget::Player(player_id)];
        assert_eq!(find_target_player(&targets).unwrap(), player_id);
    }

    #[test]
    fn test_normalize_object_selection_filters_invalid_and_dedups() {
        let candidates = vec![ObjectId(1), ObjectId(2), ObjectId(3)];
        let chosen = vec![ObjectId(2), ObjectId(99), ObjectId(2), ObjectId(3)];

        let selected = normalize_object_selection(chosen, &candidates, 2);
        assert_eq!(selected, vec![ObjectId(2), ObjectId(3)]);
    }

    #[test]
    fn test_normalize_object_selection_fills_missing_required() {
        let candidates = vec![ObjectId(10), ObjectId(11), ObjectId(12)];
        let chosen = vec![ObjectId(11)];

        let selected = normalize_object_selection(chosen, &candidates, 3);
        assert_eq!(selected, vec![ObjectId(11), ObjectId(10), ObjectId(12)]);
    }

    #[test]
    fn test_resolve_objects_from_spec_filters_out_other_selected_object_targets() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();

        let creature_id = add_battlefield_permanent(
            &mut game,
            100,
            "Target Creature",
            bob,
            vec![CardType::Creature],
        );
        let land_id =
            add_battlefield_permanent(&mut game, 101, "Target Land", bob, vec![CardType::Land]);

        let ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![
                ResolvedTarget::Object(creature_id),
                ResolvedTarget::Object(land_id),
            ])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::creature()),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::Object(
                        crate::filter::ObjectFilter::land(),
                    )),
                    range: 1..2,
                },
            ]);

        let resolved =
            resolve_objects_from_spec(&game, &ChooseSpec::target(ChooseSpec::creature()), &ctx)
                .expect("creature target should resolve");

        assert_eq!(
            resolved,
            vec![creature_id],
            "resolving a specific target clause should not include unrelated object targets from the same spell"
        );
    }

    #[test]
    fn test_resolve_players_from_spec_filters_out_other_selected_player_targets() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();

        let ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![
                ResolvedTarget::Player(alice),
                ResolvedTarget::Player(bob),
            ])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Specific(alice))),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Specific(bob))),
                    range: 1..2,
                },
            ]);

        let resolved = resolve_players_from_spec(
            &game,
            &ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Specific(bob))),
            &ctx,
        )
        .expect("player target should resolve");

        assert_eq!(
            resolved,
            vec![bob],
            "resolving one player-target clause should not include unrelated player targets from the same spell"
        );
    }

    #[test]
    fn object_or_player_target_resolves_only_the_selected_target_kind() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();
        let battle_id =
            add_battlefield_permanent(&mut game, 102, "Target Battle", bob, vec![CardType::Battle]);
        let mut battle_filter = crate::filter::ObjectFilter::default();
        battle_filter.card_types = vec![CardType::Battle];
        let spec = ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
            battle_filter,
            PlayerFilter::Opponent,
        ));

        let object_ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![ResolvedTarget::Object(battle_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: spec.clone(),
                range: 0..1,
            }]);
        assert_eq!(
            resolve_objects_from_spec(&game, &spec, &object_ctx).unwrap(),
            vec![battle_id]
        );
        assert!(resolve_players_from_spec(&game, &spec, &object_ctx).is_err());

        let player_ctx = ExecutionContext::new_default(source_id, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: spec.clone(),
                range: 0..1,
            }]);
        assert!(resolve_objects_from_spec(&game, &spec, &player_ctx).is_err());
        assert_eq!(
            resolve_players_from_spec(&game, &spec, &player_ctx).unwrap(),
            vec![bob]
        );
        assert_eq!(
            resolve_player_from_spec(&game, &spec, &player_ctx).unwrap(),
            bob
        );
    }

    #[test]
    fn test_resolve_player_filter_to_list_includes_all_targeted_players() {
        let game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = ObjectId(999);
        let filter_ctx = FilterContext::default();

        let ctx = ExecutionContext::new_default(source_id, alice).with_targets(vec![
            ResolvedTarget::Player(alice),
            ResolvedTarget::Player(bob),
        ]);

        let resolved =
            resolve_player_filter_to_list(&game, &PlayerFilter::target_player(), &filter_ctx, &ctx)
                .expect("target-player list should resolve");

        assert_eq!(
            resolved,
            vec![alice, bob],
            "target-player lists should include every targeted player, preserving order"
        );
    }

    #[test]
    fn resolve_life_total_difference_uses_all_targeted_players() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.player_mut(alice).expect("alice exists").life = 7;
        game.player_mut(bob).expect("bob exists").life = 24;
        let ctx = ExecutionContext::new_default(ObjectId(999), alice).with_targets(vec![
            ResolvedTarget::Player(alice),
            ResolvedTarget::Player(bob),
        ]);

        let value = Value::LifeTotalDifference(PlayerFilter::target_player());
        assert_eq!(
            resolve_value(&game, &value, &ctx).expect("life difference should resolve"),
            17
        );
    }

    #[test]
    fn test_apply_to_selected_objects_count_policy() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_1 = game.new_object_id();
        let target_2 = game.new_object_id();
        let spec = ChooseSpec::target(ChooseSpec::creature())
            .with_count(crate::effect::ChoiceCount::any_number());
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![
                ResolvedTarget::Object(target_1),
                ResolvedTarget::Object(target_2),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: spec.clone(),
                range: 0..2,
            }]);
        let mut seen = Vec::new();
        let result = apply_to_selected_objects(
            &mut game,
            &mut ctx,
            &spec,
            ObjectApplyResultPolicy::CountApplied,
            |_game, _ctx, object_id| {
                seen.push(object_id);
                Ok(object_id == target_1)
            },
        )
        .unwrap();

        assert_eq!(result.selected_count, 2);
        assert_eq!(result.applied_count, 1);
        assert_eq!(result.outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(seen, vec![target_1, target_2]);
    }

    #[test]
    fn test_apply_to_selected_objects_single_target_policy_resolves_when_selected() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(target_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::creature()),
                range: 0..1,
            }]);

        let spec = ChooseSpec::target(ChooseSpec::creature());
        let result = apply_to_selected_objects(
            &mut game,
            &mut ctx,
            &spec,
            ObjectApplyResultPolicy::SingleTargetResolvedOrInvalid,
            |_game, _ctx, _object_id| Ok(false),
        )
        .unwrap();

        assert_eq!(result.selected_count, 1);
        assert_eq!(result.applied_count, 0);
        assert_eq!(
            result.outcome.status,
            crate::effect::OutcomeStatus::Succeeded
        );
    }

    #[test]
    fn test_apply_to_selected_objects_prompts_for_non_targeted_with_count_object_specs() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let source_id = add_battlefield_permanent(
            &mut game,
            200,
            "Simic Growth Chamber",
            alice,
            vec![CardType::Land],
        );
        let other_land =
            add_battlefield_permanent(&mut game, 201, "Forest", alice, vec![CardType::Land]);
        let mut dm = SelectIdsDecisionMaker {
            chosen: vec![source_id],
        };
        let mut ctx = ExecutionContext::new_default(source_id, alice).with_decision_maker(&mut dm);
        let spec = ChooseSpec::Object(crate::filter::ObjectFilter::land().you_control())
            .with_count(ChoiceCount::exactly(1));
        let mut seen = Vec::new();

        let result = apply_to_selected_objects(
            &mut game,
            &mut ctx,
            &spec,
            ObjectApplyResultPolicy::CountApplied,
            |_game, _ctx, object_id| {
                seen.push(object_id);
                Ok(true)
            },
        )
        .expect("selection should resolve");

        assert_eq!(result.selected_count, 1);
        assert_eq!(result.applied_count, 1);
        assert_eq!(seen, vec![source_id]);
        assert_ne!(seen, vec![other_land]);
    }

    #[test]
    fn test_apply_single_target_object_from_context_resolves_on_none() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(target_id)]);

        let outcome = apply_single_target_object_from_context(
            &mut game,
            &mut ctx,
            |_game, _ctx, object_id| {
                assert_eq!(object_id, target_id);
                Ok(None)
            },
        )
        .unwrap();

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_apply_single_target_object_from_context_propagates_custom_result() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let target_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(target_id)]);

        let outcome = apply_single_target_object_from_context(
            &mut game,
            &mut ctx,
            |_game, _ctx, _object_id| Ok(Some(crate::effect::OutcomeStatus::Prevented)),
        )
        .unwrap();

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Prevented);
    }

    #[test]
    fn test_apply_single_target_object_from_context_target_invalid_without_object_target() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;
        let source_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Player(PlayerId(1))]);

        let outcome = apply_single_target_object_from_context(
            &mut game,
            &mut ctx,
            |_game, _ctx, _object_id| Ok(None),
        )
        .unwrap();

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_resolve_player_filter_most_cards_in_hand_requires_unique_leader() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, alice);

        add_hand_card(&mut game, 300, "Mountain", alice);
        add_hand_card(&mut game, 301, "Forest", bob);
        add_hand_card(&mut game, 302, "Island", bob);

        assert_eq!(
            resolve_player_filter(&game, &PlayerFilter::MostCardsInHand, &ctx)
                .expect("bob should be the unique hand-size leader"),
            bob
        );

        add_hand_card(&mut game, 303, "Plains", alice);
        let err = resolve_player_filter(&game, &PlayerFilter::MostCardsInHand, &ctx)
            .expect_err("ties should not resolve MostCardsInHand");
        assert!(
            matches!(err, ExecutionError::UnresolvableValue(ref message) if message.contains("unique")),
            "expected unique-leader resolution error, got {err:?}"
        );
    }

    #[test]
    fn count_players_with_minimum_hand_size_resolves_the_qualified_player_set() {
        let mut game = new_test_game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source_id = game.new_object_id();
        let ctx = ExecutionContext::new_default(source_id, alice);
        let value = Value::CountPlayersWithCardsInHandAtLeast(PlayerFilter::Opponent, 4);

        for (index, name) in ["Mountain", "Forest", "Island"].into_iter().enumerate() {
            add_hand_card(&mut game, 400 + index as u32, name, bob);
        }
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 0);

        add_hand_card(&mut game, 403, "Plains", bob);
        assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 1);

        for (index, name) in ["Swamp", "Wastes", "Mine", "Tower"].into_iter().enumerate() {
            add_hand_card(&mut game, 500 + index as u32, name, alice);
        }
        assert_eq!(
            resolve_value(&game, &value, &ctx).unwrap(),
            1,
            "the controller is not part of the opponent set"
        );
    }

    #[test]
    fn targeted_discard_history_count_uses_only_the_selected_opponent() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let cara = game.players[2].id;
        let source = game.new_object_id();

        for player in [bob, bob, cara, cara, cara] {
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::CardDiscardedEvent::new(player, game.new_object_id()),
                crate::provenance::ProvNodeId::default(),
            );
            game.turn_store
                .turn_history
                .record_event(&event, None, None);
        }

        let value = Value::CardsDiscardedThisTurn(PlayerFilter::target_opponent());
        let bob_ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let cara_ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(cara)]);

        assert_eq!(resolve_value(&game, &value, &bob_ctx).unwrap(), 2);
        assert_eq!(resolve_value(&game, &value, &cara_ctx).unwrap(), 3);
    }
}

#[cfg(test)]
mod additional_callback_owner_contract_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;
    struct Answers {
        pending: bool,
        pause: bool,
        pause_selection: bool,
        selections: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_objects(
            &mut self,
            _: &GameState,
            context: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.selections += 1;
            if self.pending || self.pause_selection {
                self.pending = true;
                Vec::new()
            } else {
                context
                    .candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id)
                    .take(context.min.max(1))
                    .collect()
            }
        }
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.pending = self.pause;
            !self.pause
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn run_callbacks(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        spec: &ChooseSpec,
        mode: u8,
        calls: &mut Vec<ObjectId>,
    ) -> Result<ObjectApplyResult, ExecutionError> {
        apply_to_selected_objects(
            game,
            ctx,
            spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object| {
                calls.push(object);
                ctx.set_tagged_objects(
                    "callback",
                    vec![crate::snapshot::ObjectSnapshot::from_object(
                        game.object(object).unwrap(),
                        game,
                    )],
                );
                let outcome = crate::effects::execute_effect(game, &Effect::gain_life(2), ctx)?;
                for event in outcome.events {
                    game.queue_trigger_event(event.provenance(), event);
                }
                if mode == 1 {
                    crate::effects::execute_effect(game, &Effect::lose_life(Value::X), ctx)?;
                }
                let outcome = crate::effects::execute_effect(
                    game,
                    &Effect::may(vec![Effect::gain_life(4)]),
                    ctx,
                )?;
                for event in outcome.events {
                    game.queue_trigger_event(event.provenance(), event);
                }
                Ok(true)
            },
        )
    }
    fn check_callbacks(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = crate::card::CardBuilder::new(CardId::new(), "Callback object")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        for _ in 0..2 {
            game.create_object_from_card(&creature, alice, Zone::Battlefield);
        }
        let card = crate::card::CardBuilder::new(CardId::new(), "Callback source")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let sentinel =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.take_pending_trigger_events();
        let spec = if mode >= 3 {
            ChooseSpec::Object(ObjectFilter::creature().you_control())
        } else {
            ChooseSpec::all(ObjectFilter::creature().you_control())
        };
        let mut dm = Answers {
            pending: mode == 4,
            pause: mode == 2,
            pause_selection: mode == 3,
            selections: 0,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects("callback", vec![sentinel.clone()]);
        let mut calls = Vec::new();
        let result = run_callbacks(&mut game, &mut ctx, &spec, mode, &mut calls);
        if mode == 1 {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
        } else {
            let receipt = result.unwrap();
            if mode >= 2 {
                assert!(ctx.decision_maker.awaiting_choice());
                assert_eq!(
                    calls.len(),
                    if mode == 2 { 1 } else { 0 },
                    "a suspended callback/selection cannot execute later callbacks"
                );
                assert_eq!(receipt.applied_count, 0);
                assert_eq!(receipt.outcome.count_or_zero(), 0);
            } else {
                assert_eq!(receipt.selected_count, 2);
                assert_eq!(receipt.applied_count, 2);
                assert_eq!(receipt.outcome.count_or_zero(), 2);
            }
        }
        if mode != 0 {
            assert_eq!(
                game.player(alice).unwrap().life,
                20,
                "callback/selection failures restore the entire original helper operation"
            );
            assert_eq!(
                ctx.get_tagged_all("callback").unwrap()[0].object_id,
                sentinel.object_id
            );
            assert!(game.take_pending_trigger_events().is_empty());
        } else {
            assert_eq!(game.player(alice).unwrap().life, 32);
            assert_eq!(
                game.take_pending_trigger_events()
                    .iter()
                    .filter_map(|e| e.downcast::<crate::events::LifeGainEvent>())
                    .map(|e| e.amount)
                    .collect::<Vec<_>>(),
                vec![2, 4, 2, 4]
            );
        }
        assert_eq!(ctx.source, source);
        assert_eq!(ctx.controller, alice);
        drop(ctx);
        if mode == 4 {
            assert_eq!(
                dm.selections, 0,
                "an already pending parent cannot be asked another selection"
            );
        }
        if mode == 3 {
            assert_eq!(dm.selections, 1);
        }
        if mode == 2 {
            let mut dm = Answers {
                pending: false,
                pause: false,
                pause_selection: false,
                selections: 0,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let mut calls = Vec::new();
            let receipt = run_callbacks(&mut game, &mut ctx, &spec, 0, &mut calls).unwrap();
            assert_eq!(receipt.applied_count, 2);
            assert!(!ctx.decision_maker.awaiting_choice());
            assert_eq!(game.player(alice).unwrap().life, 32);
            assert_eq!(calls.len(), 2);
            assert_eq!(
                game.take_pending_trigger_events()
                    .iter()
                    .filter_map(|e| e.downcast::<crate::events::LifeGainEvent>())
                    .map(|e| e.amount)
                    .collect::<Vec<_>>(),
                vec![2, 4, 2, 4]
            );
        }
    }
    #[test]
    fn callback_success_retains_original_count_and_observations() {
        check_callbacks(0);
    }
    #[test]
    fn callback_error_restores_game_context_and_queued_prefix() {
        check_callbacks(1);
    }
    #[test]
    fn callback_pending_stops_later_objects_and_replays_once() {
        check_callbacks(2);
    }
    #[test]
    fn callback_pending_selection_does_not_execute_objects() {
        check_callbacks(3);
    }
    #[test]
    fn callback_already_pending_parent_does_not_ask_again() {
        check_callbacks(4);
    }
}

#[cfg(test)]
mod replacement_object_selection_contract_tests {
    use super::*;
    use crate::target::ObjectFilter;
    fn setup() -> (GameState, ObjectId, ObjectId, ObjectId, PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let artifact =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Selection artifact")
                .card_types(vec![crate::types::CardType::Artifact])
                .build();
        let creature =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Selection creature")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        let first = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        let selected = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        (game, first, second, selected, alice)
    }
    #[test]
    fn non_target_specific_filter_intersects_supplied_objects() {
        let (game, first, second, selected, alice) = setup();
        let ctx = ExecutionContext::new_default(first, alice).with_targets(
            vec![first, second, selected]
                .into_iter()
                .map(ResolvedTarget::Object)
                .collect(),
        );
        assert_eq!(
            resolve_objects_from_spec(
                &game,
                &ChooseSpec::Object(ObjectFilter::specific(selected)),
                &ctx
            )
            .unwrap(),
            vec![selected]
        );
    }
    #[test]
    fn non_target_type_filter_intersects_supplied_objects() {
        let (game, first, second, selected, alice) = setup();
        let ctx = ExecutionContext::new_default(first, alice).with_targets(
            vec![first, second, selected]
                .into_iter()
                .map(ResolvedTarget::Object)
                .collect(),
        );
        assert_eq!(
            resolve_objects_from_spec(&game, &ChooseSpec::Object(ObjectFilter::creature()), &ctx)
                .unwrap(),
            vec![selected]
        );
    }
    #[test]
    fn regeneration_specific_filter_does_not_shield_other_supplied_objects() {
        let (mut game, first, second, selected, alice) = setup();
        let mut ctx = ExecutionContext::new_default(first, alice).with_targets(
            vec![first, second, selected]
                .into_iter()
                .map(ResolvedTarget::Object)
                .collect(),
        );
        crate::effects::execute_effect(
            &mut game,
            &crate::effect::Effect::regenerate(
                ChooseSpec::Object(ObjectFilter::specific(selected)),
                crate::effect::Until::EndOfTurn,
            ),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(selected),
            1
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(first),
            0
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(second),
            0
        );
    }
}

#[cfg(test)]
#[path = "helpers/tagged_lki_identity_tests.rs"]
mod tagged_lki_identity_tests;

#[cfg(test)]
mod aura_source_incarnation_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::events::{ZoneChangeEvent, cause::EventCause};
    use crate::ids::{CardId, PlayerId};
    use crate::object::AttachmentTarget;
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::TriggerEvent;
    use crate::turn_history::TurnEventRecord;
    use crate::types::{CardType, Subtype};
    fn setup() -> (GameState, ObjectId, ObjectId, ObjectId, ObjectSnapshot) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let owner = PlayerId(0);
        let host = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Attached host")
                .card_types(vec![CardType::Creature])
                .build(),
            owner,
            Zone::Battlefield,
        );
        let aura = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Source Aura")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .build(),
            owner,
            Zone::Battlefield,
        );
        game.object_mut(aura).unwrap().attached_to = Some(AttachmentTarget::Object(host));
        let snapshot = ObjectSnapshot::from_object(game.object(aura).unwrap(), &game);
        let graveyard = game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        // Unit fixture supplies a canonical completed SBA receipt. The full
        // Ghoulish integration scenario separately uses real native SBAs.
        game.turn_store.turn_history.event_records.clear();
        game.turn_store.turn_history.staged_event_records.clear();
        (game, host, aura, graveyard, snapshot)
    }
    fn record(
        game: &mut GameState,
        source: ObjectId,
        graveyard: ObjectId,
        snapshot: ObjectSnapshot,
        cause: EventCause,
    ) {
        game.turn_store
            .turn_history
            .event_records
            .push(TurnEventRecord {
                event: TriggerEvent::new_with_provenance(
                    ZoneChangeEvent::with_results(
                        source,
                        vec![graveyard],
                        Zone::Battlefield,
                        Zone::Graveyard,
                        cause,
                        Some(snapshot),
                    ),
                    Default::default(),
                ),
                object_snapshot: None,
                source_snapshot: None,
            });
    }
    fn trigger(host: ObjectId) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            ZoneChangeEvent::with_cause(
                host,
                Zone::Battlefield,
                Zone::Graveyard,
                EventCause::from_game_rule(),
                None,
            ),
            Default::default(),
        )
    }
    #[test]
    fn aura_exception_returns_the_recorded_sba_arrival_never_an_exile_return() {
        let (mut game, host, source, graveyard, snapshot) = setup();
        record(
            &mut game,
            source,
            graveyard,
            snapshot.clone(),
            EventCause::from_sba(),
        );
        let ctx = ExecutionContext::new_default(source, PlayerId(0))
            .with_source_snapshot(snapshot)
            .with_triggering_event(trigger(host));
        assert_eq!(resolve_source_object_id(&game, &ctx), Some(graveyard));
        let exiled = game.move_object_by_effect(graveyard, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
        assert_ne!(returned, graveyard);
        assert_eq!(resolve_source_object_id(&game, &ctx), None);
    }
    #[test]
    fn unrelated_move_or_unrelated_attachment_does_not_authorize_source_following() {
        for sba in [false, true] {
            let (mut game, host, source, graveyard, mut snapshot) = setup();
            if sba {
                snapshot.attached_to = Some(AttachmentTarget::Object(ObjectId::from_raw(90909)));
            }
            record(
                &mut game,
                source,
                graveyard,
                snapshot.clone(),
                if sba {
                    EventCause::from_sba()
                } else {
                    EventCause::from_game_rule()
                },
            );
            let ctx = ExecutionContext::new_default(source, PlayerId(0))
                .with_source_snapshot(snapshot)
                .with_triggering_event(trigger(host));
            assert_eq!(resolve_source_object_id(&game, &ctx), None);
        }
    }
    #[test]
    fn batched_source_departure_retains_the_existing_exact_event_exception() {
        let (game, host, source, graveyard, snapshot) = setup();
        let mut event = ZoneChangeEvent::with_results(
            source,
            vec![graveyard],
            Zone::Battlefield,
            Zone::Graveyard,
            EventCause::from_game_rule(),
            Some(snapshot.clone()),
        );
        event.objects.push(host);
        let ctx = ExecutionContext::new_default(source, PlayerId(0))
            .with_source_snapshot(snapshot)
            .with_triggering_event(TriggerEvent::new_with_provenance(event, Default::default()));
        assert_eq!(resolve_source_object_id(&game, &ctx), Some(graveyard));
    }
    #[test]
    fn copied_aura_can_lose_its_aura_type_in_graveyard_without_losing_the_proven_arrival() {
        let (mut game, host, source, graveyard, snapshot) = setup();
        record(
            &mut game,
            source,
            graveyard,
            snapshot.clone(),
            EventCause::from_sba(),
        );
        game.object_mut(graveyard).unwrap().subtypes.clear();
        let ctx = ExecutionContext::new_default(source, PlayerId(0))
            .with_source_snapshot(snapshot)
            .with_triggering_event(trigger(host));
        assert_eq!(resolve_source_object_id(&game, &ctx), Some(graveyard));
    }
}
#[cfg(test)]
mod shared_player_list_team_tests {
    use super::*;
    #[test]
    fn team_lists_keep_not_you_targets_iteration_and_range_distinct() {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Teammate".into(),
                "Bob".into(),
                "Charlie".into(),
            ],
            20,
        );
        let [alice, teammate, bob, charlie] = std::array::from_fn(|i| game.players[i].id);
        game.set_teams(vec![vec![alice, teammate], vec![bob, charlie]])
            .unwrap();
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(999), alice)
            .with_targets(vec![ResolvedTarget::Player(teammate)]);
        ctx.iteration.iterated_player = Some(charlie);
        let filter_ctx = ctx.filter_context(&game);
        for (filter, expected) in [
            (PlayerFilter::Opponent, vec![bob, charlie]),
            (PlayerFilter::Teammate, vec![teammate]),
            (PlayerFilter::NotYou, vec![teammate, bob, charlie]),
            (PlayerFilter::target_player(), vec![teammate]),
            (PlayerFilter::IteratedPlayer, vec![charlie]),
            (
                PlayerFilter::Excluding {
                    base: Box::new(PlayerFilter::Any),
                    excluded: Box::new(PlayerFilter::Opponent),
                },
                vec![alice, teammate],
            ),
        ] {
            assert_eq!(
                resolve_player_filter_to_list(&game, &filter, &filter_ctx, &ctx).unwrap(),
                expected
            );
        }
        let mut range = filter_ctx;
        range.players_in_range = Some(vec![alice, teammate, bob]);
        assert_eq!(
            resolve_player_filter_to_list(&game, &PlayerFilter::Opponent, &range, &ctx).unwrap(),
            vec![bob]
        );
        assert_eq!(
            resolve_player_filter_to_list(&game, &PlayerFilter::Teammate, &range, &ctx).unwrap(),
            vec![teammate]
        );
    }
}

#[cfg(test)]
mod discarded_incarnation_receipt_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::EffectExecutor;
    use crate::ids::CardId;
    #[test]
    fn self_and_group_discard_references_find_only_the_recorded_arrival() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Discarded source").build();
        let origin = game.create_object_from_card(&card, a, Zone::Hand);
        let before = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(origin).unwrap(),
            &game,
        );
        let grave = game.move_object_by_effect(origin, Zone::Graveyard).unwrap();
        let event = crate::events::other::CardDiscardedEvent::new(a, origin)
            .with_snapshot(before.clone())
            .with_batch(vec![origin], vec![before.clone()], 0)
            .with_destinations(vec![crate::events::other::DiscardedCardDestination {
                card: origin,
                object: Some(grave),
                zone: Zone::Graveyard,
            }]);
        let mut ctx = ExecutionContext::new_default(origin, a).with_source_snapshot(before.clone());
        ctx.triggering_event = Some(crate::triggers::TriggerEvent::new_with_provenance(
            event,
            Default::default(),
        ));
        ctx.set_tagged_objects(ironsmith_core::ZONE_CHANGE_GROUP_TAG, vec![before]);
        assert_eq!(resolve_source_object_id(&game, &ctx), Some(grave));
        crate::effects::TagTriggeringObjectEffect::new("discarded")
            .execute(&mut game, &mut ctx)
            .unwrap();
        let snapshot = ctx.get_tagged_all("discarded").unwrap()[0].clone();
        assert_eq!(snapshot.object_id, grave);
        assert_eq!(
            resolve_tagged_object_id(&game, &ctx, &snapshot),
            Some(grave)
        );
        let exile = game.move_object_by_effect(grave, Zone::Exile).unwrap();
        let later = game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        assert_ne!(grave, later);
        assert_eq!(
            resolve_source_object_id(&game, &ctx),
            None,
            "registration cannot pin a later graveyard incarnation"
        );
        assert_eq!(
            resolve_tagged_object_id(&game, &ctx, &snapshot),
            None,
            "group return cannot chase the physical card"
        );
        // Re-running the prelude preserves historical data rather than naming
        // the later card; source/body movement still finds no eligible arrival.
        crate::effects::TagTriggeringObjectEffect::new("discarded")
            .execute(&mut game, &mut ctx)
            .unwrap();
        let retained = &ctx.get_tagged_all("discarded").unwrap()[0];
        assert_eq!(retained.object_id, origin);
        assert_eq!(resolve_tagged_object_id(&game, &ctx, retained), None);
    }
}

#[cfg(test)]
mod reconciled_reveal_receipt_tests {
    use super::*;
    use crate::effect::{EffectId, ExecutionFact};

    // Authored only: native reveal action facts are the exact producer's evidence.
    #[test]
    fn chosen_color_reads_native_reveal_action_without_borrowing_additions() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = ObjectId::from_raw(9001);
        let color = EffectId(20);
        let reveal = EffectId(21);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        let mut query = PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
            .with_action(ironsmith_core::PriorEffectAction::Revealed)
            .with_filter(crate::target::ObjectFilter::default().of_chosen_color());
        query.color_choice = Some(ironsmith_core::ColorChoiceReference::Effect(color));
        ctx.store_outcome(color, EffectOutcome::count(1)
            .with_execution_fact(ExecutionFact::ChosenColor(crate::color::Color::Blue)));
        let mut card = ObjectSnapshot::for_testing(ObjectId::from_raw(9002), player, "Original blue reveal");
        card.colors = crate::color::ColorSet::BLUE;
        let receipt = |cards| EffectOutcome::count(0).with_execution_fact(ExecutionFact::ActionObjects {
            action: ironsmith_core::PriorEffectAction::Revealed, player: Some(player), objects: cards,
        });
        ctx.store_outcome(reveal, receipt(vec![card.clone()]));
        assert_eq!(resolve_prior_effect_metric(&game, &ctx, reveal, &query).unwrap(), 1);
        ctx.store_outcome(reveal, EffectOutcome::aggregate_replacement_outcomes(
            receipt(Vec::new()), [receipt(vec![card.clone()])],
        ));
        assert_eq!(resolve_prior_effect_metric(&game, &ctx, reveal, &query).unwrap(), 0);
        ctx.store_outcome(reveal, EffectOutcome::aggregate_replacement_outcomes(
            EffectOutcome::count(0), [receipt(vec![card])],
        ));
        assert!(matches!(resolve_prior_effect_metric(&game, &ctx, reveal, &query),
            Err(ExecutionError::IncompleteEvidence(_))));
    }
}
