//! Miscellaneous triggers.

mod any_of;
mod becomes_tapped;
mod becomes_untapped;
mod chapter_ability_resolved;
mod class_becomes_level;
mod condition_qualified;
mod zone_gated;
mod each_players_turn;
mod event_kind;
mod expend;
mod keyword_action;
mod mana_added;
mod player_changes_tap_state;
mod attachment_changed;
mod phasing_changed;
mod player_attack_declaration;
pub use player_attack_declaration::PlayerAttackDeclarationTrigger;
mod cards_milled;
pub use cards_milled::CardsMilledTrigger;
pub use phasing_changed::PhasingChangedTrigger;
pub use attachment_changed::AttachmentChangedTrigger;
mod control_changed;
pub use control_changed::ControlChangedTrigger;
mod ring_bearer_chosen;
pub use ring_bearer_chosen::RingBearerChosenTrigger;
mod permanent_becomes_tapped;
mod permanent_becomes_untapped;
mod permanent_sacrificed_or_destroyed;
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
mod permanent_lifecycle;
pub use permanent_lifecycle::PermanentMutatesTrigger;
mod wins_clash;

pub use any_of::AnyOfTrigger;
pub use becomes_tapped::BecomesTappedTrigger;
pub use becomes_untapped::BecomesUntappedTrigger;
pub use chapter_ability_resolved::FinalChapterAbilityResolvedTrigger;
pub use class_becomes_level::ClassBecomesLevelTrigger;
pub use condition_qualified::ConditionQualifiedTrigger;
pub use zone_gated::ZoneGatedTrigger;
pub use each_players_turn::EachPlayersTurnTrigger;
pub use event_kind::{
    EventKindTrigger, SourceControllerLosesControlTrigger, ThisEventObjectTrigger,
};
pub use expend::ExpendTrigger;
pub use keyword_action::KeywordActionTrigger;
pub use mana_added::ManaAddedTrigger;
pub use player_changes_tap_state::PlayerChangesTapStateTrigger;
pub use permanent_becomes_tapped::PermanentBecomesTappedTrigger;
pub use permanent_becomes_untapped::PermanentBecomesUntappedTrigger;
pub use permanent_sacrificed_or_destroyed::{
    PermanentDestroyedTrigger, PermanentSacrificedTrigger,
};
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

mod qualified_die_roll;
pub use qualified_die_roll::QualifiedDieRollTrigger;
mod damage_prevented_this_way;
pub use damage_prevented_this_way::DamagePreventedThisWayTrigger;

mod player_becomes_monarch;
pub use player_becomes_monarch::PlayerBecomesMonarchTrigger;
