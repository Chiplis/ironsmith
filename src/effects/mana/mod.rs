//! Mana-related effects.
//!
//! This module contains effects that add mana to a player's mana pool.

mod add_colorless_mana;
mod add_mana;
mod add_mana_from_commander_color_identity;
mod add_mana_of_any_color;
mod add_mana_of_any_one_color;
mod add_mana_of_imprinted_colors;

pub use add_colorless_mana::AddColorlessManaEffect;
pub use add_mana::AddManaEffect;
pub use add_mana_from_commander_color_identity::AddManaFromCommanderColorIdentityEffect;
pub use add_mana_of_any_color::AddManaOfAnyColorEffect;
pub use add_mana_of_any_one_color::AddManaOfAnyOneColorEffect;
pub use add_mana_of_imprinted_colors::AddManaOfImprintedColorsEffect;
