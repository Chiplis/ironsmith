use crate::executor::ExecutionContext;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;

/// Controller policy when an object enters the battlefield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryController {
    Preserve,
    Owner,
    Specific(PlayerId),
}

/// Config for moving an object to the battlefield through ETB processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattlefieldEntryOptions {
    pub controller: BattlefieldEntryController,
    pub tapped: bool,
}

impl BattlefieldEntryOptions {
    pub(crate) fn preserve(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Preserve,
            tapped,
        }
    }

    pub(crate) fn owner(tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Owner,
            tapped,
        }
    }

    pub(crate) fn specific(controller: PlayerId, tapped: bool) -> Self {
        Self {
            controller: BattlefieldEntryController::Specific(controller),
            tapped,
        }
    }
}

/// Result for a move-to-battlefield attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlefieldEntryOutcome {
    Moved(ObjectId),
    Prevented,
}

/// Move an object to the battlefield with ETB replacement processing and policy hooks.
pub(crate) fn move_to_battlefield_with_options(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    object_id: ObjectId,
    options: BattlefieldEntryOptions,
) -> BattlefieldEntryOutcome {
    let Some(result) = game.move_object_with_etb_processing_with_dm(
        object_id,
        Zone::Battlefield,
        &mut ctx.decision_maker,
    ) else {
        return BattlefieldEntryOutcome::Prevented;
    };

    let new_id = result.new_id;

    if let Some(obj) = game.object_mut(new_id) {
        match options.controller {
            BattlefieldEntryController::Preserve => {}
            BattlefieldEntryController::Owner => {
                obj.controller = obj.owner;
            }
            BattlefieldEntryController::Specific(controller) => {
                obj.controller = controller;
            }
        }
    }

    if let Some(obj) = game.object(new_id) {
        game.objects_entered_battlefield_this_turn
            .insert(obj.stable_id, obj.controller);
    }

    if options.tapped && !result.enters_tapped {
        game.tap(new_id);
    }

    BattlefieldEntryOutcome::Moved(new_id)
}
