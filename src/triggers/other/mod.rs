//! Miscellaneous triggers.

mod becomes_tapped;
mod becomes_untapped;
mod each_players_turn;
mod event_kind;
mod expend;
mod keyword_action;
mod permanent_becomes_tapped;
mod permanent_turned_face_up;
mod player_gives_gift;
mod player_plays_land;
mod player_reveals_card;
mod player_sacrifices;
mod player_searches_library;
mod player_shuffles_library;
mod transforms;

pub use becomes_tapped::BecomesTappedTrigger;
pub use becomes_untapped::BecomesUntappedTrigger;
pub use each_players_turn::EachPlayersTurnTrigger;
pub use event_kind::{EventKindTrigger, ThisEventObjectTrigger};
pub use expend::ExpendTrigger;
pub use keyword_action::KeywordActionTrigger;
pub use permanent_becomes_tapped::PermanentBecomesTappedTrigger;
pub use permanent_turned_face_up::PermanentTurnedFaceUpTrigger;
pub use player_gives_gift::PlayerGivesGiftTrigger;
pub use player_plays_land::PlayerPlaysLandTrigger;
pub use player_reveals_card::PlayerRevealsCardTrigger;
pub use player_sacrifices::PlayerSacrificesTrigger;
pub use player_searches_library::PlayerSearchesLibraryTrigger;
pub use player_shuffles_library::PlayerShufflesLibraryTrigger;
pub use transforms::TransformsTrigger;
