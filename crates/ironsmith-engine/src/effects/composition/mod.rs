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

mod action_program;
mod action_units;
pub(crate) use action_program::projected_program_cursor;
pub use action_program::{
    ActionProgramCursor, ProgramAction, ProgramActionScope, ProgramCompletion,
    ProgramInstructionSelection, ProgramPreparation,
};
mod aura_swap;
mod behold;
mod bid_life;
mod branch_program;
mod captured_program;
pub(crate) use captured_program::CapturedProgramFrame;
mod choose_mode;
mod choose_mode_runtime;
pub(crate) mod choose_objects;
pub(crate) mod choose_objects_runtime;
mod choose_spell_cast_history;
pub(crate) mod collect_evidence;
mod collect_mana_payments;
mod bind_x_value;
mod completion_phase;
pub(crate) use completion_phase::{CompletionInput, CompletionPhase};
mod compound;
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
mod iteration_program;
mod local_rewrite;
mod mana_restricted;
mod mana_retained;
mod may;
pub(crate) mod mechanic_actions;
mod object_iteration;
mod prepared_branch;
mod prepared_iteration;
mod reflexive_trigger;
mod repeat_effects;
pub(crate) use repeat_effects::{
    RepetitionScope, finish_repeated_sequence_outcomes, resolve_repeat_count,
};
pub(crate) mod original_observations;
mod repeat_process;
mod repeat_process_prompt;
mod player_option_choice;
mod secret_choice;
mod sequence;
mod simultaneous;
pub(crate) use prepared_iteration::{
    scope_prepared_iteration, with_iteration_tags, with_object_iteration,
};
pub(crate) use simultaneous::{
    OriginalOutcomeAdapter, adapt_original_outcome_with_outputs, with_held_original_triggers,
    with_original_execution_context,
};
pub(crate) use simultaneous::{
    complete_prepared_original, complete_prepared_original_with_grouping,
    compose_original_commits_with_fallible_projection_outputs,
    compose_original_commits_with_outputs, compose_original_commits_with_projection_outputs,
    inherit_observed_events, inherit_original_observations,
};
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
pub(crate) use choose_mode_runtime::{
    previously_chosen_mode_restriction, restricted_mode_was_chosen,
};
pub use choose_objects::ChooseObjectsEffect;
pub use choose_spell_cast_history::ChooseSpellCastHistoryEffect;
pub use collect_evidence::CollectEvidenceEffect;
pub use collect_mana_payments::CollectManaPaymentsEffect;
pub use bind_x_value::BindXValueEffect;
pub(crate) use compound::{
    execute_checkpoint_transaction, execute_compound, execute_decision_transaction,
    execute_error_transaction_if, execute_optional_world_transaction,
    execute_original_view_transaction, execute_result_checkpoint_transaction,
    execute_result_decision_checkpoint_transaction, execute_result_decision_transaction,
    execute_result_transaction, execute_transaction, execute_transaction_from_body,
    execute_world_checkpoint_transaction, execute_world_context_checkpoint_transaction,
    execute_world_error_transaction, execute_world_result_transaction,
};
pub use conditional::ConditionalEffect;
pub use cumulative_upkeep::CumulativeUpkeepEffect;
pub use emit_gift_given::EmitGiftGivenEffect;
pub use emit_keyword_action::EmitKeywordActionEffect;
pub(crate) use emit_keyword_action::{
    complete_keyword_action, complete_keyword_action_with_outputs,
    complete_keyword_action_with_result, observe_keyword_action_completion,
    observe_keyword_action_completion_with_outputs, publish_keyword_action_completion,
    publish_keyword_action_completion_receipt,
};
pub use execute_with_source::ExecuteWithSourceEffect;
pub use for_each_correlated_result::ForEachObjectCorrelatedResultEffect;
pub use for_each_object::ForEachObject;
pub use for_each_tagged::{
    ForEachControllerOfTaggedEffect, ForEachTaggedEffect, ForEachTaggedPlayerEffect,
};
pub use for_players::ForPlayersEffect;
pub(crate) use for_players::execute_player_occurrences_with_outputs;
pub use grant_repeatable_mana_payment_action::{
    GrantEndThisEffectPaymentEffect, GrantRepeatableManaPaymentActionUntilEndOfTurnEffect,
};
pub use if_effect::IfEffect;
pub use local_rewrite::LocalRewriteEffect;
pub use mana_restricted::ManaRestrictedEffect;
pub use mana_retained::ManaRetainedEffect;
pub use may::MayEffect;
pub use mechanic_actions::{
    AdaptEffect, AmplifyEffect, BackupEffect, BolsterEffect, CastEncodedCardCopyEffect,
    CipherEffect, CounterAbilityEffect, DevourEffect, ExploreEffect, ManifestCardFromHandEffect,
    ManifestDreadEffect, ManifestObjectsEffect, ManifestTopCardOfLibraryEffect,
    OpenAttractionEffect, PopulateEffect, ResolvesDespiteIllegalTargetsEffect, SupportEffect,
};
pub use reflexive_trigger::ReflexiveTriggerEffect;
pub(crate) use reflexive_trigger::{
    PendingReflexiveTrigger, queue_reflexive_trigger, queue_reflexive_trigger_with_source_snapshot,
    reflexive_trigger_stack_entry,
};
pub use repeat_effects::RepeatEffectsEffect;
pub use repeat_process::RepeatProcessEffect;
pub use repeat_process_prompt::RepeatProcessPromptEffect;
pub use secret_choice::{SecretChoiceEffect, SecretChoiceResult};
pub use sequence::SequenceEffect;
pub(crate) use sequence::execute_checked_program_with_outputs;
pub(crate) use sequence::{OrderedProgramCursor, execute_observed_replacement_cursor_with_outputs};
pub(crate) use simultaneous::{
    OriginalTriggerObservation, complete_prepared_original_with_outputs,
    execute_simultaneous_originals, execute_simultaneous_originals_with_default_outputs,
    execute_simultaneous_originals_with_outputs,
};
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
pub use player_option_choice::{
    ChoosePlayerOptionEffect, ControlVotesThisTurnEffect, PlayerOptionChooser,
    player_option_choice_tag,
};
pub use vote::{
    VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteOption, VoteResult,
};
pub use with_id::WithIdEffect;

pub(crate) mod selection_relations;
pub(crate) use conditional::prepare_conditional_branch;
pub(crate) use execute_with_source::resolve_source_binding;
pub(crate) use may::is_object_selection;
pub(crate) use tagged::apply_outcome_tags;
pub(crate) use tagging_runtime::{TaggedRuntimeState, capture_tagged_runtime_state};

pub(crate) use if_effect::{
    PreparedIfBranch, execute_if_branches_with_outputs, prepare_if_branches,
};

pub(crate) use for_players::{ForPlayersDrawContinuation, ForPlayersDrawProgress};

mod keyword_action;
pub(crate) use keyword_action::{
    KeywordActionAmount, KeywordActionOutput, execute_keyword_action,
    execute_keyword_action_with_outputs,
};

mod keyword_programs;

pub(crate) use simultaneous::{
    complete_authored_original_subtree_with_outputs, complete_committed_original_with_outputs,
    complete_original_cohort_phase_with_participants,
    complete_retained_original_phase_with_outputs, complete_retained_originals_with_outputs,
    complete_standalone_original_with_outputs, observe_original_completion,
    original_cohort_phase_status_from_receipts, original_cohort_phase_status_with_participants,
    prepare_simultaneous_originals_with_participants, prepare_standalone_completion_with_outputs,
    prepare_standalone_original_completion,
};

#[cfg(test)]
mod resolution_stop_tests;

#[cfg(test)]
mod prepared_port_regressions;

mod held_original;
pub(crate) use held_original::defer_authored_original_additions_with_outputs;
