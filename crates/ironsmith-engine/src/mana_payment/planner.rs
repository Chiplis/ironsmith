use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

use crate::ability::{AbilityKind, ActivatedAbilityRuntimeExt as _};
use crate::color::Color;
use crate::decision::SelectFirstDecisionMaker;
use crate::derived_view::DerivedGameView;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::mana::ManaSymbol;
use crate::player::ManaPool;

use super::{
    ManaPaymentActivationOption, ManaPaymentFailure, ManaPaymentPlan, ManaPaymentRequest,
    ManaPaymentScore, ManaPaymentSourceKind, ManaPaymentWarning, ManaPipId, PlannedManaActivation,
    PlannedPipAllocation, RequiredManaActivation,
};

const MAX_SEARCH_NODES: usize = 4_096;
const MAX_EXTRA_ACTIVATIONS: usize = 8;
const MAX_PLANS_PER_SELECTION: usize = 16;
const MAX_TOTAL_PLANS: usize = 32;

/// Diagnostic counters for the most recent `plan_mana_payment` call.
///
/// These counters are intentionally observational: they do not change search
/// ordering or legality.  The WASM adapter exposes them so a slow priority
/// action can be correlated with planner work in a real match.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct ManaPaymentPerfMetrics {
    pub visited_nodes: usize,
    pub search_limited: bool,
    pub plans_returned: usize,
    /// Selections answered by the clone-free assignment instead of the search.
    pub analytic_selections: usize,
    /// Selections that fell back to the cloning search.
    pub searched_selections: usize,
}

thread_local! {
    static LAST_MANA_PAYMENT_PERF: RefCell<ManaPaymentPerfMetrics> =
        const {
            RefCell::new(ManaPaymentPerfMetrics {
                visited_nodes: 0,
                search_limited: false,
                plans_returned: 0,
                analytic_selections: 0,
                searched_selections: 0,
            })
        };
}

pub fn last_mana_payment_perf() -> ManaPaymentPerfMetrics {
    LAST_MANA_PAYMENT_PERF.with(|slot| *slot.borrow())
}

/// Individually selectable life alternatives, identified in the expanded cost.
pub fn mana_payment_life_options(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<(ManaPipId, u32)> {
    if !request.allow_life_payment || request.preferences.prefer_life {
        return Vec::new();
    }
    let black_life = request.allow_black_life
        && game.player_can_pay_black_with_life_for_reason(
            request.payer,
            Some(request.source),
            request.reason,
        );
    GameState::expanded_payment_pips(&request.cost, request.x_value, black_life)
        .iter()
        .enumerate()
        .filter_map(|(index, pip)| {
            let id = ManaPipId(index as u32);
            if request.preferences.required_life_pips.contains(&id) {
                return None;
            }
            pip.iter().find_map(|symbol| match symbol {
                ManaSymbol::Life(amount) => Some((id, u32::from(*amount))),
                _ => None,
            })
        })
        .collect()
}

/// Stateless entry point used by legality, runtime, and UI snapshot code.
pub fn plan_mana_payment(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<Vec<ManaPaymentPlan>, ManaPaymentFailure> {
    let mut planner = ManaPaymentPlanner::default();
    let result = planner.plan_internal(game, request, false);
    LAST_MANA_PAYMENT_PERF.with(|slot| {
        *slot.borrow_mut() = ManaPaymentPerfMetrics {
            visited_nodes: planner.visited_nodes,
            search_limited: matches!(&result, Err(ManaPaymentFailure::SearchLimitReached)),
            plans_returned: result.as_ref().map_or(0, Vec::len),
            analytic_selections: planner.analytic_selections,
            searched_selections: planner.searched_selections,
        };
    });
    result
}

/// Return the first legal proposal discovered by the planner's ordered search.
///
/// This is intended for latency-sensitive previews. Callers that need the
/// best bounded-search result should continue to use [`plan_mana_payment`].
pub fn plan_first_mana_payment(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<ManaPaymentPlan, ManaPaymentFailure> {
    ManaPaymentPlanner {
        lazy_candidates: true,
        preview_assignment: true,
        ..Default::default()
    }
    .first_plan(game, request)
}

/// Check for one valid payment without ranking plans for display or execution.
/// Unlike previews, existence checks skip assignment setup and follow one
/// candidate line immediately. Both stop at the first legal completion.
pub fn check_mana_payment(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<(), ManaPaymentFailure> {
    ManaPaymentPlanner {
        lazy_candidates: true,
        ..Default::default()
    }
    .first_plan(game, request)
    .map(|_| ())
}

/// Legal source-level controls the client may use to constrain replanning.
pub fn mana_payment_source_inventory(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<super::ManaPaymentSourceOption> {
    let mut unconstrained = request.clone();
    unconstrained.preferences.excluded_sources.clear();
    let mut by_source =
        std::collections::BTreeMap::<ObjectId, Vec<super::ManaPaymentSourceKind>>::new();
    for choice in collect_activation_choices(game, &unconstrained) {
        let kinds = by_source.entry(choice.source).or_default();
        if !kinds.contains(&super::ManaPaymentSourceKind::ManaAbility) {
            kinds.push(super::ManaPaymentSourceKind::ManaAbility);
        }
    }
    if request.reason == crate::costs::PaymentReason::CastSpell
        && let Some(spell) = game.object(request.source)
        && game.controller_of(spell) == request.payer
    {
        if crate::decision::has_convoke(spell) {
            for (source, _) in crate::decision::get_convoke_creatures(game, request.payer) {
                if request.reserved_tap_sources.contains(&source) {
                    continue;
                }
                let kinds = by_source.entry(source).or_default();
                if !kinds.contains(&super::ManaPaymentSourceKind::Convoke) {
                    kinds.push(super::ManaPaymentSourceKind::Convoke);
                }
            }
        }
        if crate::decision::has_delve(spell) {
            for source in delve_cards(game, request) {
                by_source
                    .entry(source)
                    .or_default()
                    .push(ManaPaymentSourceKind::Delve);
            }
        }
        if crate::decision::has_improvise(spell) {
            for source in crate::decision::get_improvise_artifacts(game, request.payer) {
                if request.reserved_tap_sources.contains(&source) {
                    continue;
                }
                let kinds = by_source.entry(source).or_default();
                if !kinds.contains(&super::ManaPaymentSourceKind::Improvise) {
                    kinds.push(super::ManaPaymentSourceKind::Improvise);
                }
            }
        }
    }
    by_source
        .into_iter()
        .map(|(source, mut kinds)| {
            kinds.sort_unstable();
            super::ManaPaymentSourceOption { source, kinds }
        })
        .collect()
}

/// Exact engine-authorized activation actions available to an incremental
/// payment client. The game is only simulated here; no live state is mutated.
pub fn mana_payment_activation_inventory(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<ManaPaymentActivationOption> {
    let mut unconstrained = request.clone();
    unconstrained.preferences.excluded_sources.clear();
    collect_activation_choices(game, &unconstrained)
        .into_iter()
        .filter_map(|choice| {
            let mut staged = game.clone();
            let before = staged
                .player(unconstrained.payer)
                .map(|player| player.mana_pool.clone())
                .unwrap_or_default();
            let mut decision_maker = SelectFirstDecisionMaker;
            crate::special_actions::perform_activate_mana_ability_restricted_colors(
                &mut staged,
                unconstrained.payer,
                choice.source,
                choice.ability_index,
                choice.color_restriction.clone(),
                &mut decision_maker,
            )
            .ok()?;
            let after = staged
                .player(unconstrained.payer)
                .map(|player| player.mana_pool.clone())
                .unwrap_or_default();
            let mut repeat_staged = staged.clone();
            let mut repeat_decision_maker = SelectFirstDecisionMaker;
            let repeatable =
                crate::special_actions::perform_activate_mana_ability_restricted_colors(
                    &mut repeat_staged,
                    unconstrained.payer,
                    choice.source,
                    choice.ability_index,
                    choice.color_restriction.clone(),
                    &mut repeat_decision_maker,
                )
                .is_ok()
                    && repeat_staged
                        .player(unconstrained.payer)
                        .is_some_and(|player| player.mana_pool != after);
            (after != before).then(|| ManaPaymentActivationOption {
                source: choice.source,
                ability_index: choice.ability_index,
                color_restriction: choice.color_restriction,
                expected_mana: positive_pool_delta(&before, &after),
                repeatable,
            })
        })
        .collect()
}

/// Only offer abilities whose activation increases coverage of the current cost.
/// Simulating the ordinary activation preserves production and spending restrictions,
/// replacement effects, and the mana consumed by filters.
pub(super) fn useful_manual_mana_abilities(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<(ObjectId, usize)> {
    if game
        .preview_mana_cost_payment_with_options(
            request.payer,
            Some(request.source),
            &request.cost,
            request.x_value,
            request.reason,
            &request.spend_policy,
            false,
            false,
            false,
        )
        .is_some()
    {
        return Vec::new();
    }
    let before = game.covered_mana_payment_pips(request);
    let mut result = Vec::new();
    let mut unconstrained = request.clone();
    if request.reason != crate::costs::PaymentReason::ActivateManaAbility {
        unconstrained.preferences.excluded_sources.clear();
    }
    for choice in collect_activation_choices(game, &unconstrained) {
        let key = (choice.source, choice.ability_index);
        if result.contains(&key) {
            continue;
        }
        let mut staged = game.clone();
        let mut exclusions = unconstrained.preferences.excluded_sources.clone();
        exclusions.push(choice.source);
        if crate::special_actions::perform_mana_ability_with_payment_mode(
            &mut staged,
            request.payer,
            choice.source,
            choice.ability_index,
            choice.color_restriction,
            Some(exclusions),
            &mut SelectFirstDecisionMaker,
        )
        .is_ok()
            && staged.covered_mana_payment_pips(request) > before
        {
            result.push(key);
        }
    }
    result
}

/// Validate and execute a plan outside the priority cast/activation pipeline.
/// The caller's enclosing replay checkpoint remains responsible for surfacing
/// any nested decision made by a complex mana ability.
pub fn execute_mana_payment_plan(
    game: &mut GameState,
    request: &ManaPaymentRequest,
    expected_plan: &ManaPaymentPlan,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<super::ManaPaymentExecution, ManaPaymentFailure> {
    let matches = |plan: &ManaPaymentPlan| {
        plan.id == expected_plan.id && plan.request_hash == expected_plan.request_hash
    };
    let current = match plan_first_mana_payment(game, request) {
        Ok(plan) if matches(&plan) => plan,
        _ => plan_mana_payment(game, request)?
            .into_iter()
            .find(matches)
            .ok_or(ManaPaymentFailure::StalePlan)?,
    };
    let checkpoint = game.clone();
    for step in &current.mana_ability_steps {
        if crate::special_actions::perform_activate_mana_ability_restricted_colors(
            game,
            request.payer,
            step.source,
            step.ability_index,
            step.color_restriction.clone(),
            decision_maker,
        )
        .is_err()
        {
            *game = checkpoint;
            return Err(ManaPaymentFailure::ExecutionFailed);
        }
        if decision_maker.awaiting_choice() {
            *game = checkpoint;
            return Ok(super::ManaPaymentExecution::PendingDecision);
        }
    }
    for allocation in &current.allocations {
        let success = match allocation.payment {
            super::PlannedPipPayment::Convoke(source)
            | super::PlannedPipPayment::Improvise(source) => {
                if game.object(source).is_none() || game.is_tapped(source) {
                    false
                } else {
                    game.tap(source);
                    game.queue_trigger_event(
                        crate::provenance::ProvNodeId::default(),
                        crate::triggers::TriggerEvent::new(
                            crate::events::PermanentTappedEvent::new(source),
                            crate::provenance::ProvNodeId::default(),
                        ),
                    );
                    true
                }
            }
            super::PlannedPipPayment::Delve(source) => {
                if !delve_cards(game, request).contains(&source) {
                    false
                } else {
                    let mut context = crate::costs::CostContext::new(
                        request.source,
                        request.payer,
                        decision_maker,
                    )
                    .with_reason(request.reason)
                    .with_pre_chosen_cards(vec![source]);
                    matches!(
                        crate::costs::Cost::exile_from_graveyard(1, None).pay(game, &mut context),
                        Ok(crate::costs::CostPaymentResult::Paid)
                    )
                }
            }
            _ => true,
        };
        if !success {
            *game = checkpoint;
            return Err(ManaPaymentFailure::ExecutionFailed);
        }
    }
    if !game.try_pay_mana_cost_with_payment_options(
        request.payer,
        Some(request.source),
        &current.mana_cost_after_alternatives,
        request.x_value,
        request.reason,
        &request.spend_policy,
        request.allow_life_payment,
        request.allow_black_life,
        request.preferences.prefer_life,
    ) {
        *game = checkpoint;
        return Err(ManaPaymentFailure::ExecutionFailed);
    }
    for allocation in &current.allocations {
        let (permanent_id, effect, action) = match allocation.payment {
            super::PlannedPipPayment::Convoke(id) => (
                id,
                crate::decision::AlternativePaymentEffect::Convoke,
                crate::events::KeywordActionKind::Convoke,
            ),
            super::PlannedPipPayment::Improvise(id) => (
                id,
                crate::decision::AlternativePaymentEffect::Improvise,
                crate::events::KeywordActionKind::Improvise,
            ),
            _ => continue,
        };
        if let Some(spell) = game.object_mut(request.source) {
            let contribution = crate::decision::KeywordPaymentContribution {
                permanent_id,
                effect,
            };
            if !spell
                .keyword_payment_contributions_to_cast
                .contains(&contribution)
            {
                spell
                    .keyword_payment_contributions_to_cast
                    .push(contribution);
            }
        }
        let provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::KeywordAction);
        game.queue_trigger_event(
            provenance,
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::KeywordActionEvent::new(action, request.payer, request.source, 1),
                provenance,
            ),
        );
    }
    Ok(super::ManaPaymentExecution::Paid)
}

/// Whether every mana ability the solver counted can be used at most once
/// during a single payment.
///
/// The solver tracks sources with a used/unused flag, so it cannot express
/// activating the same ability twice. A tap cost guarantees single use: the
/// permanent is tapped afterwards and cannot pay again. Anything else — a free
/// or sacrifice-costed mana ability — may repeat, and the solver would
/// under-count it, so those boards keep the full search.
fn every_mana_ability_is_single_use(
    game: &GameState,
    request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>,
) -> bool {
    let analysis = view.simple_battlefield_mana_analysis(request.payer);
    for &source in analysis.mana_source_ids() {
        let Some(object) = game.object(source) else {
            continue;
        };
        let abilities = view
            .abilities_rc(source)
            .unwrap_or_else(|| std::sync::Arc::new(object.abilities_vec()));
        for &ability_index in analysis.mana_ability_indices_for(source) {
            let Some(ability) = abilities.get(ability_index) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.has_tap_cost() {
                return false;
            }
        }
    }
    true
}

/// Fast unpayability check run once before the planner's search begins.
///
/// Proving a cost unpayable is the planner's worst case: it expands the whole
/// candidate space, cloning a `GameState` per candidate, so the node cap bounds
/// nodes but not wall time. The solver answers the same question in
/// microseconds. A "yes" from the solver decides nothing and the search runs
/// unchanged; only a "no" short-circuits, and only when the solver could see
/// everything the planner could have spent.
fn affordability_rules_out_payment(game: &GameState, request: &ManaPaymentRequest) -> bool {
    if !affordability_solver_sees_every_resource(game, request) {
        return false;
    }
    remaining_mana_is_unpayable(game, request)
}

// Once keyword resources have been assigned, only the remaining mana cost
// matters. Ignoring reservations overestimates available mana, so a negative
// answer is still a sound veto.
fn remaining_mana_is_unpayable(game: &GameState, request: &ManaPaymentRequest) -> bool {
    if !request.reserved_graveyard_sources.is_empty()
        || !request.reserved_permanent_sources.is_empty()
    {
        return false;
    }
    let view = DerivedGameView::new(game);
    if !every_mana_ability_is_single_use(game, request, &view) {
        return false;
    }
    !crate::decision::can_pay_mana_cost_with_available_sources(
        game,
        request.payer,
        Some(request.source),
        &request.cost,
        request.x_value,
        request.reason,
        &request.spend_policy,
        request.allow_black_life,
        &view,
    )
}

#[derive(Debug, Default)]
pub struct ManaPaymentPlanner {
    /// Test-only escape hatch: runs the full search even when the affordability
    /// solver would veto it, so a differential test can check that the veto
    /// never refuses a payment the search would have found.
    skip_affordability_gate: bool,
    visited_nodes: usize,
    analytic_selections: usize,
    searched_selections: usize,
    lazy_candidates: bool,
    preview_assignment: bool,
    sliced: bool,
    remaining: usize,
    pending: bool,
    outer: Option<PlanningCursor>,
}

#[derive(Debug)]
struct PlanningCursor {
    selections: std::vec::IntoIter<AlternativeSelection>,
    pool_before: ManaPool,
    plans: Vec<ManaPaymentPlan>,
    active: Option<SelectionWork>,
}
#[derive(Debug)]
struct SelectionWork {
    selection: AlternativeSelection,
    request: ManaPaymentRequest,
    search: CandidateSearch,
}

#[derive(Debug, Clone)]
struct SearchStep {
    activation: PlannedManaActivation,
}

/// Whether the affordability solver can see every resource this request could
/// spend, making a "cannot pay" answer from it a sound veto on the planner.
///
/// The solver models the mana pool, snow mana, life payment and every
/// activatable mana ability the planner would consider — its source discovery is
/// the same `simple_battlefield_mana_analysis`, and the planner only narrows it
/// further. What the solver does not model is the keyword payments in CR 601.2h
/// and resources the caller reserved outside the request's cost, so those cases
/// keep the full search.
fn affordability_solver_sees_every_resource(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> bool {
    // Reserved graveyard cards and announced sacrifices are spendable by the
    // planner and invisible to the solver.
    if !request.reserved_graveyard_sources.is_empty()
        || !request.reserved_permanent_sources.is_empty()
    {
        return false;
    }
    // Convoke, Delve and Improvise pay pips without producing mana, and only
    // ever apply while casting a spell.
    if request.reason == crate::costs::PaymentReason::CastSpell {
        let Some(source) = game.object(request.source) else {
            return false;
        };
        if crate::decision::has_convoke(source)
            || crate::decision::has_delve(source)
            || crate::decision::has_improvise(source)
        {
            return false;
        }
    }
    true
}

impl ManaPaymentPlanner {
    pub fn plan(
        mut self,
        game: &GameState,
        request: &ManaPaymentRequest,
    ) -> Result<Vec<ManaPaymentPlan>, ManaPaymentFailure> {
        self.plan_internal(game, request, false)
    }

    pub fn first_plan(
        mut self,
        game: &GameState,
        request: &ManaPaymentRequest,
    ) -> Result<ManaPaymentPlan, ManaPaymentFailure> {
        let result = self.plan_internal(game, request, true);
        // Previews are the searches that actually hurt, so report their cost
        // the same way a full plan reports its own.
        LAST_MANA_PAYMENT_PERF.with(|slot| {
            *slot.borrow_mut() = ManaPaymentPerfMetrics {
                visited_nodes: self.visited_nodes,
                search_limited: matches!(&result, Err(ManaPaymentFailure::SearchLimitReached)),
                plans_returned: result.as_ref().map_or(0, Vec::len),
                analytic_selections: self.analytic_selections,
                searched_selections: self.searched_selections,
            };
        });
        result?
            .into_iter()
            .next()
            .ok_or(ManaPaymentFailure::NoLegalPlan)
    }

    fn plan_internal(
        &mut self,
        game: &GameState,
        request: &ManaPaymentRequest,
        stop_after_first: bool,
    ) -> Result<Vec<ManaPaymentPlan>, ManaPaymentFailure> {
        let player = game
            .player(request.payer)
            .ok_or(ManaPaymentFailure::MissingPlayer)?;
        if request
            .preferences
            .required_sources
            .iter()
            .any(|source| request.preferences.excluded_sources.contains(source))
            || request
                .preferences
                .required_activations
                .iter()
                .any(|activation| {
                    request
                        .preferences
                        .excluded_sources
                        .contains(&activation.source)
                })
            || request
                .preferences
                .required_alternatives
                .iter()
                .any(|alternative| {
                    request
                        .preferences
                        .excluded_sources
                        .contains(&alternative.source)
                })
        {
            return Err(ManaPaymentFailure::ConflictingPreferences);
        }

        // Only on a fresh search: a resumed slice has already paid for this.
        if self.outer.is_none()
            && !self.skip_affordability_gate
            && affordability_rules_out_payment(game, request)
        {
            return Err(ManaPaymentFailure::NoLegalPlan);
        }

        let mut cursor = self.outer.take().unwrap_or_else(|| PlanningCursor {
            selections: alternative_payment_selections(game, request).into_iter(),
            pool_before: player.mana_pool.clone(),
            plans: Vec::new(),
            active: None,
        });
        loop {
            if cursor.plans.len() >= MAX_TOTAL_PLANS
                || (stop_after_first && !cursor.plans.is_empty())
            {
                break;
            }
            if cursor.active.is_none() {
                if self.sliced && self.remaining == 0 && cursor.selections.len() > 0 {
                    self.pending = true;
                    self.outer = Some(cursor);
                    return Err(ManaPaymentFailure::SearchLimitReached);
                }
                let Some(selection) = cursor.selections.next() else {
                    break;
                };
                if self.sliced {
                    self.remaining = self.remaining.saturating_sub(1);
                }
                // CR 601.2g precedes keyword payments in 601.2h. Reserve
                // resources while searching, but do not tap/exile them early.
                let staged = game.clone();
                let mut payment_request = request.clone();
                for allocation in &selection.allocations {
                    match allocation.payment {
                        super::PlannedPipPayment::Convoke(source)
                        | super::PlannedPipPayment::Improvise(source) => {
                            payment_request.reserved_tap_sources.push(source);
                        }
                        super::PlannedPipPayment::Delve(source) => {
                            payment_request.reserved_graveyard_sources.push(source);
                        }
                        _ => {}
                    }
                }
                payment_request.cost = crate::mana::ManaCost::from_pips(
                    selection
                        .remaining
                        .iter()
                        .map(|slot| slot.alternatives.clone())
                        .collect(),
                );
                for allocation in &selection.allocations {
                    let source = match allocation.payment {
                        super::PlannedPipPayment::Convoke(source)
                        | super::PlannedPipPayment::Improvise(source)
                        | super::PlannedPipPayment::Delve(source) => source,
                        _ => continue,
                    };
                    payment_request
                        .preferences
                        .required_sources
                        .retain(|required| *required != source);
                }
                if !self.skip_affordability_gate
                    && remaining_mana_is_unpayable(&staged, &payment_request)
                {
                    continue;
                }
                let payable = can_pay_request(&staged, &payment_request);
                let seek_zero_life = payment_request.allow_mana_abilities
                    && payable
                    && !payment_request.preferences.prefer_life
                    && preview_life_to_pay(&staged, &payment_request) > 0;
                if payable
                    && payment_request.preferences.required_sources.is_empty()
                    && payment_request.preferences.required_activations.is_empty()
                    && !seek_zero_life
                {
                    let pool_after = staged
                        .player(request.payer)
                        .ok_or(ManaPaymentFailure::MissingPlayer)?
                        .mana_pool
                        .clone();
                    cursor.plans.push(build_plan(
                        &staged,
                        request,
                        &payment_request,
                        &selection,
                        cursor.pool_before.clone(),
                        pool_after,
                        Vec::new(),
                    ));
                    continue;
                }
                if !request.allow_mana_abilities {
                    continue;
                }
                // Payments where every source just taps for a fixed bundle are
                // an assignment, not a search: solving them directly replaces
                // thousands of state clones with one per source. The module
                // declines anything it cannot model, so this only ever skips
                // work the search would have repeated.
                //
                // Existence checks are the exception. They only need *a* plan,
                // which the lazy search already reaches by following a single
                // candidate line, while the assignment measures every candidate
                // before it can solve. Ranking is where the search explodes and
                // where measuring every candidate pays for itself.
                if (!self.lazy_candidates || self.preview_assignment)
                    && let Some(candidates) =
                        super::analytic::try_candidates(&staged, &payment_request)
                {
                    self.visited_nodes = 0;
                    self.analytic_selections += 1;
                    for (final_game, steps) in candidates {
                        let pool_after = final_game
                            .player(request.payer)
                            .ok_or(ManaPaymentFailure::MissingPlayer)?
                            .mana_pool
                            .clone();
                        cursor.plans.push(build_plan(
                            &final_game,
                            request,
                            &payment_request,
                            &selection,
                            cursor.pool_before.clone(),
                            pool_after,
                            steps,
                        ));
                    }
                    if !cursor.plans.is_empty() {
                        continue;
                    }
                }
                let depth_limit = expanded_pip_count(&payment_request)
                    .saturating_add(MAX_EXTRA_ACTIVATIONS)
                    .max(payment_request.preferences.required_activations.len())
                    .max(1);
                self.visited_nodes = 0;
                self.searched_selections += 1;
                let search = CandidateSearch::new(
                    staged,
                    &payment_request,
                    depth_limit,
                    stop_after_first,
                    self.lazy_candidates,
                );
                cursor.active = Some(SelectionWork {
                    selection,
                    request: payment_request,
                    search,
                });
            }
            let mut active = cursor.active.take().expect("prepared selection");
            let mut unbounded = usize::MAX;
            let budget = if self.sliced {
                &mut self.remaining
            } else {
                &mut unbounded
            };
            let result = active.search.step(&active.request, budget);
            self.visited_nodes = active.search.visited;
            let Some(candidates) = result else {
                cursor.active = Some(active);
                self.outer = Some(cursor);
                self.pending = true;
                return Err(ManaPaymentFailure::SearchLimitReached);
            };
            for (final_game, steps) in candidates? {
                let pool_after = final_game
                    .player(request.payer)
                    .ok_or(ManaPaymentFailure::MissingPlayer)?
                    .mana_pool
                    .clone();
                cursor.plans.push(build_plan(
                    &final_game,
                    request,
                    &active.request,
                    &active.selection,
                    cursor.pool_before.clone(),
                    pool_after,
                    steps,
                ));
                if stop_after_first || cursor.plans.len() >= MAX_TOTAL_PLANS {
                    break;
                }
            }
        }
        cursor.plans.sort_by_key(|plan| plan.score);
        cursor.plans.dedup_by_key(|plan| plan.id);
        if cursor.plans.is_empty() {
            Err(ManaPaymentFailure::NoLegalPlan)
        } else {
            Ok(cursor.plans)
        }
    }
}

type Candidate = (GameState, Vec<PlannedManaActivation>);
type PreparedChoice = (
    (u8, (u8, u8, u8, usize, u64, usize)),
    GameState,
    PlannedManaActivation,
);

#[derive(Debug)]
struct Expansion {
    game: GameState,
    path: Vec<SearchStep>,
    choices: std::vec::IntoIter<ActivationChoice>,
    prepared: Vec<PreparedChoice>,
}

/// The same search machine serves synchronous transactions and sliced previews.
/// Each activation simulation consumes one work unit, rather than treating an
/// entire battlefield's activation permutations as one indivisible node.
#[derive(Debug)]
struct CandidateSearch {
    queue: VecDeque<(GameState, Vec<SearchStep>)>,
    seen: HashSet<u64>,
    enqueued: usize,
    visited: usize,
    out: Vec<(ManaPaymentScore, GameState, Vec<PlannedManaActivation>)>,
    limited: bool,
    expansion: Option<Expansion>,
    deferred_expansions: Vec<Expansion>,
    first_seen_depths: HashMap<u64, usize>,
    depth_limit: usize,
    first: bool,
    lazy_candidates: bool,
    result: Option<Result<Vec<Candidate>, ManaPaymentFailure>>,
}

impl CandidateSearch {
    fn new(
        game: GameState,
        request: &ManaPaymentRequest,
        depth_limit: usize,
        first: bool,
        lazy_candidates: bool,
    ) -> Self {
        let root_key = safe_search_state_key(&game, request.payer);
        let seen = HashSet::from([root_key]);
        Self {
            queue: VecDeque::from([(game, Vec::new())]),
            seen,
            enqueued: 1,
            visited: 0,
            out: Vec::new(),
            limited: false,
            expansion: None,
            deferred_expansions: Vec::new(),
            first_seen_depths: HashMap::from([(root_key, 0)]),
            depth_limit,
            first,
            lazy_candidates,
            result: None,
        }
    }
    fn finish(&mut self) -> Option<Result<Vec<Candidate>, ManaPaymentFailure>> {
        self.out.sort_by_key(|candidate| candidate.0);
        let result = if self.out.is_empty() && self.limited {
            Err(ManaPaymentFailure::SearchLimitReached)
        } else {
            Ok(std::mem::take(&mut self.out)
                .into_iter()
                .map(|(_, game, actions)| (game, actions))
                .collect())
        };
        self.queue.clear();
        self.expansion = None;
        self.deferred_expansions.clear();
        self.first_seen_depths.clear();
        self.seen.clear();
        self.result = Some(result.clone());
        Some(result)
    }
    fn step(
        &mut self,
        request: &ManaPaymentRequest,
        remaining: &mut usize,
    ) -> Option<Result<Vec<Candidate>, ManaPaymentFailure>> {
        if let Some(result) = &self.result {
            return Some(result.clone());
        }
        while *remaining > 0 {
            *remaining -= 1;
            if let Some(mut expansion) = self.expansion.take() {
                if let Some(choice) = expansion.choices.next() {
                    if let Some(prepared) = prepare_activation(&expansion.game, request, choice) {
                        if self.lazy_candidates {
                            // Follow one candidate before simulating its siblings. Keep
                            // the parent iterator so failed branches and sliced searches
                            // resume without rebuilding or losing alternatives.
                            let (_, staged, activation) = prepared;
                            let mut next_path = expansion.path.clone();
                            next_path.push(SearchStep { activation });
                            // Unlike breadth-first search, a later visit can have
                            // more depth remaining. Do not prune that shorter path.
                            let duplicate =
                                if next_path.iter().all(|step| step.activation.undo_safe) {
                                    let key = safe_search_state_key(&staged, request.payer);
                                    let previous =
                                        self.first_seen_depths.entry(key).or_insert(usize::MAX);
                                    if *previous <= next_path.len() {
                                        true
                                    } else {
                                        *previous = next_path.len();
                                        false
                                    }
                                } else {
                                    false
                                };
                            if !duplicate {
                                if self.enqueued >= MAX_SEARCH_NODES {
                                    self.limited = true;
                                    return self.finish();
                                }
                                self.enqueued += 1;
                                self.queue.push_front((staged, next_path));
                                self.deferred_expansions.push(expansion);
                                continue;
                            }
                        } else {
                            expansion.prepared.push(prepared);
                        }
                    }
                    self.expansion = Some(expansion);
                    continue;
                }
                expansion.prepared.sort_by_key(|candidate| candidate.0);
                for (_, staged, activation) in expansion.prepared {
                    let mut next_path = expansion.path.clone();
                    next_path.push(SearchStep { activation });
                    if next_path.iter().all(|step| step.activation.undo_safe)
                        && !self
                            .seen
                            .insert(safe_search_state_key(&staged, request.payer))
                    {
                        continue;
                    }
                    if self.enqueued >= MAX_SEARCH_NODES {
                        self.limited = true;
                        break;
                    }
                    self.queue.push_back((staged, next_path));
                    self.enqueued += 1;
                }
                continue;
            }
            let Some((game, path)) = self.queue.pop_front() else {
                if let Some(expansion) = self.deferred_expansions.pop() {
                    self.expansion = Some(expansion);
                    continue;
                }
                return self.finish();
            };
            self.visited += 1;
            if self.visited > MAX_SEARCH_NODES {
                self.limited = true;
                return self.finish();
            }
            if can_pay_request(&game, request) && required_activations_are_present(request, &path) {
                let life_to_pay = preview_life_to_pay(&game, request);
                let activations = path
                    .iter()
                    .map(|step| step.activation.clone())
                    .collect::<Vec<_>>();
                let score = search_candidate_score(&game, request, &activations);
                self.out.push((score, game.clone(), activations));
                if self.first || score_reaches_search_floor(score) {
                    return self.finish();
                }
                self.out.sort_by_key(|candidate| candidate.0);
                self.out.truncate(MAX_PLANS_PER_SELECTION);
                if request.preferences.prefer_life || life_to_pay == 0 {
                    continue;
                }
            }
            if path.len() >= self.depth_limit {
                continue;
            }
            // Collapsed only for the search; the inventory entry points keep
            // every source so the client can still offer them all.
            let choices = collect_search_choices(&game, request).into_iter();
            self.expansion = Some(Expansion {
                game,
                path,
                choices,
                prepared: Vec::new(),
            });
        }
        None
    }
}

pub(super) fn prepare_activation(
    game: &GameState,
    request: &ManaPaymentRequest,
    choice: ActivationChoice,
) -> Option<PreparedChoice> {
    let mut staged = game.clone();
    let before = staged
        .player(request.payer)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    // An undo-safe activation taps the source and adds mana and does nothing
    // else, so when no continuous effect can observe a tap or a pool change the
    // parent's continuous state is still correct for the staged state.
    let retainable = game.continuous_state_is_clean()
        && crate::game_loop::mana_ability_is_undo_safe(game, choice.source, choice.ability_index)
        && !game.continuous_effects_are_tap_sensitive();
    let mut decision_maker = SelectFirstDecisionMaker;
    if crate::special_actions::perform_activate_mana_ability_restricted_colors(
        &mut staged,
        request.payer,
        choice.source,
        choice.ability_index,
        choice.color_restriction.clone(),
        &mut decision_maker,
    )
    .is_err()
    {
        return None;
    }
    if !retainable || !staged.retain_continuous_state_after_mana_activation() {
        staged.refresh_continuous_state();
    }
    let after = staged
        .player(request.payer)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    if after == before {
        return None;
    }

    let preference_key = activation_preference_key(request, &choice);
    let activation = PlannedManaActivation {
        source: choice.source,
        ability_index: choice.ability_index,
        color_restriction: choice.color_restriction,
        expected_mana: positive_pool_delta(&before, &after),
        expected_pool_after: after,
        flexibility: choice.flexibility,
        undo_safe: crate::game_loop::mana_ability_is_undo_safe(
            &game,
            choice.source,
            choice.ability_index,
        ),
    };
    let completes_payment = can_pay_request(&staged, request);
    let completion_rank = if !completes_payment {
        2
    } else if !request.preferences.prefer_life && preview_life_to_pay(&staged, request) > 0 {
        1
    } else {
        0
    };
    Some(((completion_rank, preference_key), staged, activation))
}

/// A first-plan query bound to an immutable game and request. Budget exhaustion
/// returns None; it is never converted into an unpayable verdict.
#[derive(Debug)]
pub struct ManaPaymentAnalysis {
    game: Box<GameState>,
    request: ManaPaymentRequest,
    planner: ManaPaymentPlanner,
    result: Option<Result<ManaPaymentPlan, ManaPaymentFailure>>,
    /// Search units the most recent slice actually consumed, so a scheduler can
    /// tell search cost apart from the fixed cost of setting a slice up.
    last_slice_units: usize,
    ranked: bool,
}
impl ManaPaymentAnalysis {
    pub fn new(game: &GameState, request: ManaPaymentRequest) -> Self {
        Self {
            game: Box::new(game.clone()),
            request,
            planner: ManaPaymentPlanner {
                sliced: true,
                lazy_candidates: true,
                preview_assignment: true,
                ..Default::default()
            },
            result: None,
            last_slice_units: 0,
            ranked: false,
        }
    }

    /// The sliced twin of [`check_mana_payment`]: can this cost be paid at all?
    ///
    /// Ranking plans means simulating every sibling activation, which grows
    /// exponentially with the untapped sources on the battlefield — eight lands
    /// already take hundreds of milliseconds to answer "yes" for `{4}`. A caller
    /// that only reads the yes/no, such as the inspector greying out an
    /// ability, follows one candidate line instead and answers in constant time.
    pub fn check(game: &GameState, request: ManaPaymentRequest) -> Self {
        Self {
            game: Box::new(game.clone()),
            request,
            planner: ManaPaymentPlanner {
                sliced: true,
                lazy_candidates: true,
                ..Default::default()
            },
            result: None,
            last_slice_units: 0,
            ranked: false,
        }
    }

    /// Optional ranking work, resumed between foreground commands.
    pub fn ranked(game: &GameState, request: ManaPaymentRequest) -> Self {
        let mut analysis = Self::new(game, request);
        analysis.ranked = true;
        analysis.planner.lazy_candidates = false;
        analysis.planner.preview_assignment = false;
        analysis
    }

    /// Units consumed by the last [`Self::step`].
    pub fn last_slice_units(&self) -> usize {
        self.last_slice_units
    }

    pub fn step(&mut self, budget: usize) -> Option<Result<ManaPaymentPlan, ManaPaymentFailure>> {
        if let Some(result) = &self.result {
            self.last_slice_units = 0;
            return Some(result.clone());
        }
        let budget = budget.max(1);
        self.planner.remaining = budget;
        self.planner.pending = false;
        let result = self
            .planner
            .plan_internal(&self.game, &self.request, !self.ranked)
            .and_then(|plans| {
                plans
                    .into_iter()
                    .next()
                    .ok_or(ManaPaymentFailure::NoLegalPlan)
            });
        self.last_slice_units = budget.saturating_sub(self.planner.remaining);
        if self.planner.pending {
            None
        } else {
            self.planner.outer = None;
            self.result = Some(result.clone());
            Some(result)
        }
    }
}

fn score_reaches_search_floor(score: ManaPaymentScore) -> bool {
    score.irreversible_cost == 0
        && score.life_paid == 0
        && score.preserved_sources_used == 0
        && score.excess_mana == 0
        && score.flexible_sources_used == 0
}

#[derive(Debug, Clone)]
pub(super) struct ActivationChoice {
    pub(super) source: ObjectId,
    pub(super) ability_index: usize,
    pub(super) color_restriction: Option<Vec<Color>>,
    pub(super) flexibility: usize,
}

#[derive(Debug, Clone)]
struct PaymentPipSlot {
    pip: ManaPipId,
    printed_index: usize,
    alternatives: Vec<ManaSymbol>,
}

#[derive(Debug, Clone, Copy)]
enum AlternativeKind {
    Convoke(crate::color::ColorSet),
    Improvise,
    Delve,
}

#[derive(Debug, Clone, Copy)]
struct AlternativeSource {
    source: ObjectId,
    kind: AlternativeKind,
    required: bool,
}

#[derive(Debug, Clone)]
struct AlternativeSelection {
    remaining: Vec<PaymentPipSlot>,
    allocations: Vec<PlannedPipAllocation>,
}

const MAX_ALTERNATIVE_SELECTIONS: usize = 128;

fn delve_cards(game: &GameState, request: &ManaPaymentRequest) -> Vec<ObjectId> {
    game.player(request.payer)
        .map(|player| {
            player
                .graveyard
                .iter()
                .copied()
                .filter(|id| {
                    *id != request.source
                        && game.object(*id).is_some_and(|obj| {
                            !matches!(obj.kind, crate::object::ObjectKind::Token)
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn alternative_payment_selections(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<AlternativeSelection> {
    let actual_black_life = request.allow_black_life
        && game.player_can_pay_black_with_life_for_reason(
            request.payer,
            Some(request.source),
            request.reason,
        );
    let mut pips =
        GameState::expanded_payment_pips(&request.cost, request.x_value, actual_black_life)
            .into_iter()
            .enumerate()
            .map(|(index, alternatives)| PaymentPipSlot {
                pip: ManaPipId(index as u32),
                printed_index: index,
                alternatives,
            })
            .collect::<Vec<_>>();

    for required in &request.preferences.required_life_pips {
        let Some(slot) = pips.get_mut(required.0 as usize) else {
            return Vec::new();
        };
        if !request.allow_life_payment {
            return Vec::new();
        }
        slot.alternatives
            .retain(|symbol| matches!(symbol, ManaSymbol::Life(_)));
        if slot.alternatives.is_empty() {
            return Vec::new();
        }
    }

    if request.reason != crate::costs::PaymentReason::CastSpell {
        return vec![AlternativeSelection {
            remaining: pips,
            allocations: Vec::new(),
        }];
    }

    let Some(source) = game.object(request.source) else {
        return vec![AlternativeSelection {
            remaining: pips,
            allocations: Vec::new(),
        }];
    };
    if game.controller_of(source) != request.payer {
        return vec![AlternativeSelection {
            remaining: pips,
            allocations: Vec::new(),
        }];
    }
    let mut sources = Vec::new();
    if crate::decision::has_convoke(source) {
        sources.extend(
            crate::decision::get_convoke_creatures(game, request.payer)
                .into_iter()
                .filter(|(source, _)| {
                    !request.preferences.excluded_sources.contains(source)
                        && !request.reserved_tap_sources.contains(source)
                })
                .map(|(source, colors)| AlternativeSource {
                    source,
                    kind: AlternativeKind::Convoke(colors),
                    required: alternative_is_required(
                        request,
                        source,
                        ManaPaymentSourceKind::Convoke,
                    ),
                }),
        );
    }
    if crate::decision::has_delve(source) {
        sources.extend(
            delve_cards(game, request)
                .into_iter()
                .filter(|source| !request.preferences.excluded_sources.contains(source))
                .map(|source| AlternativeSource {
                    source,
                    kind: AlternativeKind::Delve,
                    required: alternative_is_required(
                        request,
                        source,
                        ManaPaymentSourceKind::Delve,
                    ),
                }),
        );
    }
    if crate::decision::has_improvise(source) {
        for artifact in crate::decision::get_improvise_artifacts(game, request.payer) {
            if request.preferences.excluded_sources.contains(&artifact)
                || request.reserved_tap_sources.contains(&artifact)
            {
                continue;
            }
            sources.push(AlternativeSource {
                source: artifact,
                kind: AlternativeKind::Improvise,
                required: alternative_is_required(
                    request,
                    artifact,
                    ManaPaymentSourceKind::Improvise,
                ),
            });
        }
    }
    sources.sort_by_key(|candidate| {
        (
            u8::from(!candidate.required),
            u8::from(
                request
                    .preferences
                    .preserve_sources
                    .contains(&candidate.source),
            ),
            candidate.source.0,
        )
    });

    let mut selections = Vec::new();
    for pips in monocolored_hybrid_pip_shapes(&pips, !sources.is_empty()) {
        let mut selected = vec![None; pips.len()];
        // Explore both resource-heavy and mana-heavy ends of the bounded search.
        // Otherwise a large graveyard can exhaust the budget on small subsets.
        let mut mana_first = Vec::new();
        enumerate_alternative_selections(&pips, &sources, 0, &mut selected, &mut mana_first, false);
        selections.extend(mana_first);
        let mut resource_first = Vec::new();
        enumerate_alternative_selections(
            &pips,
            &sources,
            0,
            &mut selected,
            &mut resource_first,
            true,
        );
        selections.extend(resource_first);
    }
    selections.sort_by_key(|selection| {
        let selected_sources = selection
            .allocations
            .iter()
            .filter_map(|allocation| match allocation.payment {
                super::PlannedPipPayment::Convoke(source)
                | super::PlannedPipPayment::Improvise(source)
                | super::PlannedPipPayment::Delve(source) => Some(source),
                _ => None,
            })
            .collect::<Vec<_>>();
        let missing_required = request
            .preferences
            .required_sources
            .iter()
            .filter(|source| !selected_sources.contains(source))
            .count();
        let missing_exact_alternatives = request
            .preferences
            .required_alternatives
            .iter()
            .filter(|required| {
                !selection
                    .allocations
                    .iter()
                    .any(|allocation| allocation_matches_required_alternative(required, allocation))
            })
            .count();
        let preserved = selected_sources
            .iter()
            .filter(|source| request.preferences.preserve_sources.contains(source))
            .count();
        (
            missing_required + missing_exact_alternatives,
            selection.allocations.len(),
            preserved,
        )
    });
    selections.retain(|selection| {
        request
            .preferences
            .required_alternatives
            .iter()
            .all(|required| {
                selection
                    .allocations
                    .iter()
                    .any(|allocation| allocation_matches_required_alternative(required, allocation))
            })
    });
    selections
}

/// Monocolored hybrid pips ({2/W}) may be paid through their generic half,
/// which is N separate generic mana. Convoke, delve, and improvise pay one
/// generic mana per resource (CR 702.51a, 702.66a, 702.126a), so two
/// resources can jointly pay a {2/W}'s {2}. Offer each such pip both as
/// printed and split into N {1} slots (with fresh pip ids, same printed
/// index). The number of reshaped pips is bounded to keep the search small.
fn monocolored_hybrid_pip_shapes(
    pips: &[PaymentPipSlot],
    has_alternative_sources: bool,
) -> Vec<Vec<PaymentPipSlot>> {
    const MAX_RESHAPED_PIPS: usize = 4;
    let hybrid_indices: Vec<(usize, u8)> = if has_alternative_sources {
        pips.iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.alternatives.iter().find_map(|symbol| match symbol {
                    ManaSymbol::Generic(amount) if *amount > 1 => Some((index, *amount)),
                    _ => None,
                })
            })
            .take(MAX_RESHAPED_PIPS)
            .collect()
    } else {
        Vec::new()
    };
    let mut shapes = vec![pips.to_vec()];
    for mask in 1u32..(1u32 << hybrid_indices.len()) {
        let mut next_id = pips.len() as u32;
        let mut shape = Vec::with_capacity(pips.len() + 4);
        for (index, slot) in pips.iter().enumerate() {
            let split = hybrid_indices
                .iter()
                .enumerate()
                .find(|(_, (hybrid_index, _))| *hybrid_index == index)
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, (_, amount))| *amount);
            match split {
                Some(amount) => {
                    for _ in 0..amount {
                        shape.push(PaymentPipSlot {
                            pip: ManaPipId(next_id),
                            printed_index: slot.printed_index,
                            alternatives: vec![ManaSymbol::Generic(1)],
                        });
                        next_id += 1;
                    }
                }
                None => shape.push(slot.clone()),
            }
        }
        shapes.push(shape);
    }
    shapes
}

fn allocation_matches_required_alternative(
    required: &super::RequiredAlternativePayment,
    allocation: &PlannedPipAllocation,
) -> bool {
    match (required.kind, &allocation.payment) {
        (ManaPaymentSourceKind::Convoke, super::PlannedPipPayment::Convoke(source))
        | (ManaPaymentSourceKind::Improvise, super::PlannedPipPayment::Improvise(source))
        | (ManaPaymentSourceKind::Delve, super::PlannedPipPayment::Delve(source)) => {
            *source == required.source
        }
        _ => false,
    }
}

fn alternative_is_required(
    request: &ManaPaymentRequest,
    source: ObjectId,
    kind: ManaPaymentSourceKind,
) -> bool {
    request.preferences.required_sources.contains(&source)
        || request
            .preferences
            .required_alternatives
            .iter()
            .any(|required| required.source == source && required.kind == kind)
}

fn enumerate_alternative_selections(
    pips: &[PaymentPipSlot],
    sources: &[AlternativeSource],
    source_index: usize,
    selected: &mut [Option<AlternativeSource>],
    out: &mut Vec<AlternativeSelection>,
    resource_first: bool,
) {
    if out.len() >= MAX_ALTERNATIVE_SELECTIONS {
        return;
    }
    if source_index == sources.len() {
        let mut remaining = Vec::new();
        let mut allocations = Vec::new();
        for (slot, alternative) in pips.iter().zip(selected.iter()) {
            if let Some(alternative) = alternative {
                let payment = match alternative.kind {
                    AlternativeKind::Convoke(_) => {
                        super::PlannedPipPayment::Convoke(alternative.source)
                    }
                    AlternativeKind::Delve => super::PlannedPipPayment::Delve(alternative.source),
                    AlternativeKind::Improvise => {
                        super::PlannedPipPayment::Improvise(alternative.source)
                    }
                };
                allocations.push(PlannedPipAllocation {
                    pip: slot.pip,
                    printed_index: slot.printed_index,
                    alternatives: slot.alternatives.clone(),
                    payment,
                });
            } else {
                remaining.push(slot.clone());
            }
        }
        out.push(AlternativeSelection {
            remaining,
            allocations,
        });
        return;
    }

    let source = sources[source_index];
    let include_source = |selected: &mut [Option<AlternativeSource>],
                          out: &mut Vec<AlternativeSelection>| {
        if selected
            .iter()
            .flatten()
            .any(|choice| choice.source == source.source)
        {
            return;
        }
        let mut equivalent_pips = Vec::new();
        for (pip_index, pip) in pips.iter().enumerate() {
            if selected[pip_index].is_some()
                || !alternative_can_pay(source.kind, &pip.alternatives)
                || equivalent_pips.contains(&&pip.alternatives)
            {
                continue;
            }
            equivalent_pips.push(&pip.alternatives);
            selected[pip_index] = Some(source);
            enumerate_alternative_selections(
                pips,
                sources,
                source_index + 1,
                selected,
                out,
                resource_first,
            );
            selected[pip_index] = None;
            if out.len() >= MAX_ALTERNATIVE_SELECTIONS {
                break;
            }
        }
    };
    if source.required || resource_first {
        include_source(selected, out);
    }
    if out.len() < MAX_ALTERNATIVE_SELECTIONS {
        enumerate_alternative_selections(
            pips,
            sources,
            source_index + 1,
            selected,
            out,
            resource_first,
        );
    }
    if !source.required && !resource_first && out.len() < MAX_ALTERNATIVE_SELECTIONS {
        include_source(selected, out);
    }
}

fn alternative_can_pay(kind: AlternativeKind, pip: &[ManaSymbol]) -> bool {
    // CR 702.51a / 702.66a / 702.126a: each tapped creature, tapped artifact,
    // or exiled card pays for one generic mana. A single resource can't pay the
    // {2} half of a monocolored hybrid {2/W} pip (standalone generic and X
    // pips are already split into {1} units).
    pip.iter().any(|symbol| match (kind, symbol) {
        (_, ManaSymbol::Generic(amount)) if *amount > 1 => false,
        (AlternativeKind::Convoke(_), ManaSymbol::Generic(_)) => true,
        (AlternativeKind::Convoke(colors), ManaSymbol::White) => colors.contains(Color::White),
        (AlternativeKind::Convoke(colors), ManaSymbol::Blue) => colors.contains(Color::Blue),
        (AlternativeKind::Convoke(colors), ManaSymbol::Black) => colors.contains(Color::Black),
        (AlternativeKind::Convoke(colors), ManaSymbol::Red) => colors.contains(Color::Red),
        (AlternativeKind::Convoke(colors), ManaSymbol::Green) => colors.contains(Color::Green),
        (AlternativeKind::Improvise | AlternativeKind::Delve, ManaSymbol::Generic(_)) => true,
        _ => false,
    })
}

/// True when every mana this ability adds carries a usage restriction that
/// forbids spending it on `request`.
///
/// Restricted mana is still a legal activation, but exploring it costs a full
/// `GameState` clone per producible colour in [`prepare_activation`], so the
/// search skips branches whose output provably cannot pay this request. The
/// check is deliberately one-sided: anything it cannot decide is treated as
/// usable, so pruning never removes a payment the player could actually make.
fn ability_mana_is_unusable_for_request(
    game: &GameState,
    request: &ManaPaymentRequest,
    source: ObjectId,
    ability: &crate::ability::ActivatedAbility,
) -> bool {
    if ability.mana_usage_restrictions.is_empty() {
        return false;
    }
    // `source_chosen_creature_type: None` makes a subtype requirement match
    // anything, which keeps an undecidable restriction on the usable side.
    let unit = crate::ability::RestrictedManaUnit {
        symbol: ManaSymbol::Colorless,
        source,
        source_chosen_creature_type: None,
        restrictions: ability.mana_usage_restrictions.clone(),
    };
    !game.restricted_mana_unit_is_payable_for_transaction(
        &unit,
        Some(request.source),
        request.reason,
        Some(&request.cost),
    )
}

/// Collapses activation choices that the rest of the search and the plan scorer
/// cannot tell apart, keeping the lowest-id representative of each class.
///
/// Sixteen untapped Forests offer sixteen branches at every node even though
/// every resulting state and every resulting score is identical, which is what
/// makes a wide board expensive. Two choices are only merged when everything
/// downstream reads the same from either: the mana produced, the restrictions
/// that mana carries, snow provenance, and each preference that names a source
/// individually. Only undo-safe abilities are eligible, so a merged class is
/// always "tap this, add these symbols" with no other game effect.
///
/// This narrows which plans are *offered*, not which payments are *possible*:
/// picking a specific source is expressed through `required_sources` and
/// `required_activations`, and both are part of the class key, so a constrained
/// replan still sees the source the player named.
fn collapse_interchangeable_choices(
    game: &GameState,
    request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>,
    choices: Vec<ActivationChoice>,
) -> Vec<ActivationChoice> {
    #[derive(PartialEq)]
    struct ClassKey {
        symbols: Vec<ManaSymbol>,
        color_restriction: Option<Vec<Color>>,
        flexibility: usize,
        snow: bool,
        restrictions: Vec<crate::ability::ManaUsageRestriction>,
        exact_required: bool,
        required_source: bool,
        preserved: bool,
        reserved_tap: bool,
    }

    let mut classes: Vec<(ClassKey, usize)> = Vec::new();
    let mut keep = vec![false; choices.len()];
    for (index, choice) in choices.iter().enumerate() {
        let Some(object) = game.object(choice.source) else {
            keep[index] = true;
            continue;
        };
        let abilities = view
            .abilities_rc(choice.source)
            .unwrap_or_else(|| std::sync::Arc::new(object.abilities_vec()));
        let Some(ability) = abilities.get(choice.ability_index) else {
            keep[index] = true;
            continue;
        };
        let AbilityKind::Activated(mana_ability) = &ability.kind else {
            keep[index] = true;
            continue;
        };
        // A non-undo-safe activation can do anything to the game, so it is
        // never merged with another source.
        if !crate::game_loop::mana_ability_is_undo_safe(game, choice.source, choice.ability_index) {
            keep[index] = true;
            continue;
        }
        let mut symbols = mana_ability.inferred_mana_symbols(game, choice.source, request.payer);
        symbols.sort_by_key(|symbol| format!("{symbol:?}"));
        let key = ClassKey {
            symbols,
            color_restriction: choice.color_restriction.clone(),
            flexibility: choice.flexibility,
            snow: game.current_has_supertype(choice.source, crate::types::Supertype::Snow),
            restrictions: mana_ability.mana_usage_restrictions.clone(),
            exact_required: request
                .preferences
                .required_activations
                .iter()
                .any(|required| activation_choice_matches(required, choice)),
            required_source: request
                .preferences
                .required_sources
                .contains(&choice.source),
            preserved: request
                .preferences
                .preserve_sources
                .contains(&choice.source),
            reserved_tap: request.reserved_tap_sources.contains(&choice.source),
        };
        match classes.iter().find(|(candidate, _)| *candidate == key) {
            Some(_) => continue,
            None => {
                classes.push((key, index));
                keep[index] = true;
            }
        }
    }
    choices
        .into_iter()
        .enumerate()
        .filter_map(|(index, choice)| keep[index].then_some(choice))
        .collect()
}

pub(super) fn collect_activation_choices(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<ActivationChoice> {
    collect_activation_choices_inner(game, request, false)
}

/// The search's view of the same list, with interchangeable sources collapsed.
/// Shares one derived view with the collection pass.
pub(super) fn collect_search_choices(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<ActivationChoice> {
    collect_activation_choices_inner(game, request, true)
}

fn collect_activation_choices_inner(
    game: &GameState,
    request: &ManaPaymentRequest,
    collapse: bool,
) -> Vec<ActivationChoice> {
    if !request.allow_mana_abilities {
        return Vec::new();
    }
    let view = DerivedGameView::new(game);
    let analysis = view.simple_battlefield_mana_analysis(request.payer);
    let mut out = Vec::new();

    for &source in analysis.mana_source_ids() {
        if request.preferences.excluded_sources.contains(&source) {
            continue;
        }
        let Some(object) = game.object(source) else {
            continue;
        };
        let abilities = view
            .abilities_rc(source)
            .unwrap_or_else(|| std::sync::Arc::new(object.abilities_vec()));
        for &ability_index in analysis.mana_ability_indices_for(source) {
            let Some(ability) = abilities.get(ability_index) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if (request.reserved_tap_sources.contains(&source) && mana_ability.has_tap_cost())
                || !mana_ability.is_runtime_mana_ability(game, source, request.payer)
                || crate::special_actions::can_activate_mana_ability_check_with_view(
                    game,
                    request.payer,
                    source,
                    ability_index,
                    ability,
                    &view,
                    None,
                )
                .is_err()
                || ability_mana_is_unusable_for_request(game, request, source, mana_ability)
            {
                continue;
            }
            let inferred = mana_ability.inferred_mana_symbols(game, source, request.payer);
            let colors = inferred
                .iter()
                .filter_map(|symbol| mana_symbol_color(*symbol))
                .collect::<HashSet<_>>();
            let flexibility = colors.len();
            if flexibility > 1 {
                for color in Color::ALL {
                    if colors.contains(&color) {
                        out.push(ActivationChoice {
                            source,
                            ability_index,
                            color_restriction: Some(vec![color]),
                            flexibility,
                        });
                    }
                }
            }
            out.push(ActivationChoice {
                source,
                ability_index,
                color_restriction: None,
                flexibility,
            });
        }
    }
    if collapse {
        out = collapse_interchangeable_choices(game, request, &view, out);
    }
    out
}

fn mana_symbol_color(symbol: ManaSymbol) -> Option<Color> {
    match symbol {
        ManaSymbol::White => Some(Color::White),
        ManaSymbol::Blue => Some(Color::Blue),
        ManaSymbol::Black => Some(Color::Black),
        ManaSymbol::Red => Some(Color::Red),
        ManaSymbol::Green => Some(Color::Green),
        _ => None,
    }
}

fn activation_preference_key(
    request: &ManaPaymentRequest,
    choice: &ActivationChoice,
) -> (u8, u8, u8, usize, u64, usize) {
    let exact_required = request
        .preferences
        .required_activations
        .iter()
        .any(|required| activation_choice_matches(required, choice));
    let required = !request.preferences.required_sources.is_empty()
        && request
            .preferences
            .required_sources
            .contains(&choice.source);
    let preserved = request
        .preferences
        .preserve_sources
        .contains(&choice.source);
    (
        u8::from(!exact_required),
        u8::from(!required),
        u8::from(preserved),
        choice.flexibility,
        choice.source.0,
        choice.ability_index,
    )
}

fn activation_choice_matches(required: &RequiredManaActivation, choice: &ActivationChoice) -> bool {
    required.source == choice.source
        && required.ability_index == choice.ability_index
        && required.color_restriction == choice.color_restriction
}

fn required_activations_are_present(request: &ManaPaymentRequest, path: &[SearchStep]) -> bool {
    let sources_present = request
        .preferences
        .required_sources
        .iter()
        .all(|required| path.iter().any(|step| step.activation.source == *required));
    if !sources_present {
        return false;
    }
    let mut matched = vec![false; path.len()];
    for required in &request.preferences.required_activations {
        let Some((index, _)) = path.iter().enumerate().find(|(index, step)| {
            !matched[*index]
                && required.source == step.activation.source
                && required.ability_index == step.activation.ability_index
                && required.color_restriction == step.activation.color_restriction
        }) else {
            return false;
        };
        matched[index] = true;
    }
    true
}

fn expanded_pip_count(request: &ManaPaymentRequest) -> usize {
    request
        .cost
        .pips()
        .iter()
        .map(|pip| match pip.as_slice() {
            [ManaSymbol::Generic(amount)] => *amount as usize,
            [ManaSymbol::X] => request.x_value as usize,
            _ => 1,
        })
        .sum()
}

fn positive_pool_delta(before: &ManaPool, after: &ManaPool) -> ManaPool {
    ManaPool {
        white: after.white.saturating_sub(before.white),
        blue: after.blue.saturating_sub(before.blue),
        black: after.black.saturating_sub(before.black),
        red: after.red.saturating_sub(before.red),
        green: after.green.saturating_sub(before.green),
        colorless: after.colorless.saturating_sub(before.colorless),
    }
}

pub(super) fn can_pay_request(game: &GameState, request: &ManaPaymentRequest) -> bool {
    if request.reserved_permanent_sources.iter().any(|id| {
        !game.object(*id).is_some_and(|object| {
            object.zone == crate::zone::Zone::Battlefield
                && game.controller_of(object) == request.payer
        })
    }) {
        return false;
    }
    if request.reserved_tap_sources.iter().any(|id| {
        game.is_tapped(*id)
            || !game.object(*id).is_some_and(|object| {
                object.zone == crate::zone::Zone::Battlefield
                    && game.controller_of(object) == request.payer
            })
    }) || request.reserved_graveyard_sources.iter().any(|id| {
        !game
            .player(request.payer)
            .is_some_and(|player| player.graveyard.contains(id))
    }) {
        return false;
    }

    game.can_pay_mana_cost_with_payment_options(
        request.payer,
        Some(request.source),
        &request.cost,
        request.x_value,
        request.reason,
        &request.spend_policy,
        request.allow_life_payment,
        request.allow_black_life,
        request.preferences.prefer_life,
    )
}

fn preview_life_to_pay(game: &GameState, request: &ManaPaymentRequest) -> u32 {
    game.preview_mana_cost_payment_with_options(
        request.payer,
        Some(request.source),
        &request.cost,
        request.x_value,
        request.reason,
        &request.spend_policy,
        request.allow_life_payment,
        request.allow_black_life,
        request.preferences.prefer_life,
    )
    .map(|(_, life)| life)
    .unwrap_or(0)
}

fn search_candidate_score(
    game: &GameState,
    request: &ManaPaymentRequest,
    activations: &[PlannedManaActivation],
) -> ManaPaymentScore {
    let mut after_payment = game.clone();
    let paid = after_payment.try_pay_mana_cost_with_payment_options(
        request.payer,
        Some(request.source),
        &request.cost,
        request.x_value,
        request.reason,
        &request.spend_policy,
        request.allow_life_payment,
        request.allow_black_life,
        request.preferences.prefer_life,
    );
    let excess_mana = if paid {
        after_payment
            .player(request.payer)
            .map(|player| player.mana_pool.total())
            .unwrap_or(u32::MAX)
    } else {
        u32::MAX
    };
    ManaPaymentScore {
        irreversible_cost: activations
            .iter()
            .filter(|activation| !activation.undo_safe)
            .count() as u32,
        life_paid: preview_life_to_pay(game, request),
        preserved_sources_used: activations
            .iter()
            .filter(|activation| {
                request
                    .preferences
                    .preserve_sources
                    .contains(&activation.source)
            })
            .count() as u32,
        excess_mana,
        flexible_sources_used: activations
            .iter()
            .filter(|activation| activation.flexibility > 1)
            .count() as u32,
        source_count: activations.len() as u32,
    }
}

fn build_plan(
    game: &GameState,
    request: &ManaPaymentRequest,
    payment_request: &ManaPaymentRequest,
    selection: &AlternativeSelection,
    pool_before: ManaPool,
    pool_after_activations: ManaPool,
    steps: Vec<PlannedManaActivation>,
) -> ManaPaymentPlan {
    let (preview, life_to_pay) = game
        .preview_mana_cost_payment_with_options(
            payment_request.payer,
            Some(payment_request.source),
            &payment_request.cost,
            payment_request.x_value,
            payment_request.reason,
            &payment_request.spend_policy,
            payment_request.allow_life_payment,
            payment_request.allow_black_life,
            payment_request.preferences.prefer_life,
        )
        .unwrap_or_default();
    let mut allocations = selection.allocations.clone();
    allocations.extend(preview.into_iter().zip(selection.remaining.iter()).map(
        |((alternatives, payment), slot)| PlannedPipAllocation {
            pip: slot.pip,
            printed_index: slot.printed_index,
            alternatives,
            payment,
        },
    ));
    allocations.sort_by_key(|allocation| allocation.pip);
    let mut staged = game.clone();
    let paid = staged.try_pay_mana_cost_with_payment_options(
        payment_request.payer,
        Some(payment_request.source),
        &payment_request.cost,
        payment_request.x_value,
        payment_request.reason,
        &payment_request.spend_policy,
        payment_request.allow_life_payment,
        payment_request.allow_black_life,
        payment_request.preferences.prefer_life,
    );
    let pool_after_payment = if paid {
        staged
            .player(request.payer)
            .map(|player| player.mana_pool.clone())
            .unwrap_or_default()
    } else {
        pool_after_activations.clone()
    };
    let excess = pool_after_payment.total();
    let alternative_sources = selection
        .allocations
        .iter()
        .filter_map(|allocation| match allocation.payment {
            super::PlannedPipPayment::Convoke(source)
            | super::PlannedPipPayment::Improvise(source)
            | super::PlannedPipPayment::Delve(source) => Some(source),
            _ => None,
        })
        .collect::<Vec<_>>();
    let non_undo_safe = steps.iter().filter(|step| !step.undo_safe).count() as u32
        + alternative_sources.len() as u32;
    let preserved_sources_used = steps
        .iter()
        .filter(|step| request.preferences.preserve_sources.contains(&step.source))
        .count() as u32
        + alternative_sources
            .iter()
            .filter(|source| request.preferences.preserve_sources.contains(source))
            .count() as u32;
    let score = ManaPaymentScore {
        irreversible_cost: non_undo_safe,
        life_paid: life_to_pay,
        preserved_sources_used,
        excess_mana: excess,
        flexible_sources_used: steps.iter().filter(|step| step.flexibility > 1).count() as u32,
        source_count: (steps.len() + alternative_sources.len()) as u32,
    };
    let mut warnings = Vec::new();
    for step in &steps {
        if !step.undo_safe {
            warnings.push(ManaPaymentWarning::UsesNonUndoSafeSource(step.source));
        }
        if request.preferences.preserve_sources.contains(&step.source) {
            warnings.push(ManaPaymentWarning::UsesPreservedSource(step.source));
        }
    }
    for source in alternative_sources {
        if request.preferences.preserve_sources.contains(&source) {
            warnings.push(ManaPaymentWarning::UsesPreservedSource(source));
        }
    }
    if life_to_pay > 0 {
        warnings.push(ManaPaymentWarning::PaysLife(life_to_pay));
    }
    if excess > 0 {
        warnings.push(ManaPaymentWarning::ProducesExcessMana(excess));
    }

    let request_hash = request_hash(request);
    let id = plan_hash(
        request_hash,
        &steps,
        &allocations,
        &payment_request.cost,
        &pool_after_payment,
    );
    ManaPaymentPlan {
        payable: true,
        id,
        request_hash,
        mana_ability_steps: steps,
        allocations,
        mana_cost_after_alternatives: payment_request.cost.clone(),
        pool_before,
        expected_pool_after_activations: pool_after_activations,
        expected_pool_after_payment: pool_after_payment,
        life_to_pay,
        score,
        warnings,
    }
}

/// Keep a payment window open after manual activations leave the cost unfunded.
/// This is a display proposal only and can never be confirmed or executed.
pub fn unfunded_mana_payment_plan(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> ManaPaymentPlan {
    let pool = game
        .player(request.payer)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    ManaPaymentPlan {
        payable: false,
        id: request_hash(request),
        request_hash: request_hash(request),
        mana_ability_steps: Vec::new(),
        allocations: Vec::new(),
        mana_cost_after_alternatives: request.cost.clone(),
        pool_before: pool.clone(),
        expected_pool_after_activations: pool.clone(),
        expected_pool_after_payment: pool,
        life_to_pay: 0,
        score: ManaPaymentScore::default(),
        warnings: Vec::new(),
    }
}

fn request_hash(request: &ManaPaymentRequest) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.payer.hash(&mut hasher);
    request.source.hash(&mut hasher);
    format!("{:?}", request.reason).hash(&mut hasher);
    request.cost.pips().hash(&mut hasher);
    request.x_value.hash(&mut hasher);
    request.allow_mana_abilities.hash(&mut hasher);
    request.reserved_tap_sources.hash(&mut hasher);
    request.reserved_graveyard_sources.hash(&mut hasher);
    request.reserved_permanent_sources.hash(&mut hasher);
    request.allow_life_payment.hash(&mut hasher);
    request.allow_black_life.hash(&mut hasher);
    format!("{:?}", request.spend_policy).hash(&mut hasher);
    request.preferences.required_sources.hash(&mut hasher);
    request.preferences.required_activations.hash(&mut hasher);
    request.preferences.required_alternatives.hash(&mut hasher);
    request.preferences.excluded_sources.hash(&mut hasher);
    request.preferences.preserve_sources.hash(&mut hasher);
    request.preferences.prefer_life.hash(&mut hasher);
    request.preferences.required_life_pips.hash(&mut hasher);
    hasher.finish()
}

fn plan_hash(
    request_hash: u64,
    steps: &[PlannedManaActivation],
    allocations: &[PlannedPipAllocation],
    payment_cost: &crate::mana::ManaCost,
    pool: &ManaPool,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request_hash.hash(&mut hasher);
    for step in steps {
        step.source.hash(&mut hasher);
        step.ability_index.hash(&mut hasher);
        step.color_restriction.hash(&mut hasher);
    }
    for allocation in allocations {
        allocation.pip.hash(&mut hasher);
        format!("{:?}", allocation.payment).hash(&mut hasher);
    }
    payment_cost.pips().hash(&mut hasher);
    pool.white.hash(&mut hasher);
    pool.blue.hash(&mut hasher);
    pool.black.hash(&mut hasher);
    pool.red.hash(&mut hasher);
    pool.green.hash(&mut hasher);
    pool.colorless.hash(&mut hasher);
    hasher.finish()
}

/// Order-independent structural digest of one collection element.
fn unordered_digest<T>(items: &[T], mut each: impl FnMut(&T, &mut DefaultHasher)) -> Vec<u64> {
    let mut digests = items
        .iter()
        .map(|item| {
            let mut hasher = DefaultHasher::new();
            each(item, &mut hasher);
            hasher.finish()
        })
        .collect::<Vec<_>>();
    digests.sort_unstable();
    digests
}

fn safe_search_state_key(game: &GameState, payer: crate::ids::PlayerId) -> u64 {
    let mut hasher = DefaultHasher::new();
    if let Some(player) = game.player(payer) {
        player.life.hash(&mut hasher);
        player.mana_pool.white.hash(&mut hasher);
        player.mana_pool.blue.hash(&mut hasher);
        player.mana_pool.black.hash(&mut hasher);
        player.mana_pool.red.hash(&mut hasher);
        player.mana_pool.green.hash(&mut hasher);
        player.mana_pool.colorless.hash(&mut hasher);
        // Hashed structurally rather than through `format!("{:?}")`: this key is
        // taken once per prepared candidate and once per queued node, and a
        // provenance entry carries a full `ObjectSnapshot` whose Debug output is
        // large. The snapshot itself is not hashed because within one search
        // every entry for a given (symbol, source) was produced by the same
        // activation of the same object, so the identity fields already
        // distinguish the states this dedup can encounter.
        unordered_digest(&player.restricted_mana, |unit, hasher| {
            unit.symbol.hash(hasher);
            unit.source.hash(hasher);
            unit.source_chosen_creature_type.hash(hasher);
            unit.restrictions.len().hash(hasher);
        })
        .hash(&mut hasher);
        unordered_digest(&player.mana_source_provenance, |unit, hasher| {
            unit.symbol.hash(hasher);
            unit.source.hash(hasher);
            unit.restricted.hash(hasher);
            unit.retention.hash(hasher);
            unit.snapshot.is_some().hash(hasher);
        })
        .hash(&mut hasher);
    }
    for id in &game.battlefield {
        id.hash(&mut hasher);
        game.is_tapped(*id).hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::hash_map::DefaultHasher;

    fn game() -> (GameState, PlayerId) {
        (
            GameState::new(vec!["Alice".to_string()], 20),
            PlayerId::from_index(0),
        )
    }

    fn request(
        game: &GameState,
        payer: PlayerId,
        source: ObjectId,
        cost: ManaCost,
    ) -> ManaPaymentRequest {
        ManaPaymentRequest::new(payer, source, crate::costs::PaymentReason::Effect, cost)
            .with_spend_policy(game.mana_spend_policy(payer, Some(source)))
    }

    fn restricted_mana_land(
        game: &mut GameState,
        owner: PlayerId,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let definition = CardBuilder::new(CardId::new(), "Restricted Font")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&definition, owner, Zone::Battlefield);
        let mut ability = crate::ability::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Green],
        );
        if let AbilityKind::Activated(activated) = &mut ability.kind {
            activated.mana_usage_restrictions =
                vec![crate::ability::ManaUsageRestriction::CastSpell {
                    card_types,
                    subtype_requirement: None,
                    restrict_to_matching_spell: true,
                    grant_uncounterable: false,
                    enters_with_counters: Vec::new(),
                    granted_abilities: Vec::new(),
                }];
        }
        game.object_mut(land).unwrap().abilities_mut().push(ability);
        land
    }

    fn cast_request(game: &GameState, payer: PlayerId, spell: ObjectId) -> ManaPaymentRequest {
        ManaPaymentRequest::new(
            payer,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        )
        .with_spend_policy(game.mana_spend_policy(payer, Some(spell)))
    }

    /// Builds a land whose mana ability produces `outputs`, optionally snow and
    /// optionally with an extra activation cost.
    fn mana_land(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        outputs: &[Vec<ManaSymbol>],
        snow: bool,
        // `Some(true)` adds a life cost alongside the tap; `Some(false)` is a
        // plain tap; `None` makes the ability free, and therefore repeatable.
        extra_cost: Option<bool>,
    ) -> ObjectId {
        let mut builder = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]);
        if snow {
            builder = builder.supertypes(vec![crate::types::Supertype::Snow]);
        }
        let definition = builder.build();
        let land = game.create_object_from_card(&definition, owner, Zone::Battlefield);
        for output in outputs {
            let cost = match extra_cost {
                Some(true) => crate::cost::TotalCost::from_costs(vec![
                    crate::costs::Cost::tap(),
                    crate::costs::Cost::life(1),
                ]),
                Some(false) => crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                None => crate::cost::TotalCost::free(),
            };
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(cost, output.clone()));
        }
        land
    }

    /// The affordability solver may only veto the planner when it cannot miss a
    /// resource the search would have found. This sweeps board shapes and costs
    /// and fails if the veto ever refuses a payment the full search can make.
    ///
    /// The veto is a one-way door: a "yes" from the solver decides nothing, so
    /// only false negatives can break payments, and that is what is asserted.
    #[test]
    fn affordability_veto_never_refuses_a_payment_the_search_can_find() {
        use crate::mana::ManaSymbol as M;
        let colors = [M::White, M::Blue, M::Black, M::Red, M::Green];
        let outputs_for = |kind: usize| -> Vec<Vec<ManaSymbol>> {
            match kind {
                0 => vec![vec![M::Green]],
                1 => vec![vec![M::Green], vec![M::Blue]],
                2 => colors.iter().map(|color| vec![*color]).collect(),
                3 => vec![vec![M::Colorless]],
                _ => vec![vec![M::Green, M::Green]],
            }
        };
        let costs = [
            ManaCost::from_pips(vec![vec![M::Green]]),
            ManaCost::from_pips(vec![vec![M::Blue], vec![M::Blue]]),
            ManaCost::from_pips(vec![vec![M::Generic(3)]]),
            ManaCost::from_pips(vec![vec![M::Generic(2)], vec![M::Blue], vec![M::Blue]]),
            ManaCost::from_pips(vec![vec![M::Green, M::Blue], vec![M::Generic(1)]]),
            ManaCost::from_pips(vec![vec![M::Snow], vec![M::Green]]),
            ManaCost::from_pips(vec![vec![M::Blue, M::Life(2)]]),
            ManaCost::from_pips(vec![vec![M::Colorless], vec![M::Generic(1)]]),
            ManaCost::from_pips(vec![vec![M::Generic(6)]]),
        ];
        let mut vetoed = 0usize;
        let mut checked = 0usize;
        for kind in 0..5usize {
            for lands in [0usize, 1, 2, 3, 5] {
                for snow in [false, true] {
                    for extra_cost in [Some(false), Some(true), None] {
                        for pool_green in [0u32, 1] {
                            for (cost_index, cost) in costs.iter().enumerate() {
                                let (mut game, alice) = game();
                                for index in 0..lands {
                                    mana_land(
                                        &mut game,
                                        alice,
                                        &format!("Land {index}"),
                                        &outputs_for(kind),
                                        snow,
                                        extra_cost,
                                    );
                                }
                                if pool_green > 0 {
                                    game.player_mut(alice)
                                        .unwrap()
                                        .mana_pool
                                        .add(M::Green, pool_green);
                                }
                                game.refresh_continuous_state();
                                let source = game.new_object_id();
                                let request = request(&game, alice, source, cost.clone());

                                let veto = affordability_rules_out_payment(&game, &request);
                                let searched = ManaPaymentPlanner {
                                    skip_affordability_gate: true,
                                    ..Default::default()
                                }
                                .plan(&game, &request);
                                checked += 1;
                                if veto {
                                    vetoed += 1;
                                    assert!(
                                        searched.is_err(),
                                        "affordability veto refused a payment the search found: \
                                         kind={kind} lands={lands} snow={snow} \
                                         extra_cost={extra_cost:?} pool_green={pool_green} \
                                         cost_index={cost_index}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 500, "matrix should be broad, checked {checked}");
        assert!(
            vetoed > 50,
            "matrix should exercise the veto, vetoed {vetoed}"
        );
    }

    /// Convoke, Delve and Improvise pay pips without producing mana, so the
    /// solver cannot see them and must not be allowed to veto those casts.
    #[test]
    fn keyword_payments_are_never_vetoed_by_the_affordability_solver() {
        for keyword in ["Convoke", "Delve", "Improvise"] {
            let (mut game, alice) = game();
            let definition = CardBuilder::new(CardId::new(), format!("{keyword} Spell"))
                .card_types(vec![CardType::Sorcery])
                .build();
            let spell = game.create_object_from_card(&definition, alice, Zone::Stack);
            game.object_mut(spell).unwrap().abilities_mut().push(
                crate::ability::Ability::static_ability(match keyword {
                    "Convoke" => crate::static_abilities::StaticAbility::new(
                        crate::static_abilities::Convoke,
                    ),
                    "Delve" => {
                        crate::static_abilities::StaticAbility::new(crate::static_abilities::Delve)
                    }
                    _ => crate::static_abilities::StaticAbility::new(
                        crate::static_abilities::Improvise,
                    ),
                }),
            );
            game.refresh_continuous_state();
            let mut request = ManaPaymentRequest::new(
                alice,
                spell,
                crate::costs::PaymentReason::CastSpell,
                ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
            );
            request.spend_policy = game.mana_spend_policy(alice, Some(spell));
            assert!(
                !affordability_solver_sees_every_resource(&game, &request),
                "{keyword} must disable the affordability veto"
            );
        }
    }

    /// Reserved resources are spendable by the planner and invisible to the
    /// solver, so they must disable the veto too.
    #[test]
    fn reserved_resources_disable_the_affordability_veto() {
        let (mut game, alice) = game();
        let land = mana_land(
            &mut game,
            alice,
            "Reserved",
            &[vec![ManaSymbol::Green]],
            false,
            Some(false),
        );
        game.refresh_continuous_state();
        let source = game.new_object_id();
        let base = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        );
        assert!(affordability_solver_sees_every_resource(&game, &base));

        let mut graveyard = base.clone();
        graveyard.reserved_graveyard_sources.push(land);
        assert!(!affordability_solver_sees_every_resource(&game, &graveyard));

        let mut permanent = base.clone();
        permanent.reserved_permanent_sources.push(land);
        assert!(!affordability_solver_sees_every_resource(&game, &permanent));
    }

    /// The continuous-state cache is only retained across a staged mana
    /// activation when nothing can observe a tap. This pins the classifier that
    /// decides it, because a wrong "insensitive" answer would let the search
    /// read stale continuous state.
    #[test]
    fn tap_sensitivity_recognizes_effects_that_read_tapped_state() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};

        let (mut game, alice) = game();
        mana_land(
            &mut game,
            alice,
            "Plain",
            &[vec![ManaSymbol::Green]],
            false,
            Some(false),
        );
        game.refresh_continuous_state();
        assert!(
            !game.continuous_effects_are_tap_sensitive(),
            "a board of plain lands has nothing that reads tapped state"
        );

        let mut tapped_filter = crate::filter::ObjectFilter::default();
        tapped_filter.tapped = true;
        let source = game.new_object_id();
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Filter(tapped_filter),
                Modification::ModifyPower(1),
            ));
        assert!(
            game.continuous_effects_are_tap_sensitive(),
            "an effect whose filter reads tapped state must force the recompute"
        );
    }

    /// Collapsing interchangeable sources may change which source a plan names,
    /// but never how good the plan is or whether one exists.
    #[test]
    fn collapsing_interchangeable_sources_preserves_plan_quality() {
        use crate::mana::ManaSymbol as M;
        for lands in [2usize, 4, 6] {
            for cost in [
                ManaCost::from_pips(vec![vec![M::Green]]),
                ManaCost::from_pips(vec![vec![M::Green], vec![M::Green]]),
                ManaCost::from_pips(vec![vec![M::Generic(3)]]),
            ] {
                let (mut game, alice) = game();
                for index in 0..lands {
                    mana_land(
                        &mut game,
                        alice,
                        &format!("Forest {index}"),
                        &[vec![M::Green]],
                        false,
                        Some(false),
                    );
                }
                game.refresh_continuous_state();
                let source = game.new_object_id();
                let request = request(&game, alice, source, cost.clone());
                let all = collect_activation_choices(&game, &request);
                let collapsed = collect_search_choices(&game, &request);
                assert!(
                    collapsed.len() <= all.len(),
                    "collapsing must not invent choices"
                );
                if lands > 1 {
                    assert!(
                        collapsed.len() < all.len(),
                        "identical forests should collapse (lands={lands})"
                    );
                }
                let plan = plan_mana_payment(&game, &request);
                assert_eq!(
                    plan.is_ok(),
                    lands >= cost.mana_value() as usize,
                    "payability must not change (lands={lands})"
                );
                if let Ok(plans) = plan {
                    assert_eq!(
                        plans[0].mana_ability_steps.len(),
                        cost.mana_value() as usize,
                        "a collapsed plan still taps one source per pip"
                    );
                }
            }
        }
    }

    /// Slicing must still hand control back mid-search for boards the
    /// assignment declines, so a long plan cannot block a frame.
    #[test]
    fn sliced_search_still_yields_before_finishing_when_it_cannot_be_assigned() {
        let (mut game, alice) = game();
        for _ in 0..4 {
            let definition = CardBuilder::new(CardId::new(), "Sacrificial Font")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_costs(vec![
                        crate::costs::Cost::tap(),
                        crate::costs::Cost::life(1),
                    ]),
                    vec![ManaSymbol::Green],
                ));
        }
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green], vec![ManaSymbol::Green]]),
        );
        let mut analysis = ManaPaymentAnalysis::new(&game, request);
        assert!(
            analysis.step(1).is_none(),
            "a searched board must not finish inside a single slice"
        );
    }

    /// Restricted mana that cannot pay for the pending spell must not be
    /// expanded: each colour costs a full `GameState` clone to simulate.
    #[test]
    fn restricted_mana_unusable_for_the_pending_spell_is_not_offered() {
        let (mut game, alice) = game();
        restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        let instant = CardBuilder::new(CardId::new(), "Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let spell = game.create_object_from_card(&instant, alice, Zone::Stack);
        let request = cast_request(&game, alice, spell);
        assert!(
            collect_activation_choices(&game, &request).is_empty(),
            "creature-only mana must not be explored for an instant spell"
        );
    }

    /// The same source stays available when its restriction is satisfied, so
    /// pruning never removes a payment the player could actually make.
    #[test]
    fn restricted_mana_usable_for_the_pending_spell_is_still_offered() {
        let (mut game, alice) = game();
        let land = restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        let creature = CardBuilder::new(CardId::new(), "Test Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let spell = game.create_object_from_card(&creature, alice, Zone::Stack);
        let request = cast_request(&game, alice, spell);
        let choices = collect_activation_choices(&game, &request);
        assert!(
            choices.iter().any(|choice| choice.source == land),
            "creature-only mana must stay available for a creature spell"
        );
    }

    #[test]
    fn sliced_first_plan_matches_synchronous_search_and_preserves_live_state() {
        let (mut game, alice) = game();
        for _ in 0..4 {
            let definition = CardBuilder::new(CardId::new(), "Test Forest")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![ManaSymbol::Green],
                ));
        }
        let source = game.new_object_id();
        for symbol in [ManaSymbol::Green, ManaSymbol::Black] {
            let request = request(
                &game,
                alice,
                source,
                ManaCost::from_pips(vec![vec![symbol], vec![symbol]]),
            );
            let expected = plan_first_mana_payment(&game, &request).map(|plan| plan.id);
            let mut analysis = ManaPaymentAnalysis::new(&game, request);
            // A tap-only board is answered by the assignment, which does not
            // spend search budget, so this may now settle on the first slice.
            // What must still hold is that slicing reaches the same plan.
            let mut slices = 0;
            let actual = loop {
                slices += 1;
                assert!(slices < 10000);
                if let Some(result) = analysis.step(1) {
                    break result.map(|plan| plan.id);
                }
            };
            assert_eq!(actual, expected);
            assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        }
    }

    #[test]
    fn pool_preview_and_commit_use_the_same_allocation() {
        let (mut game, alice) = game();
        let source = game.new_object_id();
        game.player_mut(alice).unwrap().mana_pool.red = 1;
        let request = request(&game, alice, source, ManaCost::new().add_generic(1));
        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);

        assert!(matches!(
            plan.allocations.as_slice(),
            [PlannedPipAllocation {
                payment: super::super::PlannedPipPayment::Mana(ManaSymbol::Red),
                ..
            }]
        ));
        assert_eq!(plan.expected_pool_after_payment.total(), 0);

        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut dm),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn improvise_preview_skips_unfundable_subsets_and_uses_assignment() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Improvise probe")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::improvise(),
            ),
        );
        for _ in 0..8 {
            game.create_object_from_card(&card, alice, Zone::Battlefield);
        }
        for _ in 0..2 {
            let land = CardBuilder::new(CardId::new(), "Blue land")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&land, alice, Zone::Battlefield);
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![ManaSymbol::Blue],
                ));
        }
        game.refresh_continuous_state();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Blue]]),
        );
        let start = std::time::Instant::now();
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        let perf = last_mana_payment_perf();
        eprintln!("Improvise first plan: {:?}, {perf:?}", start.elapsed());
        assert!(plan.payable);
        assert_eq!(plan.mana_ability_steps.len(), 2);
        assert_eq!(
            perf.searched_selections, 0,
            "impossible subsets must not launch state searches"
        );
        assert_eq!(perf.analytic_selections, 1);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
        let mut ranked = ManaPaymentAnalysis::ranked(&game, request.clone());
        assert!(ranked.step(1).is_none());
        let mut slices = 0;
        let improved = loop {
            slices += 1;
            assert!(slices < 1000);
            if let Some(result) = ranked.step(1) {
                break result.unwrap();
            }
        };
        assert!(improved.score <= plan.score);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
    }

    #[test]
    fn first_plan_returns_a_legal_preview_without_waiting_for_all_candidates() {
        let (mut game, alice) = game();
        let source = game.new_object_id();
        game.player_mut(alice).unwrap().mana_pool.red = 1;
        let request = request(&game, alice, source, ManaCost::new().add_generic(1));

        let first = plan_first_mana_payment(&game, &request).unwrap();
        let all = plan_mana_payment(&game, &request).unwrap();

        assert!(all.iter().any(|candidate| candidate.id == first.id));
        assert_eq!(first.expected_pool_after_payment.total(), 0);
    }

    #[test]
    fn affordability_skips_sibling_simulations_and_backtracks_when_needed() {
        let (mut game, alice) = game();
        for _ in 0..32 {
            let definition = CardBuilder::new(CardId::new(), "Test Forest")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![ManaSymbol::Green],
                ));
        }
        let source = game.new_object_id();
        let request = request(&game, alice, source, ManaCost::new().add_generic(1));
        let mut search = CandidateSearch::new(game.clone(), &request, 2, true, true);
        // Visit the root, simulate one activation, then accept that child.
        // Eager sibling preparation cannot finish within these three work units.
        let candidates = search.step(&request, &mut 3).unwrap().unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].1.len(), 1);
        assert!(can_pay_request(&candidates[0].0, &request));
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));

        // A failed first branch must not discard the remaining candidates.
        let required = *game.battlefield.last().unwrap();
        let mut request = request;
        request.preferences.required_sources.push(required);
        let mut search = CandidateSearch::new(game.clone(), &request, 1, true, true);
        let mut slices = 0;
        let candidates = loop {
            slices += 1;
            assert!(slices < 1000);
            if let Some(result) = search.step(&request, &mut 1) {
                break result.unwrap();
            }
        };
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].1[0].source, required);
        assert!(check_mana_payment(&game, &request).is_ok());
    }

    #[test]
    fn prefer_life_changes_both_preview_and_commit() {
        let (mut game, alice) = game();
        let source = game.new_object_id();
        game.player_mut(alice).unwrap().mana_pool.black = 1;
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Black, ManaSymbol::Life(2)]]),
        );
        request.preferences.prefer_life = true;
        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);

        assert_eq!(plan.life_to_pay, 2);
        assert!(matches!(
            plan.allocations[0].payment,
            super::super::PlannedPipPayment::Life(2)
        ));
        assert_eq!(plan.expected_pool_after_payment.black, 1);

        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut dm),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().life, 18);
        assert_eq!(game.player(alice).unwrap().mana_pool.black, 1);
    }

    #[test]
    fn safe_mana_source_is_preferred_to_life_unless_life_is_requested() {
        let (mut game, alice) = game();
        let land = CardBuilder::new(CardId::new(), "Test Swamp")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land, alice, Zone::Battlefield);
        game.object_mut(land)
            .expect("land should exist")
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                vec![ManaSymbol::Black],
            ));
        let source = game.new_object_id();
        let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Black, ManaSymbol::Life(2)]]);

        let mana_request = request(&game, alice, source, cost.clone());
        let mana_plan = plan_mana_payment(&game, &mana_request).unwrap().remove(0);
        assert_eq!(mana_plan.life_to_pay, 0);
        assert_eq!(mana_plan.mana_ability_steps.len(), 1);
        assert_eq!(mana_plan.mana_ability_steps[0].source, land);

        let mut life_request = request(&game, alice, source, cost);
        life_request.preferences.prefer_life = true;
        let life_plan = plan_mana_payment(&game, &life_request).unwrap().remove(0);
        assert_eq!(life_plan.life_to_pay, 2);
        assert!(life_plan.mana_ability_steps.is_empty());
    }

    #[test]
    fn colored_pip_does_not_activate_unrelated_sources_before_the_matching_land() {
        let (mut game, alice) = game();
        let mut add_land = |name: &str, symbol: ManaSymbol| {
            let definition = CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![symbol],
                ));
            land
        };
        let _forest = add_land("Test Forest", ManaSymbol::Green);
        let _plains = add_land("Test Plains", ManaSymbol::White);
        let island = add_land("Test Island", ManaSymbol::Blue);
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );

        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);

        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert_eq!(plan.mana_ability_steps[0].source, island);
        assert_eq!(plan.expected_pool_after_payment.total(), 0);
        assert_eq!(plan.score.excess_mana, 0);
    }

    #[test]
    fn full_search_stops_when_a_basic_land_plan_reaches_the_score_floor() {
        let (mut game, alice) = game();
        for (name, symbol) in [
            ("Test Plains", ManaSymbol::White),
            ("Test Island", ManaSymbol::Blue),
            ("Test Swamp", ManaSymbol::Black),
            ("Test Mountain", ManaSymbol::Red),
            ("Test Forest", ManaSymbol::Green),
        ] {
            let definition = CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![symbol],
                ));
        }
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![
                vec![ManaSymbol::White],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Green],
            ]),
        );
        let mut planner = ManaPaymentPlanner::default();

        let plan = planner
            .plan_internal(&game, &request, false)
            .unwrap()
            .remove(0);

        assert_eq!(
            plan.score,
            ManaPaymentScore {
                source_count: 5,
                ..ManaPaymentScore::default()
            }
        );
        assert!(planner.visited_nodes < 64);
    }

    #[test]
    fn repeated_colored_pips_use_only_matching_sources() {
        let (mut game, alice) = game();
        let mut add_land = |name: &str, symbol: ManaSymbol| {
            let definition = CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![symbol],
                ));
            land
        };
        let _forest = add_land("Test Forest", ManaSymbol::Green);
        let first_island = add_land("Test Island One", ManaSymbol::Blue);
        let second_island = add_land("Test Island Two", ManaSymbol::Blue);
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]),
        );

        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);
        let planned_sources = plan
            .mana_ability_steps
            .iter()
            .map(|activation| activation.source)
            .collect::<HashSet<_>>();

        assert_eq!(plan.mana_ability_steps.len(), 2);
        assert_eq!(
            planned_sources,
            HashSet::from([first_island, second_island])
        );
        assert_eq!(plan.expected_pool_after_payment.total(), 0);
        assert_eq!(plan.score.excess_mana, 0);
    }

    #[test]
    fn repeated_colored_pips_skip_unrelated_basics_for_flexible_lands() {
        let (mut game, alice) = game();
        let mut add_basic = |name: &str, symbol: ManaSymbol| {
            let definition = CardBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![symbol],
                ));
            land
        };
        let _forest = add_basic("Test Forest", ManaSymbol::Green);
        let _plains = add_basic("Test Plains", ManaSymbol::White);
        let _mountain = add_basic("Test Mountain", ManaSymbol::Red);
        let _swamp = add_basic("Test Swamp", ManaSymbol::Black);
        let island = add_basic("Test Island", ManaSymbol::Blue);
        game.tap(island);

        let flexible_land = |name: &str, colors: [Color; 2]| {
            crate::cards::CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .with_ability(crate::ability::Ability::mana_with_effects(
                    crate::cost::TotalCost::free(),
                    vec![crate::effect::Effect::add_mana_of_any_color_restricted(
                        1,
                        colors.to_vec(),
                    )],
                ))
                .build()
        };
        let tropical = flexible_land("Test Tropical Island", [Color::Green, Color::Blue]);
        let tropical = game.create_object_from_definition(&tropical, alice, Zone::Battlefield);
        let volcanic = flexible_land("Test Volcanic Island", [Color::Red, Color::Blue]);
        let volcanic = game.create_object_from_definition(&volcanic, alice, Zone::Battlefield);
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]),
        );

        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);
        let planned_sources = plan
            .mana_ability_steps
            .iter()
            .map(|activation| activation.source)
            .collect::<HashSet<_>>();

        assert_eq!(plan.mana_ability_steps.len(), 2);
        assert_eq!(planned_sources, HashSet::from([tropical, volcanic]));
        assert_eq!(plan.expected_pool_after_payment.total(), 0);
        assert_eq!(plan.score.excess_mana, 0);
    }

    #[test]
    fn bounded_search_handles_large_generic_cost_without_permutation_explosion() {
        let (mut game, alice) = game();
        for index in 0..10 {
            let definition = CardBuilder::new(CardId::new(), format!("Test Land {index}"))
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&definition, alice, Zone::Battlefield);
            game.object_mut(land)
                .expect("land should exist")
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![ManaSymbol::Colorless],
                ));
        }
        let source = game.new_object_id();
        let request = request(&game, alice, source, ManaCost::new().add_generic(8));

        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);

        assert_eq!(plan.mana_ability_steps.len(), 8);
        assert_eq!(plan.expected_pool_after_payment.total(), 0);
        assert_eq!(plan.score.excess_mana, 0);
    }

    #[test]
    fn reserved_cost_resources_are_not_spent_by_mana_abilities() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Reserved resource")
            .card_types(vec![CardType::Creature])
            .build();
        let creature = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.remove_summoning_sickness(creature);
        game.object_mut(creature)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                vec![ManaSymbol::Green],
            ));
        let source = game.new_object_id();
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_symbols(vec![ManaSymbol::Green]),
        );
        request.reserved_tap_sources.push(creature);
        assert!(
            plan_mana_payment(&game, &request).is_err(),
            "Harmonize cannot share a tap with a mana ability"
        );
        request.reserved_tap_sources.clear();
        request.reserved_permanent_sources.push(creature);
        assert!(
            plan_mana_payment(&game, &request).is_ok(),
            "an Emerge or Offering sacrifice may tap for mana first"
        );
        game.object_mut(creature).unwrap().abilities_mut().clear();
        game.object_mut(creature)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::sacrifice_self()),
                vec![ManaSymbol::Green],
            ));
        assert!(
            plan_mana_payment(&game, &request).is_err(),
            "a mana ability cannot consume a reserved sacrifice"
        );
    }

    #[test]
    fn convoke_is_a_planned_pip_allocation() {
        let (mut game, alice) = game();
        let creature = CardBuilder::new(CardId::new(), "Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(), "Convoke Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell = game.create_object_from_card(&spell, alice, Zone::Stack);
        game.object_mut(spell)
            .expect("spell should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::convoke(),
            ));
        let mut request = request(&game, alice, spell, ManaCost::new().add_generic(1));
        request.reason = crate::costs::PaymentReason::CastSpell;

        let plan = plan_mana_payment(&game, &request).unwrap().remove(0);
        assert!(matches!(
            plan.allocations[0].payment,
            super::super::PlannedPipPayment::Convoke(source) if source == creature
        ));
        assert!(plan.mana_cost_after_alternatives.is_empty());
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert!(
            game.is_tapped(creature),
            "planned convoke must actually tap its resource"
        );
    }

    #[test]
    fn delve_selects_exact_cards_and_exiles_only_on_commit() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Graveyard card").build();
        let first = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let second = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::delve()));
        let mut request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::new().add_generic(1),
        );
        request
            .preferences
            .required_alternatives
            .push(super::super::RequiredAlternativePayment {
                source: first,
                kind: ManaPaymentSourceKind::Delve,
            });
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
        assert!(
            matches!(plan.allocations[0].payment, super::super::PlannedPipPayment::Delve(id) if id == first)
        );
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().graveyard.as_slice(), &[second]);
        assert_eq!(game.exile.len(), 1);
    }

    #[test]
    fn delve_cannot_pay_colored_or_colorless_pips_or_exile_the_spell_itself() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Delve card").build();
        let spell = game.create_object_from_card(&card, alice, Zone::Graveyard);
        game.object_mut(spell).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::delve()));
        let mut request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::new().add_generic(1),
        );
        assert!(plan_first_mana_payment(&game, &request).is_err());
        game.create_object_from_card(&card, alice, Zone::Graveyard);
        for symbol in [ManaSymbol::Blue, ManaSymbol::Colorless] {
            request.cost = ManaCost::from_pips(vec![vec![symbol]]);
            assert!(plan_first_mana_payment(&game, &request).is_err());
        }
    }

    #[test]
    fn large_delve_payment_is_not_lost_to_equivalent_pip_permutations() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Delve resource").build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::delve()));
        for _ in 0..30 {
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::new().add_generic(12),
        );
        let plan = plan_first_mana_payment(&game, &request)
            .expect("twelve graveyard cards can cover twelve generic pips");
        assert_eq!(plan.allocations.len(), 12);
        assert!(plan.mana_cost_after_alternatives.is_empty());
    }

    #[test]
    fn artifact_creature_cannot_pay_twice_and_can_choose_improvise() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Artifact helper")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .build();
        let resource = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        for ability in [
            crate::static_abilities::StaticAbility::convoke(),
            crate::static_abilities::StaticAbility::improvise(),
        ] {
            game.object_mut(spell)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::static_ability(ability));
        }
        let mut request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::new().add_generic(2),
        );
        assert!(plan_first_mana_payment(&game, &request).is_err());
        request.cost = ManaCost::new().add_generic(1);
        request
            .preferences
            .required_alternatives
            .push(super::super::RequiredAlternativePayment {
                source: resource,
                kind: ManaPaymentSourceKind::Improvise,
            });
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert!(
            matches!(plan.allocations[0].payment, super::super::PlannedPipPayment::Improvise(id) if id == resource)
        );
    }

    #[test]
    fn conflicting_source_constraints_are_rejected() {
        let (game, alice) = game();
        let source = ObjectId::from_raw(99);
        let mut request = request(&game, alice, source, ManaCost::new());
        request.preferences.required_sources.push(source);
        request.preferences.excluded_sources.push(source);
        assert_eq!(
            plan_mana_payment(&game, &request),
            Err(ManaPaymentFailure::ConflictingPreferences)
        );
    }

    #[test]
    fn exact_activation_constraint_preserves_the_selected_ability() {
        let (mut game, alice) = game();
        let land = CardBuilder::new(CardId::new(), "Two-Mode Land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land, alice, Zone::Battlefield);
        let abilities = game
            .object_mut(land)
            .expect("land should exist")
            .abilities_mut();
        abilities.push(crate::ability::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Blue],
        ));
        abilities.push(crate::ability::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Red],
        ));

        let source = game.new_object_id();
        let mut request = request(&game, alice, source, ManaCost::new().add_generic(1));
        request
            .preferences
            .required_activations
            .push(super::super::RequiredManaActivation {
                source: land,
                ability_index: 1,
                color_restriction: None,
            });

        let plan = plan_mana_payment(&game, &request)
            .expect("the selected ability should remain payable")
            .remove(0);
        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert_eq!(plan.mana_ability_steps[0].source, land);
        assert_eq!(plan.mana_ability_steps[0].ability_index, 1);
        assert_eq!(plan.mana_ability_steps[0].expected_mana.red, 1);
    }

    #[test]
    fn exact_activation_constraints_preserve_repeat_count() {
        let (mut game, alice) = game();
        let source_card = CardBuilder::new(CardId::new(), "Repeatable Mana Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let mana_source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(mana_source)
            .expect("source should exist")
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: crate::ability::AbilityKind::Activated(
                    crate::ability::ActivatedAbility::mana_with_costs(
                        crate::cost::TotalCost::free(),
                        vec![],
                        vec![ManaSymbol::Blue],
                    ),
                ),
                functional_zones: vec![Zone::Battlefield],
            });
        let source = game.new_object_id();
        let mut request = request(&game, alice, source, ManaCost::new().add_generic(2));
        let selected = super::super::RequiredManaActivation {
            source: mana_source,
            ability_index: 0,
            color_restriction: None,
        };
        request.preferences.required_activations = vec![selected.clone(), selected];
        assert!(
            mana_payment_activation_inventory(&game, &request)
                .iter()
                .any(|option| option.source == mana_source && option.repeatable),
            "the test source must be legally repeatable"
        );

        let plan = plan_mana_payment(&game, &request)
            .expect("both selected activations should remain payable")
            .remove(0);
        assert_eq!(plan.mana_ability_steps.len(), 2);
        assert!(
            plan.mana_ability_steps
                .iter()
                .all(|step| step.source == mana_source && step.ability_index == 0)
        );
    }

    #[test]
    fn exact_alternative_constraint_preserves_convoke_selection() {
        let (mut game, alice) = game();
        let creature = CardBuilder::new(CardId::new(), "Selected Helper")
            .card_types(vec![CardType::Creature])
            .build();
        let creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let spell = CardBuilder::new(CardId::new(), "Selected Convoke Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell = game.create_object_from_card(&spell, alice, Zone::Stack);
        game.object_mut(spell)
            .expect("spell should exist")
            .abilities_mut()
            .push(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::convoke(),
            ));
        let mut request = request(&game, alice, spell, ManaCost::new().add_generic(1));
        request.reason = crate::costs::PaymentReason::CastSpell;
        request
            .preferences
            .required_alternatives
            .push(super::super::RequiredAlternativePayment {
                source: creature,
                kind: ManaPaymentSourceKind::Convoke,
            });

        let plan = plan_mana_payment(&game, &request)
            .expect("the selected convoke source should remain payable")
            .remove(0);
        assert!(matches!(
            plan.allocations[0].payment,
            super::super::PlannedPipPayment::Convoke(source) if source == creature
        ));
    }

    #[test]
    fn excluded_exact_activation_is_a_conflicting_preference() {
        let (game, alice) = game();
        let source = ObjectId::from_raw(101);
        let mut request = request(&game, alice, source, ManaCost::new());
        request
            .preferences
            .required_activations
            .push(super::super::RequiredManaActivation {
                source,
                ability_index: 0,
                color_restriction: None,
            });
        request.preferences.excluded_sources.push(source);
        assert_eq!(
            plan_mana_payment(&game, &request),
            Err(ManaPaymentFailure::ConflictingPreferences)
        );
    }
}
