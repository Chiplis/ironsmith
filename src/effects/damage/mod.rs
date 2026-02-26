//! Damage-related effects.
//!
//! This module contains effect implementations for dealing damage:
//! - `DealDamageEffect` - Deal damage to a creature, planeswalker, or player
//! - `ClearDamageEffect` - Clear all damage from a creature

mod clear_damage;
mod deal_damage;
mod prevent_next_time_damage;
mod redirect_next_damage_to_target;
mod redirect_next_time_damage_to_source;

pub use clear_damage::ClearDamageEffect;
pub use deal_damage::DealDamageEffect;
pub use prevent_next_time_damage::{
    PreventNextTimeDamageEffect, PreventNextTimeDamageSource, PreventNextTimeDamageTarget,
};
pub use redirect_next_damage_to_target::RedirectNextDamageToTargetEffect;
pub use redirect_next_time_damage_to_source::{
    RedirectNextTimeDamageSource, RedirectNextTimeDamageToSourceEffect,
};
