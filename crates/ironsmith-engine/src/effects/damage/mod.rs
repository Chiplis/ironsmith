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
pub(crate) mod redirect_next_damage_to_target;
pub(crate) mod redirect_next_time_damage_to_source;
pub(crate) mod replace_next_damage_to_target;

pub use clear_damage::ClearDamageEffect;
pub(crate) use deal_damage::finish_damage_replacement_programs;
pub use deal_damage::{DealDamageEffect, DealDamageToRecipientsEffect};
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

mod multi_source_damage;
pub use multi_source_damage::DealDamageBySourcesEffect;
pub(crate) use multi_source_damage::commit_damage_batch;

mod deal_damage_each;
pub use deal_damage_each::DealDamageEachEffect;
