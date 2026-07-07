//! Shared object candidate query helpers.
//!
//! These helpers centralize "which object IDs are candidates for this zone/filter"
//! so value/condition/effect resolution paths stay in sync.

use std::collections::HashSet;

use crate::filter::ObjectFilter;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;

/// Collect candidate object IDs for a zone.
///
/// When `zone` is `None`, this defaults to battlefield candidates.
pub(crate) fn candidate_ids_for_zone(game: &GameState, zone: Option<Zone>) -> Vec<ObjectId> {
    match zone {
        Some(zone) => game.zone_ids(zone).collect(),
        None => game.zone_ids(Zone::Battlefield).collect(),
    }
}

pub(crate) fn for_each_candidate_id_for_zone(
    game: &GameState,
    zone: Option<Zone>,
    mut visitor: impl FnMut(ObjectId),
) {
    let zone = zone.unwrap_or(Zone::Battlefield);
    for id in game.zone_ids(zone) {
        visitor(id);
    }
}

/// Collect candidate object IDs for a full object filter.
///
/// This respects explicit `filter.zone` and broadens to nested `any_of` zones
/// when present.
pub(crate) fn candidate_ids_for_filter(game: &GameState, filter: &ObjectFilter) -> Vec<ObjectId> {
    if let Some(zone) = filter.zone {
        return candidate_ids_for_zone(game, Some(zone));
    }

    if filter.any_of.is_empty() {
        return candidate_ids_for_zone(game, None);
    }

    let mut ids = HashSet::new();
    for nested in &filter.any_of {
        for id in candidate_ids_for_zone(game, nested.zone) {
            ids.insert(id);
        }
    }

    if ids.is_empty() {
        candidate_ids_for_zone(game, None)
    } else {
        let mut ordered: Vec<_> = ids.into_iter().collect();
        ordered.sort();
        ordered
    }
}

pub(crate) fn for_each_candidate_id_for_filter(
    game: &GameState,
    filter: &ObjectFilter,
    mut visitor: impl FnMut(ObjectId),
) {
    if let Some(zone) = filter.zone {
        for_each_candidate_id_for_zone(game, Some(zone), visitor);
        return;
    }

    if filter.any_of.is_empty() {
        for_each_candidate_id_for_zone(game, None, visitor);
        return;
    }

    for id in candidate_ids_for_filter(game, filter) {
        visitor(id);
    }
}
