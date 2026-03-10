//! Miscellaneous triggers.

mod becomes_tapped;
mod becomes_untapped;
mod each_players_turn;
mod event_kind;
mod expend;
mod keyword_action;
mod permanent_becomes_tapped;
mod permanent_turned_face_up;
mod player_plays_land;
mod player_sacrifices;
mod transforms;

pub use becomes_tapped::BecomesTappedTrigger;
pub use becomes_untapped::BecomesUntappedTrigger;
pub use each_players_turn::EachPlayersTurnTrigger;
pub use event_kind::{EventKindTrigger, ThisEventObjectTrigger};
pub use expend::ExpendTrigger;
pub use keyword_action::KeywordActionTrigger;
pub use permanent_becomes_tapped::PermanentBecomesTappedTrigger;
pub use permanent_turned_face_up::PermanentTurnedFaceUpTrigger;
pub use player_plays_land::PlayerPlaysLandTrigger;
pub use player_sacrifices::PlayerSacrificesTrigger;
pub use transforms::TransformsTrigger;
