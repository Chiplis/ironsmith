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

mod behold;
mod choose_mode;
mod choose_mode_runtime;
mod choose_objects;
mod choose_objects_runtime;
mod choose_spell_cast_history;
mod conditional;
mod emit_gift_given;
mod emit_keyword_action;
mod for_each_object;
mod for_each_tagged;
mod for_players;
mod if_effect;
mod local_rewrite;
mod may;
mod mechanic_actions;
mod reflexive_trigger;
mod repeat_effects;
mod repeat_process;
mod repeat_process_prompt;
mod sequence;
mod tag_attached_to_source;
mod tag_matching_objects;
mod tag_triggering_damage_target;
mod tag_triggering_object;
mod tagged;
mod tagging_runtime;
mod target_metadata;
mod target_only;
mod unless_action;
mod unless_pays;
mod vote;
mod vote_runtime;
mod with_id;

pub use behold::BeholdEffect;
pub use choose_mode::ChooseModeEffect;
pub use choose_objects::ChooseObjectsEffect;
pub use choose_spell_cast_history::ChooseSpellCastHistoryEffect;
pub use conditional::ConditionalEffect;
pub use emit_gift_given::EmitGiftGivenEffect;
pub use emit_keyword_action::EmitKeywordActionEffect;
pub use for_each_object::ForEachObject;
pub use for_each_tagged::{
    ForEachControllerOfTaggedEffect, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
};
pub use for_players::ForPlayersEffect;
pub use if_effect::IfEffect;
pub use local_rewrite::LocalRewriteEffect;
pub use may::MayEffect;
pub use mechanic_actions::{
    AdaptEffect, BackupEffect, BolsterEffect, CastEncodedCardCopyEffect, CipherEffect,
    CounterAbilityEffect, DevourEffect, ExploreEffect, ManifestDreadEffect,
    ManifestTopCardOfLibraryEffect, OpenAttractionEffect, PopulateEffect, SupportEffect,
};
pub use reflexive_trigger::ReflexiveTriggerEffect;
pub use repeat_effects::RepeatEffectsEffect;
pub use repeat_process::RepeatProcessEffect;
pub use repeat_process_prompt::RepeatProcessPromptEffect;
pub use sequence::SequenceEffect;
pub use tag_attached_to_source::TagAttachedToSourceEffect;
pub use tag_matching_objects::TagMatchingObjectsEffect;
pub use tag_triggering_damage_target::TagTriggeringDamageTargetEffect;
pub use tag_triggering_object::TagTriggeringObjectEffect;
pub use tagged::{TagAllEffect, TaggedEffect};
pub use target_only::TargetOnlyEffect;
pub use unless_action::UnlessActionEffect;
pub use unless_pays::UnlessPaysEffect;
pub use vote::{
    VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteOption, VoteResult,
};
pub use with_id::WithIdEffect;
