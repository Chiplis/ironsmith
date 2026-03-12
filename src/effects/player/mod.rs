//! Player-related effects.
//!
//! This module contains effects that modify player state,
//! such as adding counters (poison, energy, experience),
//! win/lose conditions, turn manipulation, and emblem creation.

mod additional_land_plays;
mod become_monarch;
mod cascade;
mod cast_source;
mod cast_tagged;
mod choose_card_name;
mod control_player;
mod create_emblem;
mod discover;
mod energy_counters;
mod exile_instead_of_graveyard;
mod exile_until_match_cast;
mod exile_until_match_grant_play;
mod experience_counters;
mod extra_turn;
mod extra_turn_after_next_turn;
mod grant;
mod grant_play_from_graveyard;
mod grant_play_tagged;
mod grant_tagged_spell_free_cast_until_end_of_turn;
mod grant_tagged_spell_life_cost_by_mana_value;
mod lose_the_game;
mod may_cast_miracle;
mod pay_energy;
mod poison_counters;
mod runtime_helpers;
mod skip_combat_phases;
mod skip_draw_step;
mod skip_next_combat_phase_this_turn;
mod skip_turn;
mod win_the_game;

pub use additional_land_plays::AdditionalLandPlaysEffect;
pub use become_monarch::BecomeMonarchEffect;
pub use cascade::CascadeEffect;
pub use cast_source::CastSourceEffect;
pub use cast_tagged::CastTaggedEffect;
pub use choose_card_name::ChooseCardNameEffect;
pub use control_player::ControlPlayerEffect;
pub use create_emblem::CreateEmblemEffect;
pub use discover::DiscoverEffect;
pub use energy_counters::EnergyCountersEffect;
pub use exile_instead_of_graveyard::ExileInsteadOfGraveyardEffect;
pub use exile_until_match_cast::ExileUntilMatchCastEffect;
pub use exile_until_match_grant_play::ExileUntilMatchGrantPlayEffect;
pub use experience_counters::ExperienceCountersEffect;
pub use extra_turn::ExtraTurnEffect;
pub use extra_turn_after_next_turn::ExtraTurnAfterNextTurnEffect;
pub use grant::GrantEffect;
pub use grant_play_from_graveyard::GrantPlayFromGraveyardEffect;
pub use grant_play_tagged::{GrantPlayTaggedDuration, GrantPlayTaggedEffect};
pub use grant_tagged_spell_free_cast_until_end_of_turn::GrantTaggedSpellFreeCastUntilEndOfTurnEffect;
pub use grant_tagged_spell_life_cost_by_mana_value::GrantTaggedSpellLifeCostByManaValueEffect;
pub use lose_the_game::LoseTheGameEffect;
pub use may_cast_miracle::MayCastForMiracleCostEffect;
pub use pay_energy::PayEnergyEffect;
pub use poison_counters::PoisonCountersEffect;
pub use skip_combat_phases::SkipCombatPhasesEffect;
pub use skip_draw_step::SkipDrawStepEffect;
pub use skip_next_combat_phase_this_turn::SkipNextCombatPhaseThisTurnEffect;
pub use skip_turn::SkipTurnEffect;
pub use win_the_game::WinTheGameEffect;
