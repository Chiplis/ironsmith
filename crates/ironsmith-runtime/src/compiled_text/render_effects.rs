use super::*;
use crate::ability::{PresentationKeyword, PresentationLabel};
use crate::effect::SearchSelectionMode;
use crate::filter::StackObjectKind;
use ironsmith_core::{LibraryBottomOrder, PtValue, ValueSurfaceHint, ordinal_word};

#[path = "render_effects/abilities_and_costs.rs"]
mod abilities_and_costs;
#[path = "render_effects/chain_copy.rs"]
mod chain_copy;
#[path = "render_effects/clause_and_ability_surfaces.rs"]
mod clause_and_ability_surfaces;
#[path = "render_effects/continuous_and_choices.rs"]
mod continuous_and_choices;
#[path = "render_effects/costs_and_triggers.rs"]
mod costs_and_triggers;
#[path = "render_effects/effect_lists.rs"]
mod effect_lists;
#[path = "render_effects/emblem_surfaces.rs"]
mod emblem_surfaces;
#[path = "render_effects/looked_partition.rs"]
mod looked_partition;
#[path = "render_effects/looked_top_bottom.rs"]
mod looked_top_bottom;
#[path = "render_effects/observation_conditionals.rs"]
mod observation_conditionals;
#[path = "render_effects/player_and_zone_effects.rs"]
mod player_and_zone_effects;
#[path = "render_effects/quantified_player_actions.rs"]
mod quantified_player_actions;
#[path = "render_effects/returned_object_type_setting.rs"]
mod returned_object_type_setting;
#[path = "render_effects/roll_result_surfaces.rs"]
mod roll_result_surfaces;
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
pub(super) use chain_copy::*;
pub(crate) use clause_and_ability_surfaces::triggered_search_devotion_color;
pub(super) use clause_and_ability_surfaces::*;
pub(super) use continuous_and_choices::*;
pub(super) use costs_and_triggers::*;
pub(super) use effect_lists::*;
pub(super) use effect_lists::{
    describe_face_down_pile_then_manifest, rendered_action_target, target_specs_select_same_objects,
};
pub(super) use emblem_surfaces::*;
pub(super) use looked_partition::*;
pub(super) use looked_top_bottom::*;
pub(super) use observation_conditionals::*;
pub(crate) use player_and_zone_effects::*;
pub(super) use quantified_player_actions::*;
pub(super) use returned_object_type_setting::*;
pub(super) use roll_result_surfaces::*;
pub(super) use search_reveal_and_sacrifice::*;
pub(super) use sequences_and_votes::*;
pub(crate) use single_effects_early::*;
pub(super) use single_effects_late::*;
pub(super) use structural_bundles::*;

pub use sequences_and_votes::compile_effect_list;

fn describe_counted_consult_stop(count: &Value, selection: &str) -> String {
    let plural_selection = pluralize_noun_phrase(strip_leading_article(selection));
    let is_prior_object_count = matches!(
        count.unhinted(),
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query)
            if matches!(
                query.metric,
                crate::effect::EffectMetric::Count
                    | crate::effect::EffectMetric::ChosenCount
                    | crate::effect::EffectMetric::AffectedCount
            )
    );
    if is_prior_object_count {
        format!(
            "a number of {plural_selection} equal to {}",
            describe_value(count)
        )
    } else {
        format!("{} {plural_selection}", describe_value(count))
    }
}

#[cfg(test)]
#[path = "render_effects/tests/mod.rs"]
mod tests;
