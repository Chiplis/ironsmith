//! Damage events and matchers.

mod amounts;
pub(crate) use amounts::{
    checked_damage_amount, checked_damage_count, checked_scalar_count,
    validate_damage_history_amounts,
};
mod damage_event;
mod damage_prevented_event;
mod receipt_amounts;
pub(crate) use receipt_amounts::bind_received_damage_amounts;
pub mod matchers;

pub use damage_event::DamageEvent;
pub use damage_prevented_event::{DamagePreventedEvent, PreventedDamage};
