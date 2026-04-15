use super::*;

// ============================================================================
// Saga Support
// ============================================================================

/// Add lore counters to sagas at the start of precombat main phase.
///
/// This should be called once at the start of the precombat main phase, before
/// players receive priority.
///
/// Per MTG rules, this checks the CALCULATED subtypes (after continuous effects)
/// to determine if a permanent is still a Saga. For example, under Blood Moon,
/// Urza's Saga becomes a basic Mountain and loses its Saga subtype, so it
/// won't gain lore counters.
pub fn add_saga_lore_counters(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    let active_player = game.turn.active_player;

    // Collect sagas controlled by active player
    // IMPORTANT: Use calculated_subtypes to check if the permanent is STILL a Saga
    // after continuous effects are applied (e.g., Blood Moon removes Saga subtype)
    let sagas: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| {
            let obj = game.object(id)?;
            // Check calculated subtypes (after continuous effects), not base subtypes
            let subtypes = game.calculated_subtypes(id);
            if subtypes.contains(&Subtype::Saga) && obj.controller == active_player {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    for saga_id in sagas {
        add_lore_counter_and_check_chapters(game, saga_id, trigger_queue);
    }
}

/// Add a lore counter to a saga and check for chapter triggers.
///
/// This uses the normal trigger system: adds a lore counter, generates a
/// CounterPlaced event, and lets check_triggers find matching chapter abilities.
/// Chapter triggers use threshold-crossing logic: they fire when the lore count
/// crosses a chapter's threshold, allowing chapters to trigger multiple times
/// if counters are removed and re-added.
pub fn add_lore_counter_and_check_chapters(
    game: &mut GameState,
    saga_id: ObjectId,
    trigger_queue: &mut TriggerQueue,
) {
    // Add lore counter and get the CounterPlaced event
    let Some(event) = game.add_counters(saga_id, CounterType::Lore, 1) else {
        return;
    };

    // Check triggers - this will find any saga chapter abilities that should fire
    // based on whether the threshold was crossed by this counter addition
    queue_triggers_from_event(game, trigger_queue, event, false);
}

/// Mark a saga as having resolved its final chapter.
///
/// Call this after a saga's final chapter ability finishes resolving.
/// The saga will then be sacrificed as a state-based action IF it still has
/// enough lore counters. This function unconditionally marks the saga;
/// the SBA checks the lore counter count before sacrificing.
pub fn mark_saga_final_chapter_resolved(game: &mut GameState, saga_id: ObjectId) {
    if let Some(saga) = game.object(saga_id)
        && saga.subtypes.contains(&Subtype::Saga)
    {
        // Always mark as resolved - the SBA will check lore counters before sacrificing
        game.set_saga_final_chapter_resolved(saga_id);
    }
}
