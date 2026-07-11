#[path = "combat_damage_family_shapes/creature_types.rs"]
mod creature_types;
#[path = "combat_damage_family_shapes/for_each.rs"]
mod for_each;
#[path = "combat_damage_family_shapes/returns.rs"]
mod returns;
#[path = "combat_damage_family_shapes/stickers.rs"]
mod stickers;

pub(crate) use creature_types::*;
pub(crate) use for_each::*;
pub(crate) use returns::*;
pub(crate) use stickers::*;
