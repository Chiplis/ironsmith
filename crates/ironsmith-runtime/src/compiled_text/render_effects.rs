use super::*;
use crate::ability::{PresentationKeyword, PresentationLabel};
use crate::effect::SearchSelectionMode;
use crate::filter::StackObjectKind;
use ironsmith_core::{LibraryBottomOrder, PtValue, ValueSurfaceHint, ordinal_word};

#[path = "render_effects/abilities_and_costs.rs"]
mod abilities_and_costs;
#[path = "render_effects/clause_and_ability_surfaces.rs"]
mod clause_and_ability_surfaces;
#[path = "render_effects/continuous_and_choices.rs"]
mod continuous_and_choices;
#[path = "render_effects/costs_and_triggers.rs"]
mod costs_and_triggers;
#[path = "render_effects/effect_lists.rs"]
mod effect_lists;
#[path = "render_effects/player_and_zone_effects.rs"]
mod player_and_zone_effects;
#[path = "render_effects/search_reveal_and_sacrifice.rs"]
mod search_reveal_and_sacrifice;
#[path = "render_effects/sequences_and_votes.rs"]
mod sequences_and_votes;
#[path = "render_effects/single_effects_early.rs"]
mod single_effects_early;
#[path = "render_effects/single_effects_late.rs"]
mod single_effects_late;
#[path = "render_effects/structural_bundles.rs"]
mod structural_bundles;

pub(super) use abilities_and_costs::*;
pub(super) use clause_and_ability_surfaces::*;
pub(super) use continuous_and_choices::*;
pub(super) use costs_and_triggers::*;
pub(super) use effect_lists::*;
pub(super) use player_and_zone_effects::*;
pub(super) use search_reveal_and_sacrifice::*;
pub(super) use sequences_and_votes::*;
pub(super) use single_effects_early::*;
pub(super) use single_effects_late::*;
pub(super) use structural_bundles::*;

pub use sequences_and_votes::compile_effect_list;

#[cfg(test)]
#[path = "render_effects/tests/mod.rs"]
mod tests;
