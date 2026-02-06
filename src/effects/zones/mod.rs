//! Zone change effects.
//!
//! This module contains effects that move objects between zones,
//! such as destroy, exile, sacrifice, and return to hand.

mod destroy;
mod exile;
mod exile_from_hand_as_cost;
mod move_to_zone;
mod put_onto_battlefield;
mod return_from_graveyard_or_exile_to_battlefield;
mod return_from_graveyard_to_battlefield;
mod return_from_graveyard_to_hand;
mod return_to_hand;
mod sacrifice;

pub use destroy::DestroyEffect;
pub use exile::ExileEffect;
pub use exile_from_hand_as_cost::ExileFromHandAsCostEffect;
pub use move_to_zone::MoveToZoneEffect;
pub use put_onto_battlefield::PutOntoBattlefieldEffect;
pub use return_from_graveyard_or_exile_to_battlefield::ReturnFromGraveyardOrExileToBattlefieldEffect;
pub use return_from_graveyard_to_battlefield::ReturnFromGraveyardToBattlefieldEffect;
pub use return_from_graveyard_to_hand::ReturnFromGraveyardToHandEffect;
pub use return_to_hand::ReturnToHandEffect;
pub use sacrifice::{SacrificeEffect, SacrificeTargetEffect};
