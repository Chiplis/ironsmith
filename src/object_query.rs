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
        Some(Zone::Battlefield) => game.battlefield.clone(),
        Some(Zone::Graveyard) => game
            .players
            .iter()
            .flat_map(|player| player.graveyard.iter().copied())
            .collect(),
        Some(Zone::Hand) => game
            .players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect(),
        Some(Zone::Library) => game
            .players
            .iter()
            .flat_map(|player| player.library.iter().copied())
            .collect(),
        Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
        Some(Zone::Exile) => game.exile.clone(),
        Some(Zone::Command) => game.command_zone.clone(),
        None => game.battlefield.clone(),
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
