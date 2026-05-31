//! Damage events and matchers.

mod damage_event;
mod damage_prevented_event;
pub mod matchers;

pub use damage_event::DamageEvent;
pub use damage_prevented_event::DamagePreventedEvent;
