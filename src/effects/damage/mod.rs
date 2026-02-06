//! Damage-related effects.
//!
//! This module contains effect implementations for dealing damage:
//! - `DealDamageEffect` - Deal damage to a creature, planeswalker, or player
//! - `ClearDamageEffect` - Clear all damage from a creature

mod clear_damage;
mod deal_damage;

pub use clear_damage::ClearDamageEffect;
pub use deal_damage::DealDamageEffect;
