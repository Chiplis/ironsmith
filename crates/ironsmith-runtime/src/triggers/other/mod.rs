//! Miscellaneous triggers.

mod any_of;
mod becomes_tapped;
mod becomes_untapped;
mod chapter_ability_resolved;
mod each_players_turn;
mod event_kind;
mod expend;
mod keyword_action;
mod mana_added;
mod permanent_becomes_tapped;
mod permanent_turned_face_up;
mod player_coin_flip_result;
mod player_gives_gift;
mod player_plays_land;
mod player_reveals_card;
mod player_rolls_die;
mod player_rolls_result;
mod player_sacrifices;
mod player_searches_library;
mod player_shuffles_library;
mod transforms;
mod wins_clash;

pub use any_of::AnyOfTrigger;
pub use becomes_tapped::BecomesTappedTrigger;
pub use becomes_untapped::BecomesUntappedTrigger;
pub use chapter_ability_resolved::FinalChapterAbilityResolvedTrigger;
pub use each_players_turn::EachPlayersTurnTrigger;
pub use event_kind::{
    EventKindTrigger, SourceControllerLosesControlTrigger, ThisEventObjectTrigger,
};
pub use expend::ExpendTrigger;
pub use keyword_action::KeywordActionTrigger;
pub use mana_added::ManaAddedTrigger;
pub use permanent_becomes_tapped::PermanentBecomesTappedTrigger;
pub use permanent_turned_face_up::PermanentTurnedFaceUpTrigger;
pub use player_coin_flip_result::PlayerCoinFlipResultTrigger;
pub use player_gives_gift::PlayerGivesGiftTrigger;
pub use player_plays_land::PlayerPlaysLandTrigger;
pub use player_reveals_card::PlayerRevealsCardTrigger;
pub use player_rolls_die::PlayerRollsDieTrigger;
pub use player_rolls_result::{PlayerRollsHighestNaturalResultTrigger, PlayerRollsResultTrigger};
pub use player_sacrifices::PlayerSacrificesTrigger;
pub use player_searches_library::PlayerSearchesLibraryTrigger;
pub use player_shuffles_library::PlayerShufflesLibraryTrigger;
pub use transforms::TransformsTrigger;
pub use wins_clash::WinsClashTrigger;
