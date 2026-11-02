//! Observation owner for an actual battlefield arrival.

use crate::effects::ExecutionError;
use crate::events::EnterBattlefieldEvent;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Capture the committed entry's tapped state, look-back and action grouping.
/// Callers retain the observation until their original batch is complete, then
/// use the shared entry-freezing owner before publishing or running additions.
pub fn battlefield_entry_observation(
    game: &mut GameState,
    object: ObjectId,
    from: Zone,
    enters_tapped: bool,
    provenance: ProvNodeId,
    lookback: Vec<ObjectSnapshot>,
) -> Result<TriggerEvent, ExecutionError> {
    let entrant = game
        .object(object)
        .ok_or(ExecutionError::ObjectNotFound(object))?;
    if entrant.zone != Zone::Battlefield {
        return Err(ExecutionError::InternalError(
            "battlefield entry observation requires a committed entrant".into(),
        ));
    }
    let entry = if enters_tapped {
        EnterBattlefieldEvent::tapped(object, from)
    } else {
        EnterBattlefieldEvent::new(object, from)
    };
    observation_for_entry(game, entry, provenance, lookback)
}

pub(crate) fn battlefield_creation_observation(
    game: &mut GameState, object: ObjectId,
    creation: crate::events::CreateCardsEvent, enters_tapped: bool,
    provenance: ProvNodeId, lookback: Vec<ObjectSnapshot>,
) -> Result<TriggerEvent, ExecutionError> {
    if !game.object(object).is_some_and(|object| object.zone == Zone::Battlefield)
        || game.is_originless_card_entry_pending(object)
    {
        return Err(ExecutionError::IncompleteEvidence("creation entry observation has no actual battlefield arrival".into()));
    }
    let mut entry = EnterBattlefieldEvent::originless_creation(object, creation);
    entry.enters_tapped = enters_tapped;
    observation_for_entry(game, entry, provenance, lookback)
}

fn observation_for_entry(
    game: &mut GameState, entry: EnterBattlefieldEvent, provenance: ProvNodeId,
    lookback: Vec<ObjectSnapshot>,
) -> Result<TriggerEvent, ExecutionError> {
    let mut observation = TriggerEvent::new_with_provenance(entry, provenance)
        .with_lookback_source_snapshots(lookback);
    // Held observations may be published after their owner closes the batch.
    // Capture its identity now rather than depending on publication timing.
    if let Some(batch) = game.simultaneous_action_batch() {
        observation = observation.with_simultaneous_batch(batch);
    }
    Ok(observation)
}
