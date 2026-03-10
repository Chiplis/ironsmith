//! Zone change effects.
//!
//! This module contains effects that move objects between zones,
//! such as destroy, exile, sacrifice, and return to hand.

use crate::DecisionMaker;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::zone::Zone;

mod battlefield_entry;
mod destroy;
mod destroy_no_regen;
mod exile;
mod exile_until_source_leaves;
mod haunt_exile;
mod move_to_library_nth_from_top;
mod move_to_library_second_from_top;
mod move_to_zone;
mod put_onto_battlefield;
mod reorder_graveyard;
mod reorder_library_top;
mod return_all_to_battlefield;
mod return_from_graveyard_or_exile_to_battlefield;
mod return_from_graveyard_to_battlefield;
mod return_from_graveyard_to_hand;
mod return_to_hand;
mod sacrifice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedZoneChange {
    pub final_zone: Zone,
    pub new_object_id: Option<ObjectId>,
}

pub(crate) fn finalize_zone_change_move(
    game: &mut GameState,
    object_id: ObjectId,
    final_zone: Zone,
) -> AppliedZoneChange {
    AppliedZoneChange {
        final_zone,
        new_object_id: game.move_object(object_id, final_zone),
    }
}

pub(crate) fn apply_zone_change(
    game: &mut GameState,
    object_id: ObjectId,
    from: Zone,
    to: Zone,
    decision_maker: &mut (impl DecisionMaker + ?Sized),
) -> EventOutcome<AppliedZoneChange> {
    match process_zone_change(game, object_id, from, to, decision_maker) {
        EventOutcome::Proceed(final_zone) => {
            EventOutcome::Proceed(finalize_zone_change_move(game, object_id, final_zone))
        }
        EventOutcome::Prevented => EventOutcome::Prevented,
        EventOutcome::Replaced => EventOutcome::Replaced,
        EventOutcome::NotApplicable => EventOutcome::NotApplicable,
    }
}

pub(crate) use battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};

pub use destroy::DestroyEffect;
pub use destroy_no_regen::DestroyNoRegenerationEffect;
pub use exile::ExileEffect;
pub use exile_until_source_leaves::{ExileUntilDuration, ExileUntilEffect};
pub use haunt_exile::HauntExileEffect;
pub use move_to_library_nth_from_top::MoveToLibraryNthFromTopEffect;
pub use move_to_library_second_from_top::MoveToLibrarySecondFromTopEffect;
pub use move_to_zone::{BattlefieldController, MoveToZoneEffect};
pub use put_onto_battlefield::PutOntoBattlefieldEffect;
pub use reorder_graveyard::ReorderGraveyardEffect;
pub use reorder_library_top::ReorderLibraryTopEffect;
pub use return_all_to_battlefield::ReturnAllToBattlefieldEffect;
pub use return_from_graveyard_or_exile_to_battlefield::ReturnFromGraveyardOrExileToBattlefieldEffect;
pub use return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefieldEffect;
pub use return_from_graveyard_to_hand::ReturnFromGraveyardToHandEffect;
pub use return_to_hand::ReturnToHandEffect;
pub use sacrifice::{SacrificeEffect, SacrificeTargetEffect};
