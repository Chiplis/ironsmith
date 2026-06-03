//! Life and damage triggers.

mod is_dealt_damage;
mod player_loses_game;
mod player_loses_life;
mod player_lost_game;
mod you_gain_life;
mod you_lose_life;

pub use is_dealt_damage::IsDealtDamageTrigger;
pub use player_loses_game::PlayerLosesGameTrigger;
pub use player_loses_life::PlayerLosesLifeTrigger;
pub use player_lost_game::PlayerLostGameTrigger;
pub use you_gain_life::YouGainLifeTrigger;
pub use you_lose_life::YouLoseLifeTrigger;
