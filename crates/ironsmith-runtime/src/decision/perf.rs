use super::*;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize))]
pub struct ComputeLegalActionsPerfMetrics {
    pub derived_view_ms: f64,
    pub prewarm_ms: f64,
    pub cast_context_ms: f64,
    pub battlefield_ability_context_ms: f64,
    pub active_grant_zone_checks_ms: f64,
    pub hand_summary_ms: f64,
    pub controlled_battlefield_ms: f64,
    pub lands_ms: f64,
    pub hand_casts_ms: f64,
    pub can_cast_spell_with_view_ms: f64,
    pub spell_has_legal_targets_ms: f64,
    pub compute_potential_mana_with_view_ms: f64,
    pub hand_casts_timing_ms: f64,
    pub hand_casts_restrictions_ms: f64,
    pub hand_casts_target_legality_ms: f64,
    pub hand_casts_cost_adjustment_ms: f64,
    pub hand_casts_affordability_ms: f64,
    pub hand_special_actions_ms: f64,
    pub graveyard_casts_ms: f64,
    pub exile_casts_ms: f64,
    pub hand_alternatives_ms: f64,
    pub battlefield_abilities_ms: f64,
    pub can_activate_ability_with_restrictions_with_view_ms: f64,
    pub battlefield_ability_precheck_ms: f64,
    pub battlefield_ability_target_legality_ms: f64,
    pub battlefield_ability_cost_build_ms: f64,
    pub battlefield_ability_affordability_ms: f64,
    pub non_battlefield_abilities_ms: f64,
    pub total_ms: f64,
    pub action_count: usize,
}

thread_local! {
    static LAST_COMPUTE_LEGAL_ACTIONS_PERF: RefCell<Option<ComputeLegalActionsPerfMetrics>> =
        const { RefCell::new(None) };
}

pub(crate) fn store_compute_legal_actions_perf(metrics: ComputeLegalActionsPerfMetrics) {
    LAST_COMPUTE_LEGAL_ACTIONS_PERF.with(|slot| {
        *slot.borrow_mut() = Some(metrics);
    });
}

pub fn last_compute_legal_actions_perf() -> Option<ComputeLegalActionsPerfMetrics> {
    LAST_COMPUTE_LEGAL_ACTIONS_PERF.with(|slot| slot.borrow().clone())
}
