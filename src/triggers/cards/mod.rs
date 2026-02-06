//! Card-related triggers (draw, discard).

mod card_put_into_your_graveyard;
mod player_draws_card;
mod you_discard_card;

pub use card_put_into_your_graveyard::CardPutIntoYourGraveyardTrigger;
pub use player_draws_card::PlayerDrawsCardTrigger;
pub use you_discard_card::YouDiscardCardTrigger;
