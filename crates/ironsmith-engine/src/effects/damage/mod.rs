//! Damage-related effects.
//!
//! This module contains effect implementations for dealing damage:
//! - `DealDamageEffect` - Deal damage to a creature, planeswalker, or player
//! - `ClearDamageEffect` - Clear all damage from a creature

mod clear_damage;
mod deal_damage;
mod deal_distributed_damage;
mod heal_damage;
mod prevent_next_time_damage;
mod redirect_next_damage_to_target;
mod redirect_next_time_damage_to_source;
mod replace_next_damage_to_target;

pub use clear_damage::ClearDamageEffect;
pub use deal_damage::DealDamageEffect;
pub use deal_distributed_damage::{DamageDistributionMode, DealDistributedDamageEffect};
pub use heal_damage::HealDamageEffect;
pub use prevent_next_time_damage::{
    PreventNextTimeDamageEffect, PreventNextTimeDamageSource, PreventNextTimeDamageTarget,
};
pub use redirect_next_damage_to_target::{
    RedirectNextDamageDestination, RedirectNextDamageToTargetEffect,
};
pub use redirect_next_time_damage_to_source::{
    RedirectAllDamageThisTurnToTargetEffect, RedirectNextTimeDamageDestination,
    RedirectNextTimeDamageSource, RedirectNextTimeDamageToSourceEffect,
};
pub use replace_next_damage_to_target::ReplaceNextDamageToTargetEffect;
