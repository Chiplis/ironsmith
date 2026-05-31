//! Life and damage triggers.

mod damage_prevented;
mod is_dealt_damage;
mod player_loses_life;
mod you_gain_life;
mod you_lose_life;

pub use damage_prevented::DamagePreventedTrigger;
pub use is_dealt_damage::IsDealtDamageTrigger;
pub use player_loses_life::PlayerLosesLifeTrigger;
pub use you_gain_life::YouGainLifeTrigger;
pub use you_lose_life::YouLoseLifeTrigger;
