use super::*;

/// Perform the CR 505.5 / 717.5 precombat-main Attraction turn-based action.
///
/// The roll is made only if the active player controls at least one face-up
/// Attraction. Every controlled Attraction whose physical printing has the
/// result lit is visited simultaneously, and its stored Visit program becomes
/// a normal triggered ability waiting to be put on the stack.
pub fn roll_to_visit_attractions(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<Option<u32>, GameLoopError> {
    roll_to_visit_attractions_with_dm(
        game,
        trigger_queue,
        &mut crate::decision::AutoPassDecisionMaker,
    )
}

/// The interactive version used by the turn runner; callers must preserve the
/// transaction while the decision maker is awaiting a choice.
pub fn roll_to_visit_attractions_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<Option<u32>, GameLoopError> {
    let checkpoint = game.clone();
    let queue_checkpoint = trigger_queue.clone();
    let result = roll_to_visit_attractions_inner(game, trigger_queue, decision_maker);
    let pending = decision_maker.awaiting_choice();
    if result.is_err() || pending {
        game.restore_execution_checkpoint(checkpoint, pending && result.is_ok());
        *trigger_queue = queue_checkpoint;
    }
    if pending && result.is_ok() {
        return Ok(None);
    }
    result
}

fn roll_to_visit_attractions_inner(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<Option<u32>, GameLoopError> {
    let player = game.turn.active_player;
    if !game.face_up_attractions().iter().any(|object| {
        game.object(*object).is_some_and(|candidate| {
            !game.is_phased_out(*object)
                && candidate.zone == Zone::Battlefield
                && game.current_controller(*object) == Some(player)
        })
    }) {
        return Ok(None);
    }

    let rule_source = ObjectId::from_raw(0);
    let mut context = ExecutionContext::new(rule_source, player, decision_maker);
    let Some(roll) =
        crate::effects::player::roll_to_visit_attractions::roll_to_visit_attractions_for_player(
            game,
            &mut context,
            player,
        )
        .map_err(GameLoopError::ExecutionFailed)?
    else {
        return Ok(None);
    };
    // The root trigger-queue handoff consumes its native reported events.
    try_queue_triggers_from_reported_events(
        game,
        trigger_queue,
        roll.roll_outputs.into_outcome().events,
        true,
    )?;
    let provenance = crate::provenance::ProvNodeId::default();

    let visit_events = roll
        .visits
        .into_iter()
        .map(|visit| TriggerEvent::new_with_provenance(visit, provenance))
        .collect::<Vec<_>>();

    // Record the whole simultaneous visit batch before checking observers, so
    // turn-history conditions (for example Soul Swindler) see the completed
    // turn-based action while any resulting triggers are being created.
    queue_triggers_for_simultaneous_events(game, trigger_queue, visit_events.clone());

    Ok(Some(roll.result))
}
