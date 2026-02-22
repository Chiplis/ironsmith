//! Mana-related effects.
//!
//! This module contains effects that add mana to a player's mana pool.

mod add_colorless_mana;
mod add_mana;
mod add_mana_from_commander_color_identity;
mod add_mana_of_any_color;
mod add_mana_of_any_one_color;
mod add_mana_of_chosen_color;
mod add_mana_of_imprinted_colors;
mod add_mana_of_land_produced_types;
mod add_scaled_mana;
mod choice_helpers;
mod grant_mana_ability_until_eot;
mod pay_mana;

pub use add_colorless_mana::AddColorlessManaEffect;
pub use add_mana::AddManaEffect;
pub use add_mana_from_commander_color_identity::AddManaFromCommanderColorIdentityEffect;
pub use add_mana_of_any_color::AddManaOfAnyColorEffect;
pub use add_mana_of_any_one_color::AddManaOfAnyOneColorEffect;
pub use add_mana_of_chosen_color::AddManaOfChosenColorEffect;
pub use add_mana_of_imprinted_colors::AddManaOfImprintedColorsEffect;
pub use add_mana_of_land_produced_types::AddManaOfLandProducedTypesEffect;
pub use add_scaled_mana::AddScaledManaEffect;
pub use grant_mana_ability_until_eot::GrantManaAbilityUntilEotEffect;
pub use pay_mana::PayManaEffect;
