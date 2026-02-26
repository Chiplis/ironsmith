//! Zone change effects.
//!
//! This module contains effects that move objects between zones,
//! such as destroy, exile, sacrifice, and return to hand.

mod battlefield_entry;
mod destroy;
mod destroy_no_regen;
mod exile;
mod haunt_exile;
mod exile_from_hand_as_cost;
mod exile_until_source_leaves;
mod move_to_zone;
mod put_onto_battlefield;
mod reorder_library_top;
mod reorder_graveyard;
mod return_all_to_battlefield;
mod return_from_graveyard_or_exile_to_battlefield;
mod return_from_graveyard_to_battlefield;
mod return_from_graveyard_to_hand;
mod return_to_hand;
mod sacrifice;

pub(crate) use battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};

pub use destroy::DestroyEffect;
pub use destroy_no_regen::DestroyNoRegenerationEffect;
pub use exile::ExileEffect;
pub use haunt_exile::HauntExileEffect;
pub use exile_from_hand_as_cost::ExileFromHandAsCostEffect;
pub use exile_until_source_leaves::{ExileUntilDuration, ExileUntilEffect};
pub use move_to_zone::{BattlefieldController, MoveToZoneEffect};
pub use put_onto_battlefield::PutOntoBattlefieldEffect;
pub use reorder_library_top::ReorderLibraryTopEffect;
pub use reorder_graveyard::ReorderGraveyardEffect;
pub use return_all_to_battlefield::ReturnAllToBattlefieldEffect;
pub use return_from_graveyard_or_exile_to_battlefield::ReturnFromGraveyardOrExileToBattlefieldEffect;
pub use return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefieldEffect;
pub use return_from_graveyard_to_hand::ReturnFromGraveyardToHandEffect;
pub use return_to_hand::ReturnToHandEffect;
pub use sacrifice::{SacrificeEffect, SacrificeTargetEffect};
