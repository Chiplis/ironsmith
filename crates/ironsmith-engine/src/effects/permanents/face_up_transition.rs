//! Shared physical face-up transition and its immediate characteristic choices.
//! Callers retain authentication, costs, counters and completion publication.

use crate::decision::DecisionMaker;
use crate::effects::ExecutionError;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

/// Preserve the caller's rules for who makes immediate characteristic choices.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FaceUpChoiceController {
    Actor(PlayerId),
    CurrentOr(PlayerId),
}

pub(crate) struct FaceUpTransition {
    pub(crate) was_on_battlefield: bool,
    pub(crate) outputs: Vec<crate::effects::CompletedEffectOutputs>,
}

/// Run inside the caller's observation/rollback boundary. A pending choice
/// leaves a physically changed transition; the enclosing owner must suspend
/// and restore its transaction rather than publish a completion event.
pub(crate) fn turn_face_up_with_choices(
    game: &mut GameState,
    object_id: ObjectId,
    choice_controller: FaceUpChoiceController,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<Option<FaceUpTransition>, ExecutionError> {
    let Some(object) = game.object(object_id) else {
        return Ok(None);
    };
    let was_on_battlefield = object.zone == Zone::Battlefield;
    if !game
        .set_face_up(object_id)
        .map_err(ExecutionError::ContinuousDiscovery)?
    {
        return Ok(None);
    }
    let mut outputs = Vec::new();
    if was_on_battlefield {
        let controller = match choice_controller {
            FaceUpChoiceController::Actor(actor) => actor,
            FaceUpChoiceController::CurrentOr(fallback) => {
                game.current_controller(object_id).unwrap_or(fallback)
            }
        };
        outputs = game.execute_as_enters_effect_programs_for_turn_face_up_with_outputs(
            object_id,
            controller,
            decision_maker,
        )?;
        if !decision_maker.awaiting_choice() {
            game.apply_power_toughness_choice_as_enters_or_turns_face_up(
                object_id,
                controller,
                decision_maker,
            );
        }
    }
    Ok(Some(FaceUpTransition {
        was_on_battlefield,
        outputs,
    }))
}
