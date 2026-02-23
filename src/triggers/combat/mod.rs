//! Combat triggers.
//!
//! This module contains triggers related to combat, including attack,
//! block, and damage triggers.

mod attacks;
mod attacks_and_isnt_blocked;
mod attacks_alone;
mod attacks_while_saddled;
mod attacks_you;
mod becomes_blocked;
mod blocks;
mod blocks_or_becomes_blocked;
mod deals_combat_damage_to_player;
mod deals_damage;
mod this_attacks;
mod this_attacks_and_isnt_blocked;
mod this_attacks_with_n_others;
mod this_attacks_while_saddled;
mod this_becomes_blocked;
mod this_blocks;
mod this_blocks_object;
mod this_deals_combat_damage_to_player;
mod this_deals_damage;
mod this_deals_damage_to;

pub use attacks::AttacksTrigger;
pub use attacks_and_isnt_blocked::AttacksAndIsntBlockedTrigger;
pub use attacks_alone::AttacksAloneTrigger;
pub use attacks_while_saddled::AttacksWhileSaddledTrigger;
pub use attacks_you::AttacksYouTrigger;
pub use becomes_blocked::BecomesBlockedTrigger;
pub use blocks::BlocksTrigger;
pub use blocks_or_becomes_blocked::BlocksOrBecomesBlockedTrigger;
pub use deals_combat_damage_to_player::DealsCombatDamageToPlayerTrigger;
pub use deals_damage::DealsDamageTrigger;
pub use this_attacks::ThisAttacksTrigger;
pub use this_attacks_and_isnt_blocked::ThisAttacksAndIsntBlockedTrigger;
pub use this_attacks_with_n_others::ThisAttacksWithNOthersTrigger;
pub use this_attacks_while_saddled::ThisAttacksWhileSaddledTrigger;
pub use this_becomes_blocked::ThisBecomesBlockedTrigger;
pub use this_blocks::ThisBlocksTrigger;
pub use this_blocks_object::ThisBlocksObjectTrigger;
pub use this_deals_combat_damage_to_player::ThisDealsCombatDamageToPlayerTrigger;
pub use this_deals_damage::ThisDealsDamageTrigger;
pub use this_deals_damage_to::ThisDealsDamageToTrigger;
