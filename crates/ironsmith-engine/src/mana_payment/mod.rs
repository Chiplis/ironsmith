//! Server-authoritative mana payment planning.
//!
//! The planner is the shared source of truth for affordability, previews, and
//! execution.  A UI may constrain which resources it wants to use, but it
//! never supplies executable payment steps directly.

mod analytic;
mod color_reachability;
mod event_program;
mod plan;
mod planner;
pub mod program;
mod replacement_program;
pub(crate) mod resources;
mod sources;
mod witness;
pub use replacement_program::ReplacementDecision as ManaReplacementDecision;
pub(crate) use witness::WitnessDecisionMaker;
pub(crate) use witness::replay_replacements as replay_mana_replacements;
pub use witness::{
    ManaChoicePurpose, ManaProductionChoice, ManaProductionWitness, ManaReplacementWitness,
};

pub use plan::*;
pub use planner::{
    ManaPaymentAnalysis, ManaPaymentPerfMetrics, ManaPaymentPlanner, check_mana_payment,
    execute_mana_payment_plan, immediate_manual_mana_abilities_checked, last_mana_payment_perf,
    mana_payment_activation_inventory, mana_payment_activation_inventory_checked,
    mana_payment_expanded_pips, mana_payment_life_options, mana_payment_ready_activation_inventory,
    mana_payment_ready_activation_inventory_checked, mana_payment_ready_and_manual_inventory,
    mana_payment_ready_and_manual_inventory_checked, mana_payment_source_inventory,
    mana_payment_transaction_id, plan_first_mana_payment, plan_mana_payment,
    plan_prompt_mana_payment, unfunded_mana_payment_plan,
};
pub(crate) use planner::{
    execute_mana_payment_plan_in_context, execute_mana_payment_plan_in_context_with_outputs,
};

mod waterbend;
pub(crate) use waterbend::{
    maximum_waterbend_x, record_waterbend_payment, record_waterbend_payment_with_outputs,
    validate_waterbend_scope, validate_waterbend_taps, waterbend_sources,
};

mod interactive;
pub(crate) use interactive::activate_mana_during_payment_with_outputs;
pub(crate) use interactive::{
    activate_mana_during_payment, pay_mana_interactively, pay_mana_interactively_in_context,
    pay_mana_interactively_in_context_with_outputs,
};
pub use interactive::{manual_mana_abilities, manual_mana_abilities_checked};

pub(crate) use sources::{has_mana_modifying_replacements, has_potential_mana_triggers};

/// Native payment acknowledgement is separate from replacement-altered action counts.
pub(crate) struct CompletedManaPayment {
    pub status: ManaPaymentExecution,
    pub outputs: Vec<crate::effects::CompletedEffectOutputs>,
}

impl CompletedManaPayment {
    fn pending() -> Self {
        Self {
            status: ManaPaymentExecution::PendingDecision,
            outputs: Vec::new(),
        }
    }
}

/// Publish the actual activation notification. Manual and planned callers retain
/// their different trigger-drain boundaries and pre-cost snapshot policies.
fn publish_mana_activation_with_outputs(
    game: &mut crate::GameState,
    activation: crate::events::AbilityActivatedEvent,
) -> crate::effects::CompletedEffectOutputs {
    let outputs = game.complete_activation_notification(activation);
    for event in &outputs.outcome.events {
        game.queue_trigger_event(event.provenance(), event.clone());
    }
    outputs
}
