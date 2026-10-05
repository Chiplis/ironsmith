use super::*;

use crate::events::other::DieRolledEvent;

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
    roll_to_visit_attractions_with_dm(game, trigger_queue, &mut crate::decision::AutoPassDecisionMaker)
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
    if pending && result.is_ok() { return Ok(None); }
    result
}

fn roll_to_visit_attractions_inner(
    game: &mut GameState, trigger_queue: &mut TriggerQueue,
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

    game.turn_store.turn_history.check_completed_die_roll_capacity(player, 1)?;
    let rule_source = ObjectId::from_raw(0);
    let mut context = ExecutionContext::new(rule_source, player, decision_maker);
    let Some(mut rolls) = crate::effects::player::die_roll_transaction::roll_dice_with_modifiers(
        game,
        &mut context,
        player,
        1,
        6,
    )
    .map_err(GameLoopError::ExecutionFailed)?
    else {
        return Ok(None);
    };
    let roll = rolls.remove(0);

    let ordinal = game.turn_store.turn_history.record_completed_die_rolls(player, &[roll.result], false)
        .map_err(GameLoopError::ExecutionFailed)?;
    game.mark_continuous_state_dirty();
    game.record_ui_effect_event(
        "attraction_visit_roll",
        Some(player),
        None,
        Vec::new(),
        Some(i64::from(roll.result)),
        Some("d6".to_string()),
    );
    let provenance = crate::provenance::ProvNodeId::default();
    queue_triggers_from_event(
        game,
        trigger_queue,
        TriggerEvent::new_with_provenance(
            DieRolledEvent::new_with_natural_result(
                player,
                rule_source,
                roll.natural_result,
                roll.result,
                6,
            )
            .for_attraction_visit().with_turn_ordinal(ordinal),
            provenance,
        ),
        true,
    );

    let visits = game.attraction_visit_profiles(player, roll.result);
    let visit_events = visits
        .iter()
        .map(|visit| {
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::VisitAttraction,
                    player,
                    visit.object,
                    1,
                ),
                provenance,
            )
        })
        .collect::<Vec<_>>();

    // Record the whole simultaneous visit batch before checking observers, so
    // turn-history conditions (for example Soul Swindler) see the completed
    // turn-based action while any resulting triggers are being created.
    queue_triggers_for_simultaneous_events(game, trigger_queue, visit_events.clone());

    Ok(Some(roll.result))
}
