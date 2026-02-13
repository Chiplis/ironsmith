//! Player-related effects.
//!
//! This module contains effects that modify player state,
//! such as adding counters (poison, energy, experience),
//! win/lose conditions, turn manipulation, and emblem creation.

mod control_player;
mod create_emblem;
mod energy_counters;
mod exile_instead_of_graveyard;
mod experience_counters;
mod extra_turn;
mod grant;
mod grant_play_from_graveyard;
mod lose_the_game;
mod may_cast_miracle;
mod poison_counters;
mod skip_combat_phases;
mod skip_draw_step;
mod skip_next_combat_phase_this_turn;
mod skip_turn;
mod win_the_game;

pub use control_player::ControlPlayerEffect;
pub use create_emblem::CreateEmblemEffect;
pub use energy_counters::EnergyCountersEffect;
pub use exile_instead_of_graveyard::ExileInsteadOfGraveyardEffect;
pub use experience_counters::ExperienceCountersEffect;
pub use extra_turn::ExtraTurnEffect;
pub use grant::GrantEffect;
pub use grant_play_from_graveyard::GrantPlayFromGraveyardEffect;
pub use lose_the_game::LoseTheGameEffect;
pub use may_cast_miracle::MayCastForMiracleCostEffect;
pub use poison_counters::PoisonCountersEffect;
pub use skip_combat_phases::SkipCombatPhasesEffect;
pub use skip_draw_step::SkipDrawStepEffect;
pub use skip_next_combat_phase_this_turn::SkipNextCombatPhaseThisTurnEffect;
pub use skip_turn::SkipTurnEffect;
pub use win_the_game::WinTheGameEffect;
