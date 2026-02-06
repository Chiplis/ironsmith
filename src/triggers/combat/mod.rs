//! Combat triggers.
//!
//! This module contains triggers related to combat, including attack,
//! block, and damage triggers.

mod attacks;
mod attacks_alone;
mod attacks_you;
mod becomes_blocked;
mod blocks;
mod blocks_or_becomes_blocked;
mod deals_combat_damage_to_player;
mod deals_damage;
mod this_attacks;
mod this_becomes_blocked;
mod this_blocks;
mod this_deals_combat_damage_to_player;
mod this_deals_damage;

pub use attacks::AttacksTrigger;
pub use attacks_alone::AttacksAloneTrigger;
pub use attacks_you::AttacksYouTrigger;
pub use becomes_blocked::BecomesBlockedTrigger;
pub use blocks::BlocksTrigger;
pub use blocks_or_becomes_blocked::BlocksOrBecomesBlockedTrigger;
pub use deals_combat_damage_to_player::DealsCombatDamageToPlayerTrigger;
pub use deals_damage::DealsDamageTrigger;
pub use this_attacks::ThisAttacksTrigger;
pub use this_becomes_blocked::ThisBecomesBlockedTrigger;
pub use this_blocks::ThisBlocksTrigger;
pub use this_deals_combat_damage_to_player::ThisDealsCombatDamageToPlayerTrigger;
pub use this_deals_damage::ThisDealsDamageTrigger;
