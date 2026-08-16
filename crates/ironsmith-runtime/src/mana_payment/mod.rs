//! Server-authoritative mana payment planning.
//!
//! The planner is the shared source of truth for affordability, previews, and
//! execution.  A UI may constrain which resources it wants to use, but it
//! never supplies executable payment steps directly.

mod plan;
mod planner;

pub use plan::*;
pub use planner::{
    ManaPaymentPlanner, execute_mana_payment_plan, mana_payment_activation_inventory,
    mana_payment_source_inventory, plan_first_mana_payment, plan_mana_payment,
};
