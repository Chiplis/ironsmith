//! Server-authoritative mana payment planning.
//!
//! The planner is the shared source of truth for affordability, previews, and
//! execution.  A UI may constrain which resources it wants to use, but it
//! never supplies executable payment steps directly.

mod analytic;
mod plan;
mod planner;
mod sources;
mod color_reachability;
pub(crate) mod resources;
pub mod program;
mod event_program;
mod replacement_program;
mod witness;
pub use replacement_program::ReplacementDecision as ManaReplacementDecision;
pub use witness::{ManaChoicePurpose, ManaReplacementWitness, ManaProductionChoice, ManaProductionWitness};
pub(crate) use witness::WitnessDecisionMaker;
pub(crate) use witness::replay_replacements as replay_mana_replacements;

pub use plan::*;
pub(crate) use planner::execute_mana_payment_plan_in_context;
pub use planner::{
    ManaPaymentAnalysis, ManaPaymentPerfMetrics, ManaPaymentPlanner, check_mana_payment,
    execute_mana_payment_plan, last_mana_payment_perf, mana_payment_activation_inventory,
    mana_payment_expanded_pips, mana_payment_life_options, mana_payment_ready_activation_inventory,
    mana_payment_ready_and_manual_inventory,
    mana_payment_activation_inventory_checked, mana_payment_ready_activation_inventory_checked,
    mana_payment_ready_and_manual_inventory_checked,
    mana_payment_source_inventory, mana_payment_transaction_id, plan_first_mana_payment,
    plan_mana_payment, unfunded_mana_payment_plan,
};

mod interactive;
pub use interactive::{manual_mana_abilities, manual_mana_abilities_checked};
pub(crate) use interactive::{activate_mana_during_payment, pay_mana_interactively, pay_mana_interactively_in_context};

pub(crate) use sources::{has_potential_mana_triggers, has_mana_modifying_replacements};
