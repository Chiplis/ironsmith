//! Effect composition effects.
//!
//! This module contains effects that compose or wrap other effects:
//! - `WithId` - Track an effect's result for later reference
//! - `May` - Optional effect execution
//! - `If` - Conditional branching based on prior effect results
//! - `ForEachObject` - Iterate over objects
//! - `ForPlayers` - Iterate over players (generalizes ForEachOpponent)
//! - `ForEachTagged` - Iterate over tagged objects
//! - `ForEachControllerOfTagged` - Group tagged objects by controller and iterate
//! - `ForEachTaggedPlayer` - Iterate over tagged players
//! - `Conditional` - Game state branching
//! - `ChooseMode` - Modal spell handling
//! - `Tagged` - Tag targets for cross-effect reference
//! - `ChooseObjects` - Interactive object selection with tagging
//! - `Vote` - Council's dilemma and voting mechanics

mod choose_mode;
mod choose_objects;
mod conditional;
mod for_each_object;
mod for_each_tagged;
mod for_players;
mod if_effect;
mod mechanic_actions;
mod may;
mod sequence;
mod tag_attached_to_source;
mod tag_triggering_damage_target;
mod tag_triggering_object;
mod tagged;
mod target_only;
mod unless_action;
mod unless_pays;
mod vote;
mod with_id;

pub use choose_mode::ChooseModeEffect;
pub use choose_objects::ChooseObjectsEffect;
pub use conditional::ConditionalEffect;
pub use for_each_object::ForEachObject;
pub use for_each_tagged::{
    ForEachControllerOfTaggedEffect, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
};
pub use for_players::ForPlayersEffect;
pub use if_effect::IfEffect;
pub use mechanic_actions::{
    AdaptEffect, BolsterEffect, CounterAbilityEffect, ExploreEffect, ManifestDreadEffect,
    OpenAttractionEffect, SupportEffect,
};
pub use may::MayEffect;
pub use sequence::SequenceEffect;
pub use tag_attached_to_source::TagAttachedToSourceEffect;
pub use tag_triggering_damage_target::TagTriggeringDamageTargetEffect;
pub use tag_triggering_object::TagTriggeringObjectEffect;
pub use tagged::{TagAllEffect, TaggedEffect};
pub use target_only::TargetOnlyEffect;
pub use unless_action::UnlessActionEffect;
pub use unless_pays::UnlessPaysEffect;
pub use vote::{VoteEffect, VoteOption};
pub use with_id::WithIdEffect;
