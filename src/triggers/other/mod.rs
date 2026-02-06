//! Miscellaneous triggers.

mod becomes_tapped;
mod becomes_untapped;
mod each_players_turn;
mod event_kind;
mod keyword_action;
mod permanent_becomes_tapped;
mod player_sacrifices;
mod transforms;

pub use becomes_tapped::BecomesTappedTrigger;
pub use becomes_untapped::BecomesUntappedTrigger;
pub use each_players_turn::EachPlayersTurnTrigger;
pub use event_kind::{EventKindTrigger, ThisEventObjectTrigger};
pub use keyword_action::KeywordActionTrigger;
pub use permanent_becomes_tapped::PermanentBecomesTappedTrigger;
pub use player_sacrifices::PlayerSacrificesTrigger;
pub use transforms::TransformsTrigger;
