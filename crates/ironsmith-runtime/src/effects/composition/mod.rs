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

mod aura_swap;
mod behold;
mod bid_life;
mod choose_mode;
mod choose_mode_runtime;
mod choose_objects;
mod choose_objects_runtime;
mod choose_spell_cast_history;
mod conditional;
mod cumulative_upkeep;
mod emit_gift_given;
mod emit_keyword_action;
mod execute_with_source;
mod for_each_correlated_result;
mod for_each_object;
mod for_each_tagged;
mod for_players;
mod grant_repeatable_mana_payment_action;
mod if_effect;
mod local_rewrite;
mod mana_restricted;
mod mana_retained;
mod may;
pub(crate) mod mechanic_actions;
mod reflexive_trigger;
mod repeat_effects;
mod repeat_process;
mod repeat_process_prompt;
mod secret_choice;
mod sequence;
mod tag_attached_to_source;
mod tag_matching_objects;
mod tag_other_block_participant;
mod tag_triggering_attacker;
mod tag_triggering_blockers;
mod tag_triggering_damage_target;
mod tag_triggering_object;
mod tag_triggering_source;
mod tagged;
mod tagging_runtime;
mod target_metadata;
mod target_only;
mod unless_action;
mod unless_pays;
mod villainous_choice;
mod vote;
mod vote_runtime;
mod with_id;

pub use aura_swap::AuraSwapEffect;
pub use behold::BeholdEffect;
pub use bid_life::{BidLifeEffect, LifeBidStart};
pub use choose_mode::ChooseModeEffect;
pub use choose_objects::ChooseObjectsEffect;
pub use choose_spell_cast_history::ChooseSpellCastHistoryEffect;
pub use conditional::ConditionalEffect;
pub use cumulative_upkeep::CumulativeUpkeepEffect;
pub use emit_gift_given::EmitGiftGivenEffect;
pub use emit_keyword_action::EmitKeywordActionEffect;
pub use execute_with_source::ExecuteWithSourceEffect;
pub use for_each_correlated_result::ForEachObjectCorrelatedResultEffect;
pub use for_each_object::ForEachObject;
pub use for_each_tagged::{
    ForEachControllerOfTaggedEffect, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
};
pub use for_players::ForPlayersEffect;
pub use grant_repeatable_mana_payment_action::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect;
pub use if_effect::IfEffect;
pub use local_rewrite::LocalRewriteEffect;
pub use mana_restricted::ManaRestrictedEffect;
pub use mana_retained::ManaRetainedEffect;
pub use may::MayEffect;
pub use mechanic_actions::{
    AdaptEffect, AmplifyEffect, BackupEffect, BolsterEffect, CastEncodedCardCopyEffect,
    CipherEffect, CounterAbilityEffect, DevourEffect, ExploreEffect, ManifestCardFromHandEffect,
    ManifestDreadEffect, ManifestObjectsEffect, ManifestTopCardOfLibraryEffect,
    OpenAttractionEffect, PopulateEffect, SupportEffect,
};
pub use reflexive_trigger::ReflexiveTriggerEffect;
pub use repeat_effects::RepeatEffectsEffect;
pub use repeat_process::RepeatProcessEffect;
pub use repeat_process_prompt::RepeatProcessPromptEffect;
pub use secret_choice::{SecretChoiceEffect, SecretChoiceResult};
pub use sequence::SequenceEffect;
pub use tag_attached_to_source::TagAttachedToSourceEffect;
pub use tag_matching_objects::TagMatchingObjectsEffect;
pub use tag_other_block_participant::TagOtherBlockParticipantEffect;
pub use tag_triggering_attacker::TagTriggeringAttackerEffect;
pub use tag_triggering_blockers::TagTriggeringBlockersEffect;
pub use tag_triggering_damage_target::TagTriggeringDamageTargetEffect;
pub use tag_triggering_object::TagTriggeringObjectEffect;
pub use tag_triggering_source::TagTriggeringSourceEffect;
pub use tagged::{TagAllEffect, TaggedEffect};
pub use target_only::TargetOnlyEffect;
pub use unless_action::UnlessActionEffect;
pub use unless_pays::UnlessPaysEffect;
pub use villainous_choice::VillainousChoiceEffect;
pub use vote::{
    VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteOption, VoteResult,
};
pub use with_id::WithIdEffect;
