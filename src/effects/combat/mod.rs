//! Combat and power/toughness effects.
//!
//! This module contains effects related to combat and creature stats:
//! - `FightEffect` - Two creatures deal damage to each other
//! - `ModifyPowerToughnessEffect` - Modify a single creature's P/T
//! - `ModifyPowerToughnessAllEffect` - Modify all creatures matching a filter
//! - `ModifyPowerToughnessForEachEffect` - Modify based on a count
//! - `PreventDamageEffect` - Prevent N damage to a target
//! - `PreventAllDamageEffect` - Prevent all damage
//! - `GrantAbilitiesAllEffect` - Grant abilities to all creatures matching a filter

mod enter_attacking;
mod fight;
mod grant_abilities_all;
mod grant_abilities_target;
mod modify_power_toughness;
mod modify_power_toughness_all;
mod modify_power_toughness_for_each;
mod prevent_all_damage;
mod prevent_damage;

pub use enter_attacking::EnterAttackingEffect;
pub use fight::FightEffect;
pub use grant_abilities_all::GrantAbilitiesAllEffect;
pub use grant_abilities_target::GrantAbilitiesTargetEffect;
pub use modify_power_toughness::ModifyPowerToughnessEffect;
pub use modify_power_toughness_all::ModifyPowerToughnessAllEffect;
pub use modify_power_toughness_for_each::ModifyPowerToughnessForEachEffect;
pub use prevent_all_damage::PreventAllDamageEffect;
pub use prevent_damage::PreventDamageEffect;
