#[path = "combat_damage_family_shapes/creature_types.rs"]
mod creature_types;
#[path = "combat_damage_family_shapes/for_each.rs"]
mod for_each;
#[path = "combat_damage_family_shapes/returns.rs"]
mod returns;
#[path = "combat_damage_family_shapes/stickers.rs"]
mod stickers;

pub use creature_types::*;
pub use for_each::*;
pub use returns::*;
pub use stickers::*;
