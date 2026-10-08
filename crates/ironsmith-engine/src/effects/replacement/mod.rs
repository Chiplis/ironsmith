//! Replacement effect helpers.

mod apply_replacement;
mod register_counter_placement_replacement;
mod register_damaged_by_source_zone_replacement;
mod register_draw_replacement;
mod register_enter_tapped;
mod register_future_zone_replacement;
mod register_mana_replacement;
mod register_mana_rewrite;
mod register_mana_spend_permission;
mod register_next_batch_enter_with_counters;
mod register_zone_replacement;
mod move_replaced_object_to_library;
pub use move_replaced_object_to_library::MoveReplacedObjectToLibraryEffect;

pub use apply_replacement::{ApplyReplacementEffect, ReplacementApplyMode};
pub use register_counter_placement_replacement::RegisterCounterPlacementReplacementEffect;
pub use register_damaged_by_source_zone_replacement::RegisterDamagedBySourceZoneReplacementEffect;
pub use register_draw_replacement::RegisterDrawReplacementEffect;
pub use register_enter_tapped::RegisterEnterTappedReplacementEffect;
pub use register_enter_under_control::RegisterEnterUnderControlReplacementEffect;
pub use register_future_zone_replacement::RegisterFutureZoneReplacementEffect;
pub use register_mana_replacement::RegisterManaReplacementEffect;
pub use register_mana_rewrite::RegisterManaRewriteEffect;
pub use register_mana_spend_permission::RegisterManaSpendPermissionEffect;
pub use register_next_batch_enter_with_counters::RegisterNextBatchEnterWithCountersEffect;
pub use register_zone_replacement::RegisterZoneReplacementEffect;

mod register_enter_under_control;
pub(crate) use register_zone_replacement::zone_replacement_action;

mod register_enter_with_counters;
pub use register_enter_with_counters::RegisterEnterWithCountersReplacementEffect;

mod execute_payload;
pub(crate) use execute_payload::{
    PreparedReplacementOriginal, capture_replacement_source_snapshot,
    commit_replacement_original_with_outputs,
};
pub(crate) use execute_payload::{
    execute_deferred_replacement_programs, execute_event_expansion,
    execute_event_expansion_with_targets, execute_replacement_payload,
    execute_replacement_payload_with_object_tags, execute_replacement_payload_with_snapshot,
};

pub(crate) use execute_payload::{
    CompletedReplacementPrograms, ReplacementProgramBindings,
    complete_bound_replacement_programs_with_outputs, complete_deferred_replacement_programs,
    complete_deferred_replacement_programs_with_bindings,
    complete_deferred_replacement_programs_with_targets,
    complete_replacement_programs_with_original_outputs,
    execute_deferred_replacement_programs_with_bindings, execute_event_expansion_with_outputs,
    execute_replacement_original_payload_with_outputs, execute_replacement_payload_with_outputs,
    project_replacement_original_outputs,
};

mod register_damage_addition;
mod register_damage_multiplier;
pub use register_damage_addition::RegisterDamageAdditionEffect;
pub use register_damage_multiplier::RegisterDamageMultiplierEffect;

mod draw_continuation;
pub(crate) use draw_continuation::prepare_draw_continuation_with_outputs;

pub(crate) use draw_continuation::{
    PreparedReplacementChild, ReplacementResume, prepare_committed_draw_boundary,
    prepare_native_draw_continuation_with_outputs,
    prepare_native_proposal_draw_continuation_with_outputs, prepare_replacement_child,
    prepare_scoped_program_draw_boundary_with_outputs, replacement_effect_contains_draw,
    replacement_effect_supported, resume_replacement_child_with_outputs,
};

pub(crate) use draw_continuation::{
    prepare_draw_continuation_with_bindings_and_outputs,
    prepare_draw_continuation_with_original_and_outputs,
};

pub(crate) use execute_payload::with_replacement_child;

pub(crate) use draw_continuation::{
    prepare_scoped_draw_continuation, prepare_scoped_draw_continuation_with_outputs,
};

mod zone_draw_tail;
pub(crate) use zone_draw_tail::{prepare_zone_draw_tail, prepare_zone_draw_tail_with_outputs};

mod deferred_programs;
pub(crate) use deferred_programs::defer_replacement_programs_with_outputs;
