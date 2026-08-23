//! Card-related triggers (draw, discard).

mod card_put_into_your_graveyard;
mod player_draws_card;
mod player_draws_card_except_first_in_draw_step;
mod player_draws_nth_card_each_turn;
mod you_discard_card;

pub use card_put_into_your_graveyard::CardPutIntoYourGraveyardTrigger;
pub use player_draws_card::PlayerDrawsCardTrigger;
pub use player_draws_card_except_first_in_draw_step::PlayerDrawsCardExceptFirstInDrawStepTrigger;
pub use player_draws_nth_card_each_turn::{
    PlayerDrawsNthCardEachTurnTrigger, PlayerDrawsNumberedCardsEachTurnTrigger,
};
pub use you_discard_card::YouDiscardCardTrigger;
