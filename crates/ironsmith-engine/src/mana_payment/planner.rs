use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

use crate::ability::{AbilityKind, ActivatedAbilityRuntimeExt as _};
use crate::color::Color;
use crate::decision::{DecisionMaker as _, SelectFirstDecisionMaker};
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

// Proposal ranking may stop only after an executable witness exists. This
// limit never turns an incomplete existence search into negative legality.
const MAX_RANKING_SEARCH_NODES: usize = 4_096;
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

/// Expanded, ordered pips shared by allocation IDs and the payment editor.
pub fn mana_payment_expanded_pips(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<Vec<ManaSymbol>> {
    let black_life = request.allow_black_life
        && game.player_can_pay_black_with_life_for_reason(
            request.payer,
            Some(request.source),
            request.reason,
        );
    GameState::expanded_payment_pips(&request.cost, request.x_value, black_life)
}

/// Individually selectable life alternatives, identified in the expanded cost.
pub fn mana_payment_life_options(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<(ManaPipId, u32)> {
    if !request.allow_life_payment || request.preferences.prefer_life {
        return Vec::new();
    }
    mana_payment_expanded_pips(game, request)
        .iter()
        .enumerate()
        .filter_map(|(index, pip)| {
            let id = ManaPipId(index as u32);
            if pip.len() < 2 || request.preferences.required_life_pips.contains(&id) {
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
/// Existence checks first try projected sources without simulating siblings,
/// then follow one lazy search line. Both stop at the first legal completion.
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

/// Scratch-only inventory evaluation. A failed probe is unknown, never an
/// unavailable source; checked callers retain the original execution error.
pub(super) fn with_inventory_query<T>(
    game: &GameState,
    read: impl FnOnce(&GameState) -> T,
) -> Result<T, crate::effects::ExecutionError> {
    let scope =
        crate::effects::tokens::resources::TokenQueryScope::new(game.token_creation_limits());
    let meter = scope.meter();
    let mut query_game = game.clone();
    query_game.bind_token_query_meter(meter.clone());
    let result = read(&query_game);
    if let Some(error) = crate::effects::tokens::resources::failure(&meter) {
        game.record_token_resource_failure(&error);
        return Err(error);
    }
    Ok(result)
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
        && request.assist_completion.is_none()
        && let Some(spell) = game.object(request.source)
        && game.controller_of(spell) == request.payer
    {
        if crate::decision::spell_has_convoke(game, spell) {
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
        if crate::decision::spell_has_delve(game, spell) {
            for source in delve_cards(game, request) {
                by_source
                    .entry(source)
                    .or_default()
                    .push(ManaPaymentSourceKind::Delve);
            }
        }
        if crate::decision::spell_has_improvise(game, spell) {
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
/// payment client. Reviewed sources are projected; complex abilities are
/// simulated on scratch state. No live state is mutated.
pub fn mana_payment_activation_inventory(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<ManaPaymentActivationOption> {
    mana_payment_activation_inventory_checked(game, request).unwrap_or_default()
}

pub fn mana_payment_activation_inventory_checked(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<Vec<ManaPaymentActivationOption>, crate::effects::ExecutionError> {
    with_inventory_query(game, |query| {
        activation_inventory(query, request, false, || SelectFirstDecisionMaker)
    })
}

/// Deferred payment options must resolve without an unanswered player choice.
/// Reuse the same output calculation as the planner rather than simulating the
/// activation again in each UI adapter. Complex abilities use the caller's
/// authoritative prompt-only chooser; manual activation remains available.
pub fn mana_payment_ready_activation_inventory<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    chooser: impl FnMut() -> D,
) -> Vec<ManaPaymentActivationOption> {
    mana_payment_ready_activation_inventory_checked(game, request, chooser).unwrap_or_default()
}

pub fn mana_payment_ready_activation_inventory_checked<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    chooser: impl FnMut() -> D,
) -> Result<Vec<ManaPaymentActivationOption>, crate::effects::ExecutionError> {
    with_inventory_query(game, |query| {
        activation_inventory(query, request, true, chooser)
    })
}

/// Compute both UI inventories while sharing resolved tap-only activations.
/// The prompt-only inventory cannot replace manual analysis of costs that may
/// use interactive mana payment or of effects that still require a choice.
pub fn mana_payment_ready_and_manual_inventory<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    chooser: impl FnMut() -> D,
) -> (Vec<ManaPaymentActivationOption>, Vec<(ObjectId, usize)>) {
    mana_payment_ready_and_manual_inventory_checked(game, request, chooser).unwrap_or_default()
}

pub fn mana_payment_ready_and_manual_inventory_checked<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    chooser: impl FnMut() -> D,
) -> Result<
    (Vec<ManaPaymentActivationOption>, Vec<(ObjectId, usize)>),
    crate::effects::ExecutionError,
> {
    with_inventory_query(game, |query| {
        ready_and_manual_inventory_inner(query, request, chooser)
    })
}

fn ready_and_manual_inventory_inner<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    chooser: impl FnMut() -> D,
) -> (Vec<ManaPaymentActivationOption>, Vec<(ObjectId, usize)>) {
    let mut ready_request = request.clone();
    ready_request.preferences = Default::default();
    let before = game.covered_mana_payment_pips(request);
    let mut resolved = Vec::new();
    let options =
        activation_inventory_observing(game, &ready_request, true, chooser, |choice, staged| {
            let Some(ability) = game.current_ability(choice.source, choice.ability_index) else {
                return;
            };
            let AbilityKind::Activated(activated) = &ability.kind else {
                return;
            };
            let cost = crate::decision::calculate_effective_activation_total_cost_for_ability(
                game,
                request.payer,
                choice.source,
                &activated.mana_cost,
                &[],
                Some(crate::decision::ActivationCostAbility::of(
                    game,
                    request.payer,
                    choice.source,
                    activated,
                )),
            );
            // Interactive exclusions affect mana costs, not a sole tap cost.
            // Use the effective cost so activation taxes cannot bypass this guard.
            if cost
                .as_all()
                .is_some_and(|costs| costs.len() == 1 && costs[0].requires_tap())
            {
                resolved.push((
                    choice.source,
                    choice.ability_index,
                    choice.color_restriction.clone(),
                    staged.covered_mana_payment_pips(request) > before,
                ));
            }
        });
    let manual = if request.allow_mana_abilities {
        useful_manual_mana_abilities_with_resolved(game, request, true, &resolved)
    } else {
        Vec::new()
    };
    (options, manual)
}

type ResolvedManualActivation = (ObjectId, usize, Option<Vec<Color>>, bool);

fn activation_inventory<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    ready_only: bool,
    chooser: impl FnMut() -> D,
) -> Vec<ManaPaymentActivationOption> {
    activation_inventory_observing(game, request, ready_only, chooser, |_, _| {})
}

fn activation_inventory_observing<D: crate::decision::DecisionMaker>(
    game: &GameState,
    request: &ManaPaymentRequest,
    ready_only: bool,
    mut chooser: impl FnMut() -> D,
    mut resolved: impl FnMut(&ActivationChoice, &GameState),
) -> Vec<ManaPaymentActivationOption> {
    let mut unconstrained = request.clone();
    unconstrained.preferences.excluded_sources.clear();
    let analysis = super::sources::ManaSourceAnalysis::new(game);
    collect_raw_activation_choices_with_view(game, &unconstrained, false, &analysis.view)
        .into_iter()
        .filter_map(|choice| {
            let (expected_mana, max_activations) = if let Some(projected) =
                analysis.project(&choice)
            {
                if ready_only && projected.needs_choice {
                    return None;
                }
                (projected.output, 1)
            } else {
                let mut staged = game.clone();
                let before = staged.player(unconstrained.payer)?.mana_pool.clone();
                let mut decision_maker = chooser();
                activate_with_mana_triggers(
                    &mut staged,
                    unconstrained.payer,
                    choice.source,
                    choice.ability_index,
                    choice.color_restriction.clone(),
                    &mut decision_maker,
                )
                .ok()?;
                if decision_maker.awaiting_choice() {
                    return None;
                }
                let after = staged.player(unconstrained.payer)?.mana_pool.clone();
                resolved(&choice, &staged);
                // Probe the ordinary legality/cost path, including per-turn
                // limits, life, counters, sacrifices and state-based actions.
                // A successful second activation does not imply unlimited uses.
                let probe_limit = expanded_pip_count(request)
                    .saturating_add(MAX_EXTRA_ACTIVATIONS)
                    .max(request.preferences.required_activations.len())
                    .max(2);
                let mut max_activations = 1;
                while max_activations < probe_limit {
                    let legal = {
                        let view = DerivedGameView::new(&staged);
                        view.abilities_rc(choice.source)
                            .and_then(|abilities| abilities.get(choice.ability_index).cloned())
                            .is_some_and(|ability| {
                                crate::special_actions::can_activate_mana_ability_check_with_view(
                                    &staged,
                                    unconstrained.payer,
                                    choice.source,
                                    choice.ability_index,
                                    &ability,
                                    &view,
                                    None,
                                )
                                .is_ok()
                            })
                    };
                    if !legal {
                        break;
                    }
                    let previous_pool = staged.player(unconstrained.payer)?.mana_pool.clone();
                    let mut repeat_decision_maker = chooser();
                    if activate_with_mana_triggers(
                        &mut staged,
                        unconstrained.payer,
                        choice.source,
                        choice.ability_index,
                        choice.color_restriction.clone(),
                        &mut repeat_decision_maker,
                    )
                    .is_err()
                        || repeat_decision_maker.awaiting_choice()
                        || staged
                            .player(unconstrained.payer)
                            .is_none_or(|player| player.mana_pool == previous_pool)
                    {
                        break;
                    }
                    max_activations += 1;
                }
                (positive_pool_delta(&before, &after), max_activations)
            };
            (expected_mana.total() > 0).then(|| ManaPaymentActivationOption {
                source: choice.source,
                ability_index: choice.ability_index,
                color_restriction: choice.color_restriction,
                expected_mana,
                repeatable: max_activations > 1,
                max_activations,
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
    useful_manual_mana_abilities_inner(game, request, true)
}

fn useful_manual_mana_abilities_inner(
    game: &GameState,
    request: &ManaPaymentRequest,
    allow_projection: bool,
) -> Vec<(ObjectId, usize)> {
    useful_manual_mana_abilities_with_resolved(game, request, allow_projection, &[])
}

fn useful_manual_mana_abilities_with_resolved(
    game: &GameState,
    request: &ManaPaymentRequest,
    allow_projection: bool,
    resolved: &[ResolvedManualActivation],
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
    let analysis = super::sources::ManaSourceAnalysis::new(game);
    for choice in collect_activation_choices_with_view(game, &unconstrained, false, &analysis.view)
    {
        let key = (choice.source, choice.ability_index);
        if result.contains(&key) {
            continue;
        }
        if let Some((_, _, _, useful)) = resolved.iter().find(|(source, index, colors, _)| {
            *source == choice.source
                && *index == choice.ability_index
                && *colors == choice.color_restriction
        }) {
            if *useful {
                result.push(key);
            }
            continue;
        }
        if allow_projection
            && let Some(projected) = analysis.project(&choice)
            && !projected.needs_choice
        {
            // The reviewed projection includes every immediate mana trigger
            // and replacement, with the same restriction/snow provenance used
            // by execution. A source is still checked again when activated.
            let mut staged = game.clone();
            staged.tap(choice.source);
            if projected
                .credits
                .iter()
                .all(|credit| credit.commit(&mut staged).is_ok())
            {
                if staged.covered_mana_payment_pips(request) > before {
                    result.push(key);
                }
                continue;
            }
        }
        let mut staged = game.clone();
        let mut exclusions = unconstrained.preferences.excluded_sources.clone();
        exclusions.push(choice.source);
        let snapshot = game.object(choice.source).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        });
        let has_tap = game.current_ability(choice.source, choice.ability_index).is_some_and(|ability|
            matches!(&ability.kind, AbilityKind::Activated(activated) if activated.has_tap_cost()));
        let mut decision_maker = SelectFirstDecisionMaker;
        let Ok(events) = crate::special_actions::perform_mana_ability_with_payment_mode(
            &mut staged,
            request.payer,
            choice.source,
            choice.ability_index,
            choice.color_restriction,
            Some(exclusions),
            &mut decision_maker,
        ) else {
            continue;
        };
        if decision_maker.awaiting_choice() {
            continue;
        }
        for event in events {
            staged.queue_trigger_event(event.provenance(), event);
        }
        if finish_mana_activation(
            &mut staged,
            request.payer,
            choice.source,
            has_tap,
            snapshot,
            &mut decision_maker,
        )
        .is_ok()
            && !decision_maker.awaiting_choice()
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
    execute_mana_payment_plan_in_context(game, request, expected_plan, decision_maker, None)
}
pub(crate) fn execute_mana_payment_plan_in_context(
    game: &mut GameState,
    request: &ManaPaymentRequest,
    expected_plan: &ManaPaymentPlan,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
    execution: Option<&crate::effects::ExecutionContextCheckpoint>,
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
        let mut replay = super::witness::WitnessDecisionMaker::for_activation(
            step.replacement_witnesses.as_deref(),
            step.production_witnesses.as_deref(),
            decision_maker,
        );
        if let Err(error) = activate_with_mana_triggers_retaining_state(
            game,
            request.payer,
            step.source,
            step.ability_index,
            step.color_restriction.clone(),
            &mut replay,
            false,
        ) {
            *game = checkpoint;
            return Err(ManaPaymentFailure::from_execution(error));
        }
        if replay.awaiting_choice() {
            *game = checkpoint;
            return Ok(super::ManaPaymentExecution::PendingDecision);
        }
        if !replay.complete() {
            *game = checkpoint;
            return Err(ManaPaymentFailure::ExecutionFailed);
        }
    }
    let before = crate::events::other::before_tap_state_snapshots(game);
    let mut tapped_events = Vec::new();
    for allocation in &current.allocations {
        let success = match allocation.payment {
            super::PlannedPipPayment::Convoke(source)
            | super::PlannedPipPayment::Improvise(source) => {
                if game.object(source).is_none() || game.is_tapped(source) {
                    false
                } else {
                    game.tap(source);
                    tapped_events.push(crate::triggers::TriggerEvent::new(
                        crate::events::PermanentTappedEvent::capture(
                            game,
                            source,
                            Some(request.payer),
                        ),
                        crate::provenance::ProvNodeId::default(),
                    ));
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
                    match crate::costs::Cost::exile_from_graveyard(1, None).pay(game, &mut context)
                    {
                        Ok(crate::costs::CostPaymentResult::Paid) => true,
                        Err(crate::cost::CostPaymentError::ExecutionFailed(error)) => {
                            *game = checkpoint;
                            return Err(ManaPaymentFailure::EffectExecutionFailed(error));
                        }
                        _ => false,
                    }
                }
            }
            _ => true,
        };
        if !success {
            *game = checkpoint;
            return Err(ManaPaymentFailure::ExecutionFailed);
        }
    }
    crate::events::other::bind_before_tap_state_snapshots(&mut tapped_events, &before);
    crate::events::other::group_tap_state_events(game, &mut tapped_events, Default::default());
    let paid = game
        .try_pay_mana_cost_with_payment_options_in_context(
            request.payer,
            Some(request.source),
            &current.mana_cost_after_alternatives,
            request.x_value,
            request.reason,
            &request.spend_policy,
            request.allow_life_payment,
            request.allow_black_life,
            request.preferences.prefer_life,
            decision_maker,
            execution,
        )
        .map_err(|error| {
            *game = checkpoint.clone();
            ManaPaymentFailure::EffectExecutionFailed(error)
        })?;
    if decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint, true);
        return Ok(super::ManaPaymentExecution::PendingDecision);
    }
    if !paid {
        *game = checkpoint;
        return Err(ManaPaymentFailure::ExecutionFailed);
    }
    for event in tapped_events {
        game.queue_trigger_event(event.provenance(), event);
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
            // A currently unavailable conditional source can become legal
            // after an earlier activation. A static inventory is not an upper
            // bound in that case, even when every source has a tap cost.
            if !mana_ability.has_tap_cost()
                || mana_ability.activation_condition.is_some()
                || !mana_ability.activation_restrictions.is_empty()
                || !mana_ability.additional_restrictions.is_empty()
            {
                return false;
            }
        }
    }
    true
}

/// Fast unpayability check run once before the planner's search begins.
///
/// Proving a cost unpayable is the planner's worst case: it expands the whole
/// candidate space, cloning a `GameState` per candidate. The solver answers
/// the same question for supported independent sources in
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
    let view = DerivedGameView::new(game);
    if super::color_reachability::rules_out_payment(game, request, &view) {
        return true;
    }
    if finite_fixed_production_is_insufficient(game, request, &view) {
        return true;
    }
    if super::sources::has_potential_mana_triggers(game, &view)
        || super::sources::has_mana_modifying_replacements(game)
        || !every_mana_ability_is_single_use(game, request, &view)
    {
        return false;
    }
    // A tap cost alone does not prove independence: the activation may also
    // untap another source, change an ability, or expose a new resource. Only
    // veto costs when every currently legal activation has an exact projection.
    let analysis = super::sources::ManaSourceAnalysis::new(game);
    if collect_activation_choices_with_view(game, request, false, &analysis.view)
        .iter()
        .any(|choice| analysis.project(choice).is_none())
    {
        return false;
    }
    // Constrained costs use this complete planner in the public affordability
    // query. Calling that query here would re-enter the same payment search.
    // Keep the preceding conservative bounds, then let the current search
    // validate producer provenance and the X allocation itself.
    if !request.cost.spending_restrictions().is_empty() {
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

/// A quantity-only upper bound, after keyword resources have been allocated.
/// Unlike an executable projection, this may ignore life/input restrictions:
/// each fixed tap producer contributes its largest output once. Unknown state
/// changes keep the full search. In particular, paying life must not uncover
/// another ability, change production, or cause immediate extra mana.
fn finite_fixed_production_is_insufficient(
    game: &GameState,
    request: &ManaPaymentRequest,
    view: &DerivedGameView<'_>,
) -> bool {
    fn symbol_units(symbols: &[ManaSymbol]) -> Option<u64> {
        symbols
            .iter()
            .all(|symbol| {
                matches!(
                    symbol,
                    ManaSymbol::White
                        | ManaSymbol::Blue
                        | ManaSymbol::Black
                        | ManaSymbol::Red
                        | ManaSymbol::Green
                        | ManaSymbol::Colorless
                )
            })
            .then_some(symbols.len() as u64)
    }
    fn fixed_units(production: super::program::ManaProduction<'_>) -> Option<u64> {
        use super::program::ManaProduction;
        use crate::effect::Value;
        match production {
            ManaProduction::Fixed { symbols, .. } => symbol_units(symbols),
            ManaProduction::Repeated {
                symbols,
                amount: Value::Fixed(amount),
                ..
            } => symbol_units(symbols)?.checked_mul((*amount).max(0) as u64),
            ManaProduction::ChooseColors {
                amount: Value::Fixed(amount),
                ..
            }
            | ManaProduction::ChosenColor {
                amount: Value::Fixed(amount),
                ..
            }
            | ManaProduction::CommanderIdentity {
                amount: Value::Fixed(amount),
                ..
            }
            | ManaProduction::NotedType {
                amount: Value::Fixed(amount),
                ..
            } => Some((*amount).max(0) as u64),
            _ => None,
        }
    }
    let Some(player) = game.player(request.payer) else {
        return false;
    };
    let pips = mana_payment_expanded_pips(game, request);
    if pips
        .iter()
        .flatten()
        .any(|symbol| matches!(symbol, ManaSymbol::X))
    {
        return false;
    }
    let minimum = pips.iter().try_fold(0u64, |total, pip| {
        let units = pip
            .iter()
            .filter_map(|symbol| match symbol {
                // Ignore life availability/permissions in this upper-bound proof.
                ManaSymbol::Life(_) => Some(0),
                ManaSymbol::Black if request.allow_black_life => Some(0),
                ManaSymbol::Generic(amount) => Some(u64::from(*amount)),
                ManaSymbol::X => None,
                _ => Some(1),
            })
            .min()?;
        total.checked_add(units)
    });
    let Some(minimum) = minimum else {
        return false;
    };
    let pool = &player.mana_pool;
    let mut available = [
        pool.white,
        pool.blue,
        pool.black,
        pool.red,
        pool.green,
        pool.colorless,
    ]
    .into_iter()
    .map(u64::from)
    .sum::<u64>();
    if available >= minimum {
        return false;
    }
    if !request.allow_mana_abilities {
        return true;
    }
    let analysis = view.simple_battlefield_mana_analysis(request.payer);
    for &source in analysis.mana_source_ids() {
        let Some(abilities) = view.abilities_rc(source) else {
            return false;
        };
        let mut largest = 0u64;
        for &index in analysis.mana_ability_indices_for(source) {
            let Some(ability) = abilities.get(index) else {
                return false;
            };
            let AbilityKind::Activated(activated) = &ability.kind else {
                return false;
            };
            let Some(costs) = activated.mana_cost.as_all() else {
                return false;
            };
            if !costs.iter().any(|cost| cost.requires_tap())
                || costs
                    .iter()
                    .any(|cost| !cost.requires_tap() && cost.life_amount().is_none())
                || !activated.choices.is_empty()
                || activated
                    .effects
                    .segments
                    .iter()
                    .any(|segment| !segment.self_replacements.is_empty())
            {
                return false;
            }
            let Some(mut count) = activated
                .mana_output
                .as_ref()
                .map_or(Some(0), |output| symbol_units(output))
            else {
                return false;
            };
            for effect in activated.effects.iter() {
                let Some(units) = effect.mana_production().and_then(fixed_units) else {
                    return false;
                };
                let Some(total) = count.checked_add(units) else {
                    return false;
                };
                count = total;
            }
            largest = largest.max(count);
        }
        if !game.is_tapped(source) {
            let Some(total) = available.checked_add(largest) else {
                return false;
            };
            available = total;
        }
    }
    if available >= minimum {
        return false;
    }
    let effects = &game.effect_store;
    if !effects.continuous_effects.effects().is_empty()
        || !effects.granted_mana_abilities.is_empty()
        || !effects.pending_reflexive_triggers.is_empty()
        || super::sources::has_potential_mana_triggers(game, view)
        || super::sources::has_replacements_for_events(
            game,
            &[
                crate::events::EventKind::BecomeTapped,
                crate::events::EventKind::ManaAdded,
                crate::events::EventKind::AbilityActivated,
                crate::events::EventKind::LifeLoss,
            ],
        )
    {
        return false;
    }
    // Check printed as well as current abilities: an inactive life-dependent
    // static effect may begin applying only after a source's life cost is paid.
    for id in game.object_ids_in_deterministic_order() {
        let Some(object) = game.object(id) else {
            return false;
        };
        let Some(abilities) = view.abilities_rc(id) else {
            return false;
        };
        // The inventory below covers battlefield sources only. An ability
        // functioning in hand/graveyard/another zone is outside this proof's
        // domain even if the ordinary planner currently misses it as well.
        if object.zone != crate::zone::Zone::Battlefield
            && abilities.iter().any(|ability| {
                ability.functions_in(&object.zone)
                    && matches!(&ability.kind,
                AbilityKind::Activated(activated) if activated.is_runtime_mana_ability(
                    game, id, game.controller_of(object)))
            })
        {
            return false;
        }
        if object.abilities.iter().chain(abilities.iter()).any(|ability|
            ability.functions_in(&object.zone) && matches!(&ability.kind,
                AbilityKind::Static(static_ability) if static_ability.may_generate_continuous_effects()))
        { return false; }
    }
    true
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
    resource_scope: Option<crate::effects::tokens::resources::TokenQueryScope>,
}

#[derive(Debug)]
struct PlanningCursor {
    selections: AlternativeSelectionStream,
    deferred_selections: VecDeque<AlternativeSelection>,
    search_limited: bool,
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
        if crate::decision::spell_has_convoke(game, source)
            || crate::decision::spell_has_delve(game, source)
            || crate::decision::spell_has_improvise(game, source)
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
        let scope = self.resource_scope.get_or_insert_with(|| {
            crate::effects::tokens::resources::TokenQueryScope::new(game.token_creation_limits())
        });
        let meter = scope.meter();
        let mut staged = game.clone();
        staged.bind_token_query_meter(meter.clone());
        let mut result = self.plan_internal_with_resources(&staged, request, stop_after_first);
        if let Some(error) = crate::effects::tokens::resources::failure(&meter) {
            // A nested query owns its own work budget, but its enclosing
            // execution must still learn that a branch could not be computed.
            game.record_token_resource_failure(&error);
            self.pending = false;
            self.outer = None;
            result = Err(ManaPaymentFailure::EffectExecutionFailed(error));
        }
        if !self.pending {
            self.resource_scope = None;
        }
        result
    }

    fn plan_internal_with_resources(
        &mut self,
        game: &GameState,
        request: &ManaPaymentRequest,
        stop_after_first: bool,
    ) -> Result<Vec<ManaPaymentPlan>, ManaPaymentFailure> {
        let player = game
            .player(request.payer)
            .ok_or(ManaPaymentFailure::MissingPlayer)?;
        if request.preferences.x_allocation.is_some() && !request.cost.has_x_spending_restriction()
        {
            return Err(ManaPaymentFailure::ConflictingPreferences);
        }
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
            selections: alternative_payment_selections(game, request),
            deferred_selections: VecDeque::new(),
            search_limited: false,
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
                if self.sliced
                    && self.remaining == 0
                    && (cursor.selections.has_more() || !cursor.deferred_selections.is_empty())
                {
                    self.pending = true;
                    self.outer = Some(cursor);
                    return Err(ManaPaymentFailure::SearchLimitReached);
                }
                let (selection, deferred) =
                    if let Some(selection) = cursor.selections.next_preferred() {
                        (selection, false)
                    } else if let Some(selection) = cursor.deferred_selections.pop_front() {
                        (selection, true)
                    } else if let Some(selection) = cursor.selections.next_complete() {
                        (selection, false)
                    } else {
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
                payment_request.cost = request
                    .cost
                    .clone()
                    .bind_x_payment_if_unbound(request.x_value)
                    .with_required_x_allocation(request.preferences.x_allocation)
                    .with_pips(
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
                    cursor.plans.extend(build_plan(
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
                // per-source state clones with resource assignment. The module
                // declines anything it cannot model, so this only ever skips
                // work the search would have repeated.
                //
                // First proposals and existence checks use only cheap projected
                // candidates here. Unknown sources stay in the lazy fallback;
                // failed assignment never proves a payment impossible.
                let assigned = if deferred {
                    // This immutable selection already failed its cheap checks.
                    None
                } else if self.sliced && self.lazy_candidates && !self.preview_assignment {
                    // Replaying an entire assignment is not a sliced work unit.
                    None
                } else if self.lazy_candidates {
                    super::analytic::try_projected_candidates(&staged, &payment_request)
                } else {
                    super::analytic::try_candidates(&staged, &payment_request)
                };
                if let Some(candidates) = assigned {
                    self.visited_nodes = 0;
                    self.analytic_selections += 1;
                    for (final_game, steps) in candidates {
                        let pool_after = final_game
                            .player(request.payer)
                            .ok_or(ManaPaymentFailure::MissingPlayer)?
                            .mana_pool
                            .clone();
                        cursor.plans.extend(build_plan(
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
                if stop_after_first && self.lazy_candidates && !deferred {
                    // A failed projection is unknown, not an affordability
                    // result. Try the other keyword assignments' cheap exact
                    // witnesses before simulating this unresolved assignment.
                    // Retain every fallback for the case where none succeeds.
                    cursor.deferred_selections.push_back(selection);
                    continue;
                }
                let depth_limit = expanded_pip_count(&payment_request)
                    .saturating_add(MAX_EXTRA_ACTIVATIONS)
                    .max(payment_request.preferences.required_activations.len())
                    .max(1);
                self.visited_nodes = 0;
                self.searched_selections += 1;
                let mut search = CandidateSearch::new(
                    staged,
                    &payment_request,
                    depth_limit,
                    stop_after_first,
                    self.lazy_candidates,
                );
                if stop_after_first && self.preview_assignment {
                    search.preview_root = Some(game.clone());
                }
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
            let candidates = match candidates {
                Ok(candidates) => candidates,
                Err(ManaPaymentFailure::SearchLimitReached) => {
                    // One unresolved keyword assignment cannot exclude a
                    // legal witness in a later assignment. Preserve unknown
                    // only if every remaining assignment fails as well.
                    cursor.search_limited = true;
                    continue;
                }
                Err(error) => return Err(error),
            };
            for (final_game, steps) in candidates {
                let pool_after = final_game
                    .player(request.payer)
                    .ok_or(ManaPaymentFailure::MissingPlayer)?
                    .mana_pool
                    .clone();
                cursor.plans.extend(build_plan(
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
            Err(if cursor.search_limited {
                ManaPaymentFailure::SearchLimitReached
            } else {
                ManaPaymentFailure::NoLegalPlan
            })
        } else {
            Ok(cursor.plans)
        }
    }
}

/// On this deliberately narrow program subset, mana activations can change
/// only pools/provenance and tapped bits, all present in the search key. Unlike
/// user-facing undo permission, this also covers repeatable fixed mana filters.
/// Unknown effects, conditions and history observers retain full simulation.
fn fixed_mana_filter_board(game: &GameState) -> bool {
    let effects = &game.effect_store;
    if !effects.continuous_effects.effects().is_empty()
        || !effects.replacement_effects.effects().is_empty()
        || !effects.delayed_triggers.is_empty()
        || !effects.pending_trigger_events.is_empty()
        || !effects.pending_trigger_entries.is_empty()
        || !effects.pending_reflexive_triggers.is_empty()
        || !effects.granted_mana_abilities.is_empty()
        || !effects.restriction_effects.is_empty()
        || !effects.mana_spend_effects.permissions.is_empty()
    {
        return false;
    }
    game.object_ids_in_deterministic_order()
        .into_iter()
        .all(|id| {
            game.try_current_characteristics(id)
                .ok()
                .flatten()
                .is_some_and(|chars| {
                    chars.abilities.iter().all(|ability| {
                        let AbilityKind::Activated(activated) = &ability.kind else {
                            return false;
                        };
                        activated.timing == crate::ability::ActivationTiming::AnyTime
                            && activated.activation_condition.is_none()
                            && activated.activation_restrictions.is_empty()
                            && activated.additional_restrictions.is_empty()
                            && activated.mana_usage_restrictions.is_empty()
                            && activated.choices.is_empty()
                            && !activated.is_loyalty_ability
                            && activated.effects.is_empty()
                            && activated.mana_output.is_some()
                            && activated.mana_cost.as_all().is_some_and(|costs| {
                                costs.iter().all(|cost| {
                                    cost.requires_tap()
                                        || cost.requires_untap()
                                        || cost.is_mana_cost()
                                })
                            })
                    })
                })
        })
}

/// An overestimate for boards already proven to contain only fixed mana
/// programs. Ignore input costs, taps, ownership, restrictions and quantities:
/// if even unlimited copies of every output cannot satisfy one pip, no finite
/// activation sequence can pay it. This also terminates growing irrelevant
/// mana pools without imposing a search-depth or node cutoff.
fn fixed_outputs_cannot_pay_a_pip(game: &GameState, request: &ManaPaymentRequest) -> bool {
    let Some(player) = game.player(request.payer) else {
        return false;
    };
    let mut symbols = super::sources::pool_units(&player.mana_pool);
    symbols.extend(player.restricted_mana.iter().map(|unit| unit.symbol));
    let mut snow = player.mana_source_provenance.iter().any(|unit| {
        unit.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.supertypes.contains(&crate::types::Supertype::Snow))
            || game.current_has_supertype(unit.source, crate::types::Supertype::Snow)
    });
    for id in game.object_ids_in_deterministic_order() {
        let Ok(Some(chars)) = game.try_current_characteristics(id) else {
            return false;
        };
        for ability in chars.abilities.iter() {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return false;
            };
            let Some(output) = activated.mana_output.as_ref() else {
                return false;
            };
            symbols.extend(output.iter().copied());
            snow |=
                !output.is_empty() && game.current_has_supertype(id, crate::types::Supertype::Snow);
        }
    }
    let mut minimum_life = 0u32;
    for pip in mana_payment_expanded_pips(game, request) {
        let cheapest = pip
            .iter()
            .filter_map(|required| match required {
                ManaSymbol::Life(amount) => {
                    request.allow_life_payment.then_some(u32::from(*amount))
                }
                ManaSymbol::X | ManaSymbol::Generic(0) => Some(0),
                ManaSymbol::Snow => snow.then_some(0),
                required => symbols
                    .iter()
                    .any(|symbol| request.spend_policy.can_pay_symbol(*symbol, *required))
                    .then_some(0),
            })
            .min();
        let Some(life) = cheapest else {
            return true;
        };
        minimum_life = minimum_life.saturating_add(life);
    }
    // Fixed mana-only programs cannot increase life. This is a lower bound:
    // unlimited hypothetical outputs can only reduce the required life.
    !game.can_pay_life_with_reason(request.payer, minimum_life, request.reason)
}

/// A second, quantity-aware overestimate for the same fixed-program subset.
/// Starting mana remains finite. Each producible color is supplied in enough
/// quantity to cover the entire cost, ignoring all activation/input constraints.
fn fixed_outputs_cannot_cover_quantities(game: &GameState, request: &ManaPaymentRequest) -> bool {
    if !request.cost.spending_restrictions().is_empty() {
        return false;
    }
    let pips = mana_payment_expanded_pips(game, request);
    // Snow depends on source provenance. Leave it to the existing conservative
    // type proof and authoritative search until the bound models that provenance.
    if pips
        .iter()
        .flatten()
        .any(|symbol| matches!(symbol, ManaSymbol::Snow | ManaSymbol::X))
    {
        return false;
    }
    let Some(bound) = pips.iter().try_fold(0u32, |total, pip| {
        let maximum = pip
            .iter()
            .map(|symbol| match symbol {
                ManaSymbol::Generic(amount) => u32::from(*amount),
                _ => 1,
            })
            .max()
            .unwrap_or(0);
        total.checked_add(maximum)
    }) else {
        return false;
    };
    let mut outputs = Vec::new();
    for id in game.object_ids_in_deterministic_order() {
        let Ok(Some(chars)) = game.try_current_characteristics(id) else {
            return false;
        };
        for ability in chars.abilities.iter() {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return false;
            };
            let Some(produced) = &activated.mana_output else {
                return false;
            };
            outputs.extend(produced.iter().copied());
        }
    }
    let mut optimistic = game.clone();
    let Some(player) = optimistic.player_mut(request.payer) else {
        return false;
    };
    // Restricted units are already counted in mana_pool. Removing their
    // restrictions only enlarges the possible payments and preserves quantity.
    player.restricted_mana.clear();
    player.mana_source_provenance.clear();
    for symbol in outputs {
        let amount = match symbol {
            ManaSymbol::White => &mut player.mana_pool.white,
            ManaSymbol::Blue => &mut player.mana_pool.blue,
            ManaSymbol::Black => &mut player.mana_pool.black,
            ManaSymbol::Red => &mut player.mana_pool.red,
            ManaSymbol::Green => &mut player.mana_pool.green,
            ManaSymbol::Colorless => &mut player.mana_pool.colorless,
            _ => return false,
        };
        *amount = (*amount).max(bound);
    }
    optimistic
        .preview_mana_cost_payment_with_options(
            request.payer,
            Some(request.source),
            &request.cost,
            request.x_value,
            request.reason,
            &request.spend_policy,
            request.allow_life_payment,
            request.allow_black_life,
            false,
        )
        .is_none()
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
    visited: usize,
    out: Vec<(ManaPaymentScore, GameState, Vec<PlannedManaActivation>)>,
    expansion: Option<Expansion>,
    deferred_expansions: Vec<Expansion>,
    first_seen_depths: HashMap<u64, usize>,
    fixed_mana_only: bool,
    depth_limit: usize,
    depth_frontier: VecDeque<(GameState, Vec<SearchStep>)>,
    first: bool,
    lazy_candidates: bool,
    preview_root: Option<GameState>,
    cleanup: Option<ProposalCleanup>,
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
        let root_key = constrained_search_state_key(&game, request, &[]);
        let seen = HashSet::from([root_key]);
        let fixed_mana_only = fixed_mana_filter_board(&game);
        let unreachable = fixed_mana_only
            && (fixed_outputs_cannot_pay_a_pip(&game, request)
                || fixed_outputs_cannot_cover_quantities(&game, request));
        Self {
            queue: if unreachable {
                VecDeque::new()
            } else {
                VecDeque::from([(game, Vec::new())])
            },
            seen,
            visited: 0,
            out: Vec::new(),
            expansion: None,
            deferred_expansions: Vec::new(),
            first_seen_depths: HashMap::from([(root_key, 0)]),
            fixed_mana_only,
            depth_limit,
            depth_frontier: VecDeque::new(),
            first,
            lazy_candidates,
            preview_root: None,
            cleanup: None,
            result: None,
        }
    }
    fn finish(&mut self) -> Option<Result<Vec<Candidate>, ManaPaymentFailure>> {
        self.out.sort_by_key(|candidate| candidate.0);
        let result = Ok(std::mem::take(&mut self.out)
            .into_iter()
            .map(|(_, game, actions)| (game, actions))
            .collect());
        self.queue.clear();
        self.depth_frontier.clear();
        self.preview_root = None;
        self.cleanup = None;
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
            if let Some(cleanup) = self.cleanup.as_mut() {
                self.visited += 1;
                if let Some((game, activations)) = cleanup.step(request) {
                    let score = search_candidate_score(&game, request, &activations);
                    self.out.push((score, game, activations));
                    self.cleanup = None;
                    return self.finish();
                }
                continue;
            }
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
                            let duplicate = if self.fixed_mana_only
                                || next_path.iter().all(|step| step.activation.undo_safe)
                            {
                                let key =
                                    constrained_search_state_key(&staged, request, &next_path);
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
                    if (self.fixed_mana_only
                        || next_path.iter().all(|step| step.activation.undo_safe))
                        && !self
                            .seen
                            .insert(constrained_search_state_key(&staged, request, &next_path))
                    {
                        continue;
                    }
                    self.queue.push_back((staged, next_path));
                }
                continue;
            }
            let Some((game, path)) = self.queue.pop_front() else {
                if let Some(expansion) = self.deferred_expansions.pop() {
                    self.expansion = Some(expansion);
                    continue;
                }
                if !self.depth_frontier.is_empty() {
                    // Depth is an ordering hint, never a negative legality
                    // result. Resume the retained frontier after shallower
                    // alternatives have been explored. Sliced callers still
                    // yield according to their activation work budget.
                    self.depth_limit = self
                        .depth_limit
                        .saturating_mul(2)
                        .max(self.depth_limit.saturating_add(1));
                    self.queue = std::mem::take(&mut self.depth_frontier);
                    continue;
                }
                return self.finish();
            };
            self.visited += 1;
            if !self.out.is_empty() && self.visited >= MAX_RANKING_SEARCH_NODES {
                return self.finish();
            }
            if can_pay_request(&game, request) && required_activations_are_present(request, &path) {
                let life_to_pay = preview_life_to_pay(&game, request);
                let activations = path
                    .iter()
                    .map(|step| step.activation.clone())
                    .collect::<Vec<_>>();
                // Most proposals already have no surplus. Only polish surplus
                // previews; existence checks never pay for presentation cleanup.
                if self.first
                    && activations.len() > 1
                    && game.player(request.payer).is_some_and(|player| {
                        player.mana_pool.total() as usize > expanded_pip_count(request)
                    })
                    && let Some(root) = self.preview_root.take()
                {
                    self.cleanup = Some(ProposalCleanup::new(root, game, activations));
                    continue;
                }
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
                self.depth_frontier.push_back((game, path));
                continue;
            }
            // Collapsed only for the search; the inventory entry points keep
            // every source so the client can still offer them all.
            let mut ordering_request = request.clone();
            for step in &path {
                ordering_request
                    .preferences
                    .required_sources
                    .retain(|source| *source != step.activation.source);
                if let Some(index) = ordering_request
                    .preferences
                    .required_activations
                    .iter()
                    .position(|required| {
                        required.source == step.activation.source
                            && required.ability_index == step.activation.ability_index
                            && required.color_restriction == step.activation.color_restriction
                    })
                {
                    ordering_request
                        .preferences
                        .required_activations
                        .remove(index);
                }
            }
            let choices = collect_search_choices(&game, &ordering_request).into_iter();
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

/// Bounded, resumable deletion pass. Each unit replays at most one activation,
/// so sliced callers retain their normal work budget. Never subtract mana from
/// a simulated final pool: filters, triggers and replacements may depend on the
/// activation being removed. Only an authoritative replay can prove redundancy.
#[derive(Debug)]
struct ProposalCleanup {
    root: GameState,
    best: Candidate,
    removals: Vec<usize>,
    attempt: Option<(usize, usize, GameState, Vec<PlannedManaActivation>)>,
    remaining: usize,
}

impl ProposalCleanup {
    fn new(root: GameState, game: GameState, steps: Vec<PlannedManaActivation>) -> Self {
        let mut cleanup = Self {
            root,
            best: (game, steps),
            removals: Vec::new(),
            attempt: None,
            remaining: 32,
        };
        cleanup.reset_removals();
        cleanup
    }

    fn choice(step: &PlannedManaActivation) -> ActivationChoice {
        ActivationChoice {
            source: step.source,
            ability_index: step.ability_index,
            stored_color_choices: step
                .production_witnesses
                .as_deref()
                .unwrap_or_default()
                .iter()
                .filter(|record| record.choice.purpose == super::ManaChoicePurpose::StoredColor)
                .flat_map(|record| {
                    record
                        .output
                        .iter()
                        .filter_map(|symbol| mana_symbol_color(*symbol))
                })
                .collect(),
            color_restriction: step.color_restriction.clone(),
            flexibility: step.flexibility,
            replacement_witnesses: step.replacement_witnesses.clone(),
        }
    }

    fn reset_removals(&mut self) {
        self.removals = (0..self.best.1.len()).collect();
        // pop() tries consumable resources first, then later activations.
        self.removals.sort_by_key(|&index| {
            (
                activation_consumes_resources(&self.root, &Self::choice(&self.best.1[index])),
                index,
            )
        });
    }

    fn step(&mut self, request: &ManaPaymentRequest) -> Option<Candidate> {
        if self.remaining == 0
            || self.best.1.len() <= 1
            || (self.best.0.player(request.payer).is_some_and(|player| {
                player.mana_pool.total() as usize <= expanded_pip_count(request)
            }) && preview_life_to_pay(&self.best.0, request) == 0)
        {
            return Some(self.best.clone());
        }
        self.remaining -= 1;
        if self.attempt.is_none() {
            let Some(skip) = self.removals.pop() else {
                return Some(self.best.clone());
            };
            let remaining: Vec<_> = self
                .best
                .1
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != skip)
                .map(|(_, activation)| SearchStep {
                    activation: activation.clone(),
                })
                .collect();
            if !required_activations_are_present(request, &remaining) {
                return None;
            }
            self.attempt = Some((skip, 0, self.root.clone(), Vec::new()));
        }
        let (skip, mut index, mut staged, mut steps) = self.attempt.take().unwrap();
        if index == skip {
            index += 1;
        }
        if index < self.best.1.len() {
            let (_, next, activation) =
                prepare_owned_activation(staged, request, Self::choice(&self.best.1[index]))?;
            staged = next;
            steps.push(activation);
            self.attempt = Some((skip, index + 1, staged, steps));
            return None;
        }
        let life_after_payment = |game: &GameState| {
            game.player(request.payer)
                .map(|player| {
                    i64::from(player.life) - i64::from(preview_life_to_pay(game, request))
                })
                .unwrap_or(i64::MIN)
        };
        if can_pay_request(&staged, request)
            && life_after_payment(&staged) >= life_after_payment(&self.best.0)
        {
            self.best = (staged, steps);
            self.reset_removals();
        }
        None
    }
}

/// The payment model must observe the same immediate mana triggers as priority
/// activation. Callers own the transaction checkpoint (or a disposable branch).
fn activate_with_mana_triggers(
    game: &mut GameState,
    payer: crate::ids::PlayerId,
    source: ObjectId,
    ability_index: usize,
    colors: Option<Vec<Color>>,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<(), crate::game_loop::GameLoopError> {
    activate_with_mana_triggers_retaining_state(
        game,
        payer,
        source,
        ability_index,
        colors,
        decision_maker,
        false,
    )
}

fn activate_with_mana_witnesses(
    game: &mut GameState,
    payer: crate::ids::PlayerId,
    source: ObjectId,
    ability_index: usize,
    colors: Option<Vec<Color>>,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
    retain_continuous: bool,
    witnesses: Option<&[super::ManaReplacementWitness]>,
) -> Result<(), crate::game_loop::GameLoopError> {
    let Some(witnesses) = witnesses else {
        return activate_with_mana_triggers_retaining_state(
            game,
            payer,
            source,
            ability_index,
            colors,
            decision_maker,
            retain_continuous,
        );
    };
    let mut replay = super::witness::WitnessDecisionMaker::new(witnesses, decision_maker);
    activate_with_mana_triggers_retaining_state(
        game,
        payer,
        source,
        ability_index,
        colors,
        &mut replay,
        retain_continuous,
    )?;
    if !replay.awaiting_choice() && !replay.complete() {
        return Err(crate::game_loop::GameLoopError::InvalidState(
            "unused mana replacement witness".into(),
        ));
    }
    Ok(())
}

fn activate_with_mana_triggers_retaining_state(
    game: &mut GameState,
    payer: crate::ids::PlayerId,
    source: ObjectId,
    ability_index: usize,
    colors: Option<Vec<Color>>,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
    retain_continuous: bool,
) -> Result<(), crate::game_loop::GameLoopError> {
    let snapshot = game.object(source).map(|object| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
    });
    let has_tap = game.current_ability(source, ability_index).is_some_and(|ability| {
        matches!(&ability.kind, AbilityKind::Activated(activated) if activated.has_tap_cost())
    });
    crate::special_actions::perform_activate_mana_ability_restricted_colors(
        game,
        payer,
        source,
        ability_index,
        colors,
        decision_maker,
    )?;
    if !decision_maker.awaiting_choice() {
        // Trigger queuing queries characteristics and refreshes dirty state.
        // Retain before that boundary, while the proven tap/mana-only
        // activation is still the only mutation whose effects we suppress.
        if retain_continuous {
            game.retain_continuous_state_after_mana_activation();
        }
        finish_mana_activation(game, payer, source, has_tap, snapshot, decision_maker)?;
    }
    Ok(())
}

fn finish_mana_activation(
    game: &mut GameState,
    payer: crate::ids::PlayerId,
    source: ObjectId,
    has_tap: bool,
    snapshot: Option<crate::snapshot::ObjectSnapshot>,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<(), crate::game_loop::GameLoopError> {
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::AbilityActivated);
    game.queue_trigger_event(
        provenance,
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::AbilityActivatedEvent::new(source, payer, true)
                .with_activation_cost_has_tap(has_tap)
                .with_snapshot(snapshot),
            provenance,
        ),
    );
    crate::game_loop::resolve_pending_mana_triggers(game, decision_maker)
}

pub(super) fn prepare_activation(
    game: &GameState,
    request: &ManaPaymentRequest,
    choice: ActivationChoice,
) -> Option<PreparedChoice> {
    prepare_owned_activation(game.clone(), request, choice)
}

/// Continue a selected sequence without copying the state it already owns.
/// Branching search still calls `prepare_activation` to preserve its parent.
pub(super) fn prepare_owned_activation(
    mut staged: GameState,
    request: &ManaPaymentRequest,
    choice: ActivationChoice,
) -> Option<PreparedChoice> {
    // Analytic plans may select several sources from one initial inventory.
    // Recheck each activation in sequence: earlier costs can change whether
    // the next source is legal, even when its projected output is fixed.
    {
        let view = DerivedGameView::new(&staged);
        let abilities = view.abilities_rc(choice.source)?;
        let ability = abilities.get(choice.ability_index)?;
        crate::special_actions::can_activate_mana_ability_check_with_view(
            &staged,
            request.payer,
            choice.source,
            choice.ability_index,
            ability,
            &view,
            None,
        )
        .ok()?;
    }
    let before = staged
        .player(request.payer)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    // An undo-safe activation taps the source and adds mana and does nothing
    // else, so when no continuous effect can observe a tap or a pool change the
    // parent's continuous state is still correct for the staged state.
    let merge_safe =
        !super::sources::has_potential_mana_triggers(&staged, &DerivedGameView::new(&staged))
            && !super::sources::has_mana_modifying_replacements(&staged)
            && crate::game_loop::mana_ability_is_undo_safe(
                &staged,
                choice.source,
                choice.ability_index,
            )
            && !staged.continuous_effects_are_tap_sensitive();
    // Search-state equivalence is stricter than characteristic-cache reuse:
    // fixed mana triggers still record trigger history and pending ordinary
    // triggers, but their immediate effects only add mana. The reviewed source
    // projection proves that narrower mutation footprint independently.
    let retainable = staged.continuous_state_is_clean()
        && (merge_safe
            || super::sources::ManaSourceAnalysis::new(&staged)
                .project(&choice)
                .is_some());
    let mut fallback_decision_maker = SelectFirstDecisionMaker;
    let mut decision_maker = super::witness::WitnessDecisionMaker::record_production(
        request.payer,
        &mut fallback_decision_maker,
    )
    .with_stored_colors(choice.stored_color_choices.clone());
    if let Err(error) = activate_with_mana_witnesses(
        &mut staged,
        request.payer,
        choice.source,
        choice.ability_index,
        choice.color_restriction.clone(),
        &mut decision_maker,
        retainable,
        choice.replacement_witnesses.as_deref(),
    ) {
        if let ManaPaymentFailure::EffectExecutionFailed(error) =
            ManaPaymentFailure::from_execution(error)
        {
            staged.record_token_resource_failure(&error);
        }
        return None;
    }
    if !decision_maker.stored_colors_consumed() {
        return None;
    }
    if !retainable || !staged.retain_continuous_state_after_mana_activation() {
        if let Err(error) = staged.refresh_continuous_state() {
            staged.record_token_resource_failure(
                &crate::effects::ExecutionError::ContinuousDiscovery(error),
            );
            return None;
        }
    }
    let after = staged
        .player(request.payer)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    // Equal mana pools do not imply equal game states: a filter may tap a
    // source, pay life, or run a trigger that unlocks a later activation.
    // Let the search's constrained, merge-safe state key reject equivalent
    // cycles, preserving explicit repeated-activation requirements as well.
    let preference_key = activation_preference_key(request, &choice);
    let activation = PlannedManaActivation {
        production_witnesses: decision_maker
            .recorded_production()
            .map(|records| records.to_vec()),
        replacement_witnesses: choice.replacement_witnesses.clone(),
        source: choice.source,
        ability_index: choice.ability_index,
        color_restriction: choice.color_restriction,
        expected_mana: positive_pool_delta(&before, &after),
        expected_pool_after: after,
        flexibility: choice.flexibility,
        // The search uses this flag to merge states. The printed activation
        // alone cannot prove equivalence when its triggers/replacements can
        // change other resources or activation history.
        undo_safe: merge_safe,
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
    pub(super) replacement_witnesses: Option<Vec<super::ManaReplacementWitness>>,
    pub(super) stored_color_choices: Vec<Color>,

    pub(super) source: ObjectId,
    pub(super) ability_index: usize,
    pub(super) color_restriction: Option<Vec<Color>>,
    pub(super) flexibility: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct AlternativeSelection {
    remaining: Vec<PaymentPipSlot>,
    allocations: Vec<PlannedPipAllocation>,
}

#[derive(Debug)]
struct AlternativeSelectionStream {
    preferred: std::vec::IntoIter<AlternativeSelection>,
    preferred_keys: Vec<AlternativeSelection>,
    complete: Option<CompleteAlternativeSelections>,
}
impl AlternativeSelectionStream {
    fn only(selections: Vec<AlternativeSelection>) -> Self {
        Self {
            preferred: selections.into_iter(),
            preferred_keys: Vec::new(),
            complete: None,
        }
    }
    fn new(
        preferred: Vec<AlternativeSelection>,
        pips: Vec<PaymentPipSlot>,
        sources: Vec<AlternativeSource>,
        required: Vec<super::RequiredAlternativePayment>,
    ) -> Self {
        Self {
            preferred_keys: preferred.clone(),
            preferred: preferred.into_iter(),
            complete: Some(CompleteAlternativeSelections {
                shapes: HybridPipShapes::new(pips, !sources.is_empty()),
                sources,
                required,
                assignments: None,
            }),
        }
    }
    fn has_more(&self) -> bool {
        self.preferred.len() > 0 || self.complete.is_some()
    }
    fn next_preferred(&mut self) -> Option<AlternativeSelection> {
        self.preferred.next()
    }
    fn next_complete(&mut self) -> Option<AlternativeSelection> {
        let complete = self.complete.as_mut()?;
        for selection in complete {
            if !self.preferred_keys.contains(&selection) {
                return Some(selection);
            }
        }
        self.complete = None;
        None
    }
}
#[derive(Debug)]
struct CompleteAlternativeSelections {
    shapes: HybridPipShapes,
    sources: Vec<AlternativeSource>,
    required: Vec<super::RequiredAlternativePayment>,
    assignments: Option<AlternativeAssignments>,
}
impl Iterator for CompleteAlternativeSelections {
    type Item = AlternativeSelection;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(assignments) = self.assignments.as_mut() {
                for selection in assignments {
                    if self.required.iter().all(|required| {
                        selection.allocations.iter().any(|allocation| {
                            allocation_matches_required_alternative(required, allocation)
                        })
                    }) {
                        return Some(selection);
                    }
                }
            }
            let pips = self.shapes.next()?;
            self.assignments = Some(AlternativeAssignments {
                pending: vec![(0, vec![None; pips.len()])],
                pips,
                sources: self.sources.clone(),
            });
        }
    }
}
#[derive(Debug)]
struct HybridPipShapes {
    pips: Vec<PaymentPipSlot>,
    hybrid_indices: Vec<(usize, u8)>,
    bits: Vec<bool>,
    phase: u8,
}
impl HybridPipShapes {
    fn new(pips: Vec<PaymentPipSlot>, enabled: bool) -> Self {
        let hybrid_indices: Vec<_> = if enabled {
            pips.iter()
                .enumerate()
                .filter_map(|(index, pip)| {
                    pip.alternatives.iter().find_map(|symbol| match symbol {
                        ManaSymbol::Generic(amount) if *amount > 1 => Some((index, *amount)),
                        _ => None,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        Self {
            bits: vec![false; hybrid_indices.len()],
            pips,
            hybrid_indices,
            phase: 0,
        }
    }
    fn shape(&self, all_split: bool) -> Vec<PaymentPipSlot> {
        let mut next_id = self.pips.len() as u32;
        let mut shape = Vec::new();
        for (index, slot) in self.pips.iter().enumerate() {
            let split = self
                .hybrid_indices
                .iter()
                .enumerate()
                .find(|(bit, (i, _))| *i == index && (all_split || self.bits[*bit]))
                .map(|(_, (_, n))| *n);
            if let Some(amount) = split {
                for _ in 0..amount {
                    shape.push(PaymentPipSlot {
                        pip: ManaPipId(next_id),
                        printed_index: slot.printed_index,
                        alternatives: vec![ManaSymbol::Generic(1)],
                    });
                    next_id += 1;
                }
            } else {
                shape.push(slot.clone());
            }
        }
        shape
    }
}
impl Iterator for HybridPipShapes {
    type Item = Vec<PaymentPipSlot>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.phase == 0 {
            self.phase = if self.bits.is_empty() { 3 } else { 1 };
            return Some(self.pips.clone());
        }
        if self.phase == 1 {
            self.phase = 2;
            return Some(self.shape(true));
        }
        if self.phase == 3 {
            return None;
        }
        for bit in &mut self.bits {
            if !*bit {
                *bit = true;
                break;
            }
            *bit = false;
        }
        // The all-split shape was offered first; there are no later bit patterns.
        if self.bits.iter().all(|bit| *bit) {
            self.phase = 3;
            return None;
        }
        Some(self.shape(false))
    }
}
#[derive(Debug)]
struct AlternativeAssignments {
    pips: Vec<PaymentPipSlot>,
    sources: Vec<AlternativeSource>,
    pending: Vec<(usize, Vec<Option<AlternativeSource>>)>,
}
impl Iterator for AlternativeAssignments {
    type Item = AlternativeSelection;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((index, selected)) = self.pending.pop() {
            if index == self.sources.len() {
                let mut remaining = Vec::new();
                let mut allocations = Vec::new();
                for (slot, source) in self.pips.iter().zip(selected) {
                    if let Some(source) = source {
                        let payment = match source.kind {
                            AlternativeKind::Convoke(_) => {
                                super::PlannedPipPayment::Convoke(source.source)
                            }
                            AlternativeKind::Improvise => {
                                super::PlannedPipPayment::Improvise(source.source)
                            }
                            AlternativeKind::Delve => {
                                super::PlannedPipPayment::Delve(source.source)
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
                return Some(AlternativeSelection {
                    remaining,
                    allocations,
                });
            }
            let source = self.sources[index];
            self.pending.push((index + 1, selected.clone()));
            if selected
                .iter()
                .flatten()
                .any(|other| other.source == source.source)
            {
                continue;
            }
            let mut equivalent_pips = Vec::new();
            let mut children = Vec::new();
            for (pip_index, pip) in self.pips.iter().enumerate() {
                if selected[pip_index].is_some()
                    || !alternative_can_pay(source.kind, &pip.alternatives)
                    || equivalent_pips.contains(&&pip.alternatives)
                {
                    continue;
                }
                equivalent_pips.push(&pip.alternatives);
                let mut next = selected.clone();
                next[pip_index] = Some(source);
                children.push((index + 1, next));
            }
            self.pending.extend(children.into_iter().rev());
        }
        None
    }
}

// This bounds ordering hints only; complete lazy traversal follows them.
const PREFERRED_ALTERNATIVE_SELECTIONS: usize = 128;

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
) -> AlternativeSelectionStream {
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
            return AlternativeSelectionStream::only(Vec::new());
        };
        if !request.allow_life_payment {
            return AlternativeSelectionStream::only(Vec::new());
        }
        slot.alternatives
            .retain(|symbol| matches!(symbol, ManaSymbol::Life(_)));
        if slot.alternatives.is_empty() {
            return AlternativeSelectionStream::only(Vec::new());
        }
    }

    if request.reason != crate::costs::PaymentReason::CastSpell
        || request.assist_completion.is_some()
    {
        return AlternativeSelectionStream::only(vec![AlternativeSelection {
            remaining: pips,
            allocations: Vec::new(),
        }]);
    }

    let Some(source) = game.object(request.source) else {
        return AlternativeSelectionStream::only(vec![AlternativeSelection {
            remaining: pips,
            allocations: Vec::new(),
        }]);
    };
    if game.controller_of(source) != request.payer {
        return AlternativeSelectionStream::only(vec![AlternativeSelection {
            remaining: pips,
            allocations: Vec::new(),
        }]);
    }
    let mut sources = Vec::new();
    if crate::decision::spell_has_convoke(game, source) {
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
    if crate::decision::spell_has_delve(game, source) {
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
    if crate::decision::spell_has_improvise(game, source) {
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
    for pips in preferred_hybrid_pip_shapes(&pips, !sources.is_empty()) {
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
    AlternativeSelectionStream::new(
        selections,
        pips,
        sources,
        request.preferences.required_alternatives.clone(),
    )
}

/// Monocolored hybrid pips ({2/W}) may be paid through their generic half,
/// which is N separate generic mana. Convoke, delve, and improvise pay one
/// generic mana per resource (CR 702.51a, 702.66a, 702.126a), so two
/// resources can jointly pay a {2/W}'s {2}. Offer each such pip both as
/// printed and split into N {1} slots (with fresh pip ids, same printed
/// index). This bounded prefix provides ordering hints; HybridPipShapes
/// supplies every remaining shape lazily.
fn preferred_hybrid_pip_shapes(
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
    if out.len() >= PREFERRED_ALTERNATIVE_SELECTIONS {
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
            if out.len() >= PREFERRED_ALTERNATIVE_SELECTIONS {
                break;
            }
        }
    };
    if source.required || resource_first {
        include_source(selected, out);
    }
    if out.len() < PREFERRED_ALTERNATIVE_SELECTIONS {
        enumerate_alternative_selections(
            pips,
            sources,
            source_index + 1,
            selected,
            out,
            resource_first,
        );
    }
    if !source.required && !resource_first && out.len() < PREFERRED_ALTERNATIVE_SELECTIONS {
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
        source_controller: Some(request.payer),
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
    // Matching printed symbols does not prove independence: a trigger or
    // replacement can distinguish two sources, and tapping can alter another
    // source's output. Preserve those branches until their dependencies are
    // represented by the compact state model.
    if super::sources::has_potential_mana_triggers(game, view)
        || !game.effect_store.replacement_effects.effects().is_empty()
        || game.continuous_effects_are_tap_sensitive()
        || !every_mana_ability_is_single_use(game, request, view)
    {
        return choices;
    }
    let analysis = super::sources::ManaSourceAnalysis::new(game);
    #[derive(PartialEq)]
    struct ClassKey {
        symbols: Vec<ManaSymbol>,
        color_restriction: Option<Vec<Color>>,
        flexibility: usize,
        snow: bool,
        qualified_units: Vec<super::resources::PaymentManaUnit>,
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
        let Some(projected) = analysis.project(choice) else {
            keep[index] = true;
            continue;
        };
        // Preserve multiplicity: one green and two green are different
        // resources even though their sets of possible colors are identical.
        let symbols = super::sources::pool_units(&projected.output);
        let key = ClassKey {
            symbols,
            color_restriction: choice.color_restriction.clone(),
            flexibility: choice.flexibility,
            snow: game.current_has_supertype(choice.source, crate::types::Supertype::Snow),
            qualified_units: projected
                .credits
                .iter()
                .flat_map(|credit| credit.spendable_units(game, request))
                .collect(),
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
    let mut choices = collect_activation_choices_inner(game, request, true);
    let missing = game.uncovered_mana_payment_pips(request);
    // Ordering only: uncertain production never removes a legal branch. In
    // particular triggers, replacements and filter dependencies still execute
    // through prepare_activation when their branch is selected.
    choices.sort_by_cached_key(|choice| {
        let preference = activation_preference_key(request, choice);
        let ability = game.current_ability(choice.source, choice.ability_index);
        let activated = ability.as_ref().and_then(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        });
        let symbols = activated
            .map(|ability| ability.inferred_mana_symbols(game, choice.source, request.payer))
            .unwrap_or_default();
        let symbols: Vec<_> = symbols
            .into_iter()
            .filter(|produced| {
                // Fixed bundles add every symbol even if the ability also exposes
                // colour choices. Only inferred choice outputs use the restriction.
                activated.is_some_and(|ability| !ability.mana_symbols().is_empty())
                    || choice.color_restriction.as_ref().is_none_or(|colors| {
                        mana_symbol_color(*produced).is_none_or(|color| colors.contains(&color))
                    })
            })
            .collect();
        let edges: Vec<Vec<usize>> = symbols
            .iter()
            .map(|produced| {
                missing
                    .iter()
                    .enumerate()
                    .filter_map(|(index, pip)| {
                        pip.iter()
                            .any(|required| match required {
                                ManaSymbol::Generic(_) => true,
                                ManaSymbol::Snow => game.current_has_supertype(
                                    choice.source,
                                    crate::types::Supertype::Snow,
                                ),
                                ManaSymbol::Life(_) | ManaSymbol::X => false,
                                _ => request.spend_policy.can_pay_symbol(*produced, *required),
                            })
                            .then_some(index)
                    })
                    .collect()
            })
            .collect();
        fn assign(
            unit: usize,
            edges: &[Vec<usize>],
            owners: &mut [Option<usize>],
            seen: &mut [bool],
        ) -> bool {
            for &pip in &edges[unit] {
                if seen[pip] {
                    continue;
                }
                seen[pip] = true;
                if owners[pip].is_none_or(|previous| assign(previous, edges, owners, seen)) {
                    owners[pip] = Some(unit);
                    return true;
                }
            }
            false
        }
        let mut owners = vec![None; missing.len()];
        for unit in 0..symbols.len() {
            assign(unit, &edges, &mut owners, &mut vec![false; missing.len()]);
        }
        let useful_pips = owners.iter().filter(|owner| owner.is_some()).count();
        let constrained = missing
            .iter()
            .zip(&owners)
            .filter(|(pip, owner)| {
                owner.is_some() && !pip.iter().any(|s| matches!(s, ManaSymbol::Generic(_)))
            })
            .count();
        let costly = activation_consumes_resources(game, choice);
        (
            preference.0,
            preference.1,
            preference.2,
            u8::from(useful_pips == 0),
            costly,
            std::cmp::Reverse(constrained),
            std::cmp::Reverse(useful_pips),
            preference.3,
            preference.4,
            preference.5,
        )
    });
    choices
}

fn collect_activation_choices_inner(
    game: &GameState,
    request: &ManaPaymentRequest,
    collapse: bool,
) -> Vec<ActivationChoice> {
    let view = DerivedGameView::new(game);
    collect_activation_choices_with_view(game, request, collapse, &view)
}

pub(super) fn collect_activation_choices_with_view(
    game: &GameState,
    request: &ManaPaymentRequest,
    collapse: bool,
    view: &DerivedGameView<'_>,
) -> Vec<ActivationChoice> {
    let raw = collect_raw_activation_choices_with_view(game, request, collapse, view);
    let analysis = super::sources::ManaSourceAnalysis::new(game);
    raw.into_iter()
        .flat_map(|choice| {
            if analysis
                .project(&choice)
                .is_some_and(|projected| !projected.needs_choice)
            {
                return vec![choice];
            }
            let Some(branches) = analysis.branches(&choice) else {
                return vec![choice];
            };
            branches
                .iter()
                .map(|branch| ActivationChoice {
                    replacement_witnesses: Some(branch.witnesses.clone()),
                    ..choice.clone()
                })
                .collect()
        })
        .collect()
}

fn collect_raw_activation_choices_with_view(
    game: &GameState,
    request: &ManaPaymentRequest,
    collapse: bool,
    view: &DerivedGameView<'_>,
) -> Vec<ActivationChoice> {
    if !request.allow_mana_abilities {
        return Vec::new();
    }
    let analysis = view.simple_battlefield_mana_analysis(request.payer);
    // Even unusable output can accompany a tap that unlocks another source.
    let independent_activations = every_mana_ability_is_single_use(game, request, view);
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
                    view,
                    None,
                )
                .is_err()
                || (ability_mana_is_unusable_for_request(game, request, source, mana_ability)
                    && !super::sources::has_potential_mana_triggers(game, view)
                    && !super::sources::has_mana_modifying_replacements(game)
                    && independent_activations
                    && crate::game_loop::mana_ability_is_undo_safe(game, source, ability_index))
                // Paying a cost is never a time an instant could be cast.
                || crate::special_actions::activation_restricted_to_instant_timing(mana_ability)
            {
                continue;
            }
            let choice_start = out.len();
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
                            stored_color_choices: Vec::new(),
                            replacement_witnesses: None,
                            source,
                            ability_index,
                            color_restriction: Some(vec![color]),
                            flexibility,
                        });
                    }
                }
            }
            out.push(ActivationChoice {
                stored_color_choices: Vec::new(),
                replacement_witnesses: None,
                source,
                ability_index,
                color_restriction: None,
                flexibility,
            });
            let color_decisions = mana_ability
                .effects
                .iter()
                .filter(|effect| {
                    effect
                        .downcast_ref::<crate::effects::ChooseColorEffect>()
                        .is_some()
                })
                .count();
            if color_decisions > 0 {
                let base = out[choice_start..].to_vec();
                let mut domains = vec![Vec::new()];
                for _ in 0..color_decisions {
                    domains = domains
                        .into_iter()
                        .flat_map(|prefix| {
                            Color::ALL.into_iter().map(move |color| {
                                let mut next = prefix.clone();
                                next.push(color);
                                next
                            })
                        })
                        .collect();
                }
                for domain in domains {
                    for original in &base {
                        let mut choice = original.clone();
                        choice.stored_color_choices = domain.clone();
                        out.push(choice);
                    }
                }
            }
        }
    }
    if collapse {
        out = collapse_interchangeable_choices(game, request, view, out);
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

/// Tap-only and free sources precede sacrifices, life/counter payments and
/// filters. This is a preference, never a legality rule: dependencies stay in
/// the search and can be selected when ordinary sources cannot complete it.
fn activation_consumes_resources(game: &GameState, choice: &ActivationChoice) -> u8 {
    let Some(ability) = game.current_ability(choice.source, choice.ability_index) else {
        return 1;
    };
    let AbilityKind::Activated(activated) = &ability.kind else {
        return 1;
    };
    u8::from(
        activated.is_exhaust_ability()
            || activated
                .mana_cost
                .as_all()
                .is_none_or(|costs| costs.iter().any(|cost| !cost.requires_tap())),
    )
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
    if request.preferences.x_allocation.is_some() && !request.cost.has_x_spending_restriction() {
        return false;
    }
    let constrained_cost = request
        .cost
        .clone()
        .bind_x_payment_if_unbound(request.x_value)
        .with_required_x_allocation(request.preferences.x_allocation);
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

    payable_assignment_cost(game, request, &constrained_cost).is_some()
}

/// A helper's first affordable color assignment need not permit completion.
/// Search the actual unit assignments under the linked caster obligation.
fn payable_assignment_cost(
    game: &GameState,
    request: &ManaPaymentRequest,
    cost: &crate::mana::ManaCost,
) -> Option<crate::mana::ManaCost> {
    if let Some(completion) = request.assist_completion.as_deref() {
        if completion.assist_completion.is_some()
            || completion.payer == request.payer
            || completion.source != request.source
            || completion.reason != crate::costs::PaymentReason::CastSpell
            || request.reason != crate::costs::PaymentReason::CastSpell
            || request
                .cost
                .pips()
                .iter()
                .any(|pip| !matches!(pip.as_slice(), [ManaSymbol::Generic(_)]))
        {
            return None;
        }
        return game
            .mana_cost_with_payable_continuation(
                request.payer,
                Some(request.source),
                cost,
                request.x_value,
                request.reason,
                &request.spend_policy,
                request.allow_life_payment,
                request.allow_black_life,
                request.preferences.prefer_life,
                |after, paid| {
                    let mut completion = completion.clone();
                    completion.cost = completion.cost.with_prepaid_generic(paid.symbols());
                    match check_mana_payment(after, &completion) {
                        Ok(_) => true,
                        Err(ManaPaymentFailure::EffectExecutionFailed(error)) => {
                            after.record_token_resource_failure(&error);
                            false
                        }
                        Err(_) => false,
                    }
                },
            )
            .unwrap_or_else(|error| {
                game.record_token_resource_failure(&error);
                None
            });
    }
    game.can_pay_mana_cost_with_payment_options(
        request.payer,
        Some(request.source),
        cost,
        request.x_value,
        request.reason,
        &request.spend_policy,
        request.allow_life_payment,
        request.allow_black_life,
        request.preferences.prefer_life,
    )
    .then(|| cost.clone())
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
    let paid = after_payment
        .try_pay_mana_cost_with_payment_options(
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
        .unwrap_or_else(|error| {
            game.record_token_resource_failure(&error);
            false
        });
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
) -> Option<ManaPaymentPlan> {
    let mut qualified_request = payment_request.clone();
    qualified_request.cost = payable_assignment_cost(game, payment_request, &payment_request.cost)?;
    let payment_request = &qualified_request;
    let (preview, life_to_pay) = game.preview_mana_cost_payment_with_options(
        payment_request.payer,
        Some(payment_request.source),
        &payment_request.cost,
        payment_request.x_value,
        payment_request.reason,
        &payment_request.spend_policy,
        payment_request.allow_life_payment,
        payment_request.allow_black_life,
        payment_request.preferences.prefer_life,
    )?;
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
    let paid = staged
        .try_pay_mana_cost_with_payment_options(
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
        .unwrap_or_else(|error| {
            game.record_token_resource_failure(&error);
            false
        });
    if !paid {
        return None;
    }
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

    let x_allocation = game.preview_x_mana_allocation(
        payment_request.payer,
        Some(payment_request.source),
        &payment_request.cost,
        payment_request.x_value,
        payment_request.reason,
        &payment_request.spend_policy,
        payment_request.allow_life_payment,
        payment_request.allow_black_life,
        payment_request.preferences.prefer_life,
    )?;
    let payment_cost = payment_request
        .cost
        .clone()
        .with_required_x_allocation(x_allocation);
    let request_hash = request_hash(request);
    let id = plan_hash(
        request_hash,
        &steps,
        &allocations,
        &payment_cost,
        &pool_after_payment,
    );
    Some(ManaPaymentPlan {
        payable: true,
        id,
        request_hash,
        mana_ability_steps: steps,
        allocations,
        mana_cost_after_alternatives: payment_cost,
        pool_before,
        expected_pool_after_activations: pool_after_activations,
        expected_pool_after_payment: pool_after_payment,
        life_to_pay,
        score,
        warnings,
    })
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

/// Identity of the announced payment, independent of source preferences.
pub fn mana_payment_transaction_id(request: &ManaPaymentRequest) -> u64 {
    let mut announced = request.clone();
    announced.preferences = Default::default();
    request_hash(&announced)
}

fn request_hash(request: &ManaPaymentRequest) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.payer.hash(&mut hasher);
    request.source.hash(&mut hasher);
    format!("{:?}", request.reason).hash(&mut hasher);
    request.cost.pips().hash(&mut hasher);
    if !request.cost.spending_restrictions().is_empty() {
        "consumer mana spending constraints".hash(&mut hasher);
        request.cost.spending_restrictions().hash(&mut hasher);
    }
    if let Some(scope) = request.cost.x_payment_scope() {
        "generic X payment scope".hash(&mut hasher);
        scope.hash(&mut hasher);
    }
    if let Some(allocation) = request.preferences.x_allocation {
        "selected actual X allocation".hash(&mut hasher);
        allocation.hash(&mut hasher);
    }
    if let Some(completion) = request.assist_completion.as_deref() {
        "Assist caster continuation".hash(&mut hasher);
        request_hash(completion).hash(&mut hasher);
    }
    if let Some(payment) = request.cost.required_actual_payment() {
        "exact actual mana payment".hash(&mut hasher);
        payment.hash(&mut hasher);
    }
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
        step.replacement_witnesses.hash(&mut hasher);
        step.production_witnesses.hash(&mut hasher);
    }
    for allocation in allocations {
        allocation.pip.hash(&mut hasher);
        format!("{:?}", allocation.payment).hash(&mut hasher);
    }
    payment_cost.pips().hash(&mut hasher);
    if !payment_cost.spending_restrictions().is_empty() {
        "consumer mana spending constraints".hash(&mut hasher);
        payment_cost.spending_restrictions().hash(&mut hasher);
    }
    if let Some(scope) = payment_cost.x_payment_scope() {
        "generic X payment scope".hash(&mut hasher);
        scope.hash(&mut hasher);
    }
    if let Some(payment) = payment_cost.required_actual_payment() {
        "exact actual mana payment".hash(&mut hasher);
        payment.hash(&mut hasher);
    }
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

/// Equal mana pools do not imply equal remaining payment obligations. A
/// restricted activation and an unrestricted activation can produce the same
/// mana while satisfying different exact selections. Preserve each required
/// occurrence in the search key so deduplication cannot discard that path.
fn constrained_search_state_key(
    game: &GameState,
    request: &ManaPaymentRequest,
    path: &[SearchStep],
) -> u64 {
    let state_key = safe_search_state_key(game, request.payer);
    if request.preferences.required_sources.is_empty()
        && request.preferences.required_activations.is_empty()
    {
        return state_key;
    }
    let mut hasher = DefaultHasher::new();
    state_key.hash(&mut hasher);
    for source in &request.preferences.required_sources {
        path.iter()
            .any(|step| step.activation.source == *source)
            .hash(&mut hasher);
    }
    let mut matched = vec![false; path.len()];
    for required in &request.preferences.required_activations {
        let index = path.iter().enumerate().position(|(index, step)| {
            !matched[index]
                && required.source == step.activation.source
                && required.ability_index == step.activation.ability_index
                && required.color_restriction == step.activation.color_restriction
        });
        index.is_some().hash(&mut hasher);
        if let Some(index) = index {
            matched[index] = true;
        }
    }
    hasher.finish()
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
        // large. Hash the production characteristics used by consumer-side
        // spending requirements explicitly, along with the identity metadata.
        unordered_digest(&player.restricted_mana, |unit, hasher| {
            unit.symbol.hash(hasher);
            unit.source.hash(hasher);
            unit.source_chosen_creature_type.hash(hasher);
            // Different abilities of one source can produce identical colors
            // with different spending rules. Preserve the complete payload.
            crate::trigger_identity::hash_debug(hasher, &unit.restrictions)
                .expect("hash-only formatting is infallible");
        })
        .hash(&mut hasher);
        unordered_digest(&player.mana_source_provenance, |unit, hasher| {
            unit.symbol.hash(hasher);
            unit.source.hash(hasher);
            unit.restricted.hash(hasher);
            unit.retention.hash(hasher);
            unit.snapshot.is_some().hash(hasher);
            if let Some(snapshot) = &unit.snapshot {
                snapshot.zone.hash(hasher);
                snapshot.card_types.hash(hasher);
                snapshot.supertypes.hash(hasher);
                snapshot.subtypes.hash(hasher);
            }
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

    #[test]
    fn color_reachability_matches_full_search_for_devotion_and_mana_triggers() {
        use crate::{
            effect::{Effect, Value},
            target::{ObjectFilter, PlayerFilter},
        };
        let (mut game, alice) = game();
        let land = mana_land(&mut game, alice, "Variable source", &[], false, None);
        game.object_mut(land)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::mana_with_effects(
                crate::TotalCost::free(),
                vec![
                    Effect::new(crate::effects::ChooseColorEffect::new(PlayerFilter::You)),
                    Effect::new(crate::effects::AddManaOfChosenColorEffect::new(
                        Value::DevotionToChosenColor(PlayerFilter::You),
                        PlayerFilter::You,
                    )),
                ],
            ));
        let creature = CardBuilder::new(CardId::new(), "Devotion and mana trigger")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .build();
        let permanent = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.object_mut(permanent)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    PlayerFilter::You,
                    ObjectFilter::land(),
                ),
                vec![Effect::add_mana(vec![ManaSymbol::Green])],
            ));
        game.refresh_continuous_state().unwrap();
        for color in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Green,
            ManaSymbol::Black,
        ] {
            let request = request(&game, alice, land, ManaCost::from_pips(vec![vec![color]]));
            let veto = super::super::color_reachability::rules_out_payment(
                &game,
                &request,
                &DerivedGameView::new(&game),
            );
            let oracle = ManaPaymentPlanner {
                skip_affordability_gate: true,
                ..Default::default()
            }
            .first_plan(&game, &request);
            assert_eq!(veto, matches!(color, ManaSymbol::White | ManaSymbol::Black));
            assert_eq!(oracle.is_ok(), !veto, "{color:?}");
        }
        let mut request = request(
            &game,
            alice,
            land,
            ManaCost::from_pips(vec![vec![ManaSymbol::White, ManaSymbol::Life(2)]]),
        );
        assert!(!super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::White]]);
        request.spend_policy = crate::player::ManaSpendPolicy::from_any_color(true);
        assert!(!super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        request.spend_policy = Default::default();
        game.player_mut(alice).unwrap().mana_pool.white = 1;
        assert!(!super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        assert!(!game.is_tapped(land));
        assert!(game.chosen_color(land).is_none());
        // The fixed output is an alternative to the stored color. Its amount
        // reads blue devotion even though the resulting mana may be white.
        game.player_mut(alice).unwrap().mana_pool.white = 0;
        game.object_mut(land).unwrap().abilities_mut()[0] = crate::Ability::mana_with_effects(
            crate::TotalCost::free(),
            vec![
                Effect::new(crate::effects::ChooseColorEffect::new(PlayerFilter::You)),
                Effect::new(
                    crate::effects::AddManaOfChosenColorEffect::with_fixed_option(
                        Value::DevotionToChosenColor(PlayerFilter::You),
                        PlayerFilter::You,
                        Color::White,
                    ),
                ),
            ],
        );
        assert!(!super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        let plan = ManaPaymentPlanner::default()
            .first_plan(&game, &request)
            .expect("blue choice can produce fixed white");
        let mut replay = game.clone();
        execute_mana_payment_plan(&mut replay, &request, &plan, &mut SelectFirstDecisionMaker)
            .expect("chosen-color witness must replay");
        assert_eq!(replay.chosen_color(land), Some(Color::Blue));
        assert_eq!(replay.player(alice).unwrap().mana_pool.white, 0);
        assert!(!game.is_tapped(land));
    }

    #[test]
    fn color_reachability_keeps_unknown_sources_and_hybrid_alternatives() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = mana_land(
            &mut game,
            alice,
            "Green source",
            &[vec![ManaSymbol::Green]],
            false,
            None,
        );
        let opponent = mana_land(
            &mut game,
            bob,
            "Other source",
            &[vec![ManaSymbol::White]],
            false,
            None,
        );
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::White]]),
        );
        assert!(super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(opponent).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.timing = crate::ability::ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep;
        assert!(
            !super::super::color_reachability::rules_out_payment(
                &game,
                &request,
                &DerivedGameView::new(&game)
            ),
            "opponent-controlled sources with activator permissions are outside this proof"
        );
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::game_state::Phase::NextMain;
        game.turn.step = None;
        assert!(crate::decision::compute_legal_actions(&game, alice).unwrap().iter().any(|action|
            matches!(action, crate::decision::LegalAction::ActivateManaAbility { source, .. } if *source == opponent)));
        let mut funded_request = request.clone();
        funded_request.cost =
            ManaCost::from_pips(vec![vec![ManaSymbol::White], vec![ManaSymbol::Generic(1)]]);
        assert!(!finite_fixed_production_is_insufficient(
            &game,
            &funded_request,
            &DerivedGameView::new(&game)
        ));
        let plan = ManaPaymentPlanner::default()
            .first_plan(&game, &funded_request)
            .expect("shared source funds payment");
        let mut paid = game.clone();
        execute_mana_payment_plan(
            &mut paid,
            &funded_request,
            &plan,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(paid.is_tapped(source) && paid.is_tapped(opponent));
        assert_eq!(paid.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(paid.player(bob).unwrap().mana_pool.total(), 0);
        // The ability controller is its activator, even though its permanent
        // belongs to Bob. Alice's immediate mana trigger must see that event.
        let mut bonus = game.clone();
        bonus.tap(source);
        bonus
            .object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::You,
                    crate::target::ObjectFilter::land(),
                ),
                vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Green])],
            ));
        funded_request.cost =
            ManaCost::from_pips(vec![vec![ManaSymbol::White], vec![ManaSymbol::Green]]);
        let plan = ManaPaymentPlanner::default()
            .first_plan(&bonus, &funded_request)
            .expect("activator's trigger funds second pip");
        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert_eq!(plan.mana_ability_steps[0].source, opponent);
        execute_mana_payment_plan(
            &mut bonus,
            &funded_request,
            &plan,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(bonus.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(bonus.player(bob).unwrap().mana_pool.total(), 0);

        let AbilityKind::Activated(ability) =
            &mut game.object_mut(opponent).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.timing = crate::ability::ActivationTiming::AnyTime;
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::White, ManaSymbol::Green]]);
        assert!(!super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::White, ManaSymbol::Black]]);
        assert!(super::super::color_reachability::rules_out_payment(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        game.object_mut(source).unwrap().abilities_mut()[0] = crate::Ability::mana_with_effects(
            crate::TotalCost::free(),
            vec![
                crate::effect::Effect::add_mana(vec![ManaSymbol::Green]),
                crate::effect::Effect::untap(crate::target::ChooseSpec::Source),
            ],
        );
        assert!(
            !super::super::color_reachability::rules_out_payment(
                &game,
                &request,
                &DerivedGameView::new(&game)
            ),
            "a mana source with another mutation retains the complete search"
        );
    }

    #[test]
    fn finite_fixed_production_bound_matches_full_search_with_life_sources() {
        for sources in 1..=3 {
            let (mut game, alice) = game();
            let mut ids = Vec::new();
            for index in 0..sources {
                ids.push(mana_land(
                    &mut game,
                    alice,
                    "Fixed life source",
                    &[vec![ManaSymbol::Blue], vec![ManaSymbol::Green]],
                    false,
                    Some(index == 0),
                ));
            }
            game.refresh_continuous_state().unwrap();
            for amount in 1..=4 {
                let request = request(
                    &game,
                    alice,
                    ids[0],
                    ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]; amount]),
                );
                let veto = finite_fixed_production_is_insufficient(
                    &game,
                    &request,
                    &DerivedGameView::new(&game),
                );
                assert_eq!(veto, amount > sources, "sources={sources}, amount={amount}");
                let oracle = ManaPaymentPlanner {
                    skip_affordability_gate: true,
                    ..Default::default()
                }
                .first_plan(&game, &request);
                assert_eq!(oracle.is_ok(), amount <= sources);
                assert_eq!(
                    ManaPaymentPlanner::default()
                        .first_plan(&game, &request)
                        .is_ok(),
                    oracle.is_ok()
                );
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(ids.iter().all(|id| !game.is_tapped(*id)));
        }
    }

    #[test]
    fn finite_fixed_production_bound_preserves_pool_life_and_repeatable_alternatives() {
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Life source",
            &[vec![ManaSymbol::Blue]],
            false,
            Some(true),
        );
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Generic(1)]]),
        );
        assert!(finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        game.player_mut(alice).unwrap().mana_pool.colorless = 1;
        assert!(!finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        assert!(
            ManaPaymentPlanner::default()
                .first_plan(&game, &request)
                .is_ok()
        );
        game.player_mut(alice).unwrap().mana_pool.colorless = 0;
        request.cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue, ManaSymbol::Life(2)],
        ]);
        assert!(!finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        assert!(
            ManaPaymentPlanner::default()
                .first_plan(&game, &request)
                .is_ok()
        );
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 2]);
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(source).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.mana_cost = crate::cost::TotalCost::free();
        assert!(!finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        let plan = ManaPaymentPlanner::default()
            .first_plan(&game, &request)
            .unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 2);
    }

    #[test]
    fn finite_fixed_production_bound_keeps_immediate_mana_triggers_in_full_search() {
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Triggered life source",
            &[vec![ManaSymbol::Blue]],
            false,
            Some(true),
        );
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::You,
                    crate::target::ObjectFilter::land(),
                ),
                vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Blue])],
            ));
        game.refresh_continuous_state().unwrap();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 2]),
        );
        assert!(!finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        assert!(
            ManaPaymentPlanner::default()
                .first_plan(&game, &request)
                .is_ok()
        );
    }

    #[test]
    fn finite_fixed_production_bound_rejects_life_sensitive_effects_and_replacements() {
        use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Life source",
            &[vec![ManaSymbol::Blue]],
            false,
            Some(true),
        );
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 2]),
        );
        assert!(finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
        let mut branch = game.clone();
        let mut effect = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Source,
            Modification::AddAbility(crate::static_abilities::StaticAbility::flying()),
        );
        effect.condition = Some(crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::LifeLostThisTurn(crate::target::PlayerFilter::You),
            operator: ironsmith_core::effect_model::ValueComparisonOperator::GreaterThan,
            right: crate::effect::Value::Fixed(0),
        });
        branch.effect_store.continuous_effects.add_effect(effect);
        assert!(!finite_fixed_production_is_insufficient(
            &branch,
            &request,
            &DerivedGameView::new(&branch)
        ));
        game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::life::matchers::WouldLoseLifeMatcher::any_player(),
                crate::replacement::ReplacementAction::Double,
            ),
        );
        assert!(!finite_fixed_production_is_insufficient(
            &game,
            &request,
            &DerivedGameView::new(&game)
        ));
    }

    #[test]
    fn finite_fixed_production_bound_does_not_exclude_mana_from_other_zones() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Hand mana source")
            .card_types(vec![CardType::Creature])
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Hand);
        let mut ability =
            crate::Ability::mana(crate::cost::TotalCost::free(), vec![ManaSymbol::Blue]);
        ability.functional_zones = vec![Zone::Hand];
        let AbilityKind::Activated(activated) = &mut ability.kind else {
            unreachable!()
        };
        activated.mana_cost = crate::cost::TotalCost::from_cost(crate::costs::Cost::exile_self());
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(ability);
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );
        assert!(
            !finite_fixed_production_is_insufficient(&game, &request, &DerivedGameView::new(&game)),
            "battlefield-only enumeration cannot prove this source unavailable"
        );
    }

    #[test]
    fn alternative_activation_costs_do_not_enter_fixed_program_proofs() {
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Alternative cost source",
            &[vec![ManaSymbol::Green]],
            false,
            None,
        );
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(source).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.mana_cost = crate::cost::TotalCost::one_of(vec![
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
        ]);
        assert!(!fixed_mana_filter_board(&game));
    }

    #[test]
    fn unbounded_irrelevant_fixed_mana_is_proven_unpayable_without_a_cutoff() {
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Repeatable green",
            &[vec![ManaSymbol::Green]],
            false,
            None,
        );
        // The convenience mana constructor adds tap even for TotalCost::free.
        // Explicitly remove it so this fixture really can produce indefinitely.
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(source).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.mana_cost = crate::cost::TotalCost::free();
        assert!(!ability.has_tap_cost());
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );
        assert!(fixed_mana_filter_board(&game));
        assert!(fixed_outputs_cannot_pay_a_pip(&game, &request));
        assert!(matches!(
            plan_first_mana_payment(&game, &request),
            Err(ManaPaymentFailure::NoLegalPlan)
        ));
        request.spend_policy = crate::player::ManaSpendPolicy::from_any_color(true);
        assert!(!fixed_outputs_cannot_pay_a_pip(&game, &request));
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 1);
        request.spend_policy = crate::player::ManaSpendPolicy::default();
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue, ManaSymbol::Green]]);
        assert!(!fixed_outputs_cannot_pay_a_pip(&game, &request));
        assert!(plan_first_mana_payment(&game, &request).is_ok());
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue, ManaSymbol::Life(2)]]);
        assert!(!fixed_outputs_cannot_pay_a_pip(&game, &request));
        assert!(plan_first_mana_payment(&game, &request).is_ok());
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Life(2)]]);
        game.player_mut(alice).unwrap().life = 1;
        assert!(fixed_outputs_cannot_pay_a_pip(&game, &request));
        assert!(matches!(
            plan_first_mana_payment(&game, &request),
            Err(ManaPaymentFailure::NoLegalPlan)
        ));
        game.player_mut(alice).unwrap().life = 20;
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Snow]]);
        assert!(fixed_outputs_cannot_pay_a_pip(&game, &request));
        game.object_mut(source)
            .unwrap()
            .supertypes
            .push(crate::types::Supertype::Snow);
        assert!(!fixed_outputs_cannot_pay_a_pip(&game, &request));
        assert!(plan_first_mana_payment(&game, &request).is_ok());
    }

    #[test]
    fn finite_blue_pool_with_unbounded_green_does_not_hide_an_unpayable_second_blue() {
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Repeatable green",
            &[vec![ManaSymbol::Green]],
            false,
            None,
        );
        // The convenience mana constructor adds tap even for TotalCost::free.
        // Explicitly remove it so this fixture really can produce indefinitely.
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(source).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.mana_cost = crate::cost::TotalCost::free();
        assert!(!ability.has_tap_cost());
        game.player_mut(alice).unwrap().mana_pool.blue = 1;
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]),
        );
        assert!(fixed_mana_filter_board(&game));
        assert!(!fixed_outputs_cannot_pay_a_pip(&game, &request));
        assert!(fixed_outputs_cannot_cover_quantities(&game, &request));
        assert!(matches!(
            plan_first_mana_payment(&game, &request),
            Err(ManaPaymentFailure::NoLegalPlan)
        ));
        request.spend_policy = crate::player::ManaSpendPolicy::from_any_color(true);
        assert!(!fixed_outputs_cannot_cover_quantities(&game, &request));
        assert!(plan_first_mana_payment(&game, &request).is_ok());
        request.cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 3]);
        let repeated = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(repeated.mana_ability_steps.len(), 2);
        assert!(
            repeated
                .mana_ability_steps
                .iter()
                .all(|step| step.source == source)
        );
        request.spend_policy = Default::default();
        request.cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue, ManaSymbol::Green],
        ]);
        assert!(!fixed_outputs_cannot_cover_quantities(&game, &request));
        assert!(plan_first_mana_payment(&game, &request).is_ok());
        request.cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue, ManaSymbol::Life(2)],
        ]);
        assert!(!fixed_outputs_cannot_cover_quantities(&game, &request));
        assert!(plan_first_mana_payment(&game, &request).is_ok());
        game.player_mut(alice).unwrap().life = 1;
        assert!(fixed_outputs_cannot_cover_quantities(&game, &request));
    }

    #[test]
    fn search_key_preserves_same_source_mana_restriction_payloads() {
        let (mut game, alice) = game();
        let source = restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        );
        let (_, left, _) =
            prepare_owned_activation(game, &request, preview_choice(source, 0)).unwrap();
        let mut right = left.clone();
        let unit = &mut right.player_mut(alice).unwrap().restricted_mana[0];
        let crate::ability::ManaUsageRestriction::CastSpell { card_types, .. } =
            &mut unit.restrictions[0]
        else {
            panic!("expected typed restriction");
        };
        *card_types = vec![CardType::Sorcery];
        assert_eq!(
            left.player(alice).unwrap().mana_pool,
            right.player(alice).unwrap().mana_pool
        );
        assert_eq!(
            left.player(alice).unwrap().restricted_mana[0]
                .restrictions
                .len(),
            right.player(alice).unwrap().restricted_mana[0]
                .restrictions
                .len()
        );
        assert_ne!(
            safe_search_state_key(&left, alice),
            safe_search_state_key(&right, alice)
        );
    }

    #[test]
    fn combined_ready_and_manual_inventory_matches_independent_paths() {
        let (mut game, alice) = game();
        let source = mana_land(
            &mut game,
            alice,
            "Tap with life effect",
            &[vec![ManaSymbol::Green]],
            false,
            Some(false),
        );
        game.object_mut(source).unwrap().abilities_mut()[0] = crate::Ability::mana_with_effects(
            crate::cost::TotalCost::free(),
            vec![
                crate::effect::Effect::add_mana(vec![ManaSymbol::Green]),
                crate::effect::Effect::gain_life(1),
            ],
        );
        let life_source = mana_land(
            &mut game,
            alice,
            "Life cost",
            &[vec![ManaSymbol::Blue]],
            false,
            Some(true),
        );
        restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        game.refresh_continuous_state().unwrap();
        assert!(
            super::super::sources::ManaSourceAnalysis::new(&game)
                .project(&preview_choice(source, 0))
                .is_none(),
            "exercise shared full simulation, not the reviewed projection"
        );
        for symbol in [ManaSymbol::Green, ManaSymbol::Blue, ManaSymbol::Colorless] {
            for nested in [false, true] {
                let mut request = request(
                    &game,
                    alice,
                    source,
                    ManaCost::from_pips(vec![vec![symbol]]),
                );
                request.preferences.excluded_sources = vec![source, life_source];
                if nested {
                    request.reason = crate::costs::PaymentReason::ActivateManaAbility;
                }
                let mut ready_request = request.clone();
                ready_request.preferences = Default::default();
                let expected_ready =
                    mana_payment_ready_activation_inventory(&game, &ready_request, || {
                        SelectFirstDecisionMaker
                    });
                let expected_manual = useful_manual_mana_abilities(&game, &request);
                let (ready, manual) =
                    mana_payment_ready_and_manual_inventory(&game, &request, || {
                        SelectFirstDecisionMaker
                    });
                assert_eq!(ready, expected_ready, "{symbol:?}, nested={nested}");
                assert_eq!(manual, expected_manual, "{symbol:?}, nested={nested}");
                if symbol == ManaSymbol::Green && !nested {
                    assert!(
                        manual.contains(&(source, 0)),
                        "resolved tap-only source must actually be useful"
                    );
                    assert!(ready.iter().any(|option| option.source == source));
                }
                request.allow_mana_abilities = false;
                let (ready, manual) =
                    mana_payment_ready_and_manual_inventory(&game, &request, || {
                        SelectFirstDecisionMaker
                    });
                assert!(ready.is_empty() && manual.is_empty());
            }
        }
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
    }

    #[test]
    fn projected_manual_sources_match_authoritative_triggered_and_restricted_credit() {
        let (mut game, alice) = game();
        let restricted = restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        game.object_mut(restricted)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::You,
                    crate::target::ObjectFilter::land(),
                ),
                vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Blue])],
            ));
        mana_land(
            &mut game,
            alice,
            "Snow source",
            &[vec![ManaSymbol::Green]],
            true,
            Some(false),
        );
        mana_land(
            &mut game,
            alice,
            "Life source",
            &[vec![ManaSymbol::Red]],
            false,
            Some(true),
        );
        game.refresh_continuous_state().unwrap();
        let source = game.new_object_id();
        for symbol in [
            ManaSymbol::Blue,
            ManaSymbol::Green,
            ManaSymbol::Red,
            ManaSymbol::Snow,
            ManaSymbol::Colorless,
        ] {
            let request = request(
                &game,
                alice,
                source,
                ManaCost::from_pips(vec![vec![symbol]]),
            );
            let projected = useful_manual_mana_abilities_inner(&game, &request, true);
            let authoritative = useful_manual_mana_abilities_inner(&game, &request, false);
            assert_eq!(projected, authoritative, "{symbol:?}");
            if symbol == ManaSymbol::Blue {
                assert!(projected.contains(&(restricted, 0)));
            }
        }
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
    }

    #[test]
    fn conditional_tap_source_can_be_unlocked_during_payment() {
        let (mut game, alice) = game();
        let first = mana_land(
            &mut game,
            alice,
            "First source",
            &[vec![ManaSymbol::Green]],
            false,
            Some(false),
        );
        let second = mana_land(
            &mut game,
            alice,
            "Unlocked source",
            &[vec![ManaSymbol::Blue]],
            false,
            Some(false),
        );
        let mut tapped = crate::filter::ObjectFilter::land().you_control();
        tapped.tapped = true;
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(second).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.activation_condition = Some(crate::effect::Condition::ValueComparison {
            left: crate::effect::Value::Count(tapped),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(1),
        });
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );
        let witness = preview_sequence(
            &game,
            &request,
            vec![preview_choice(first, 0), preview_choice(second, 0)],
        );
        assert!(can_pay_request(&witness.0, &request));
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 2);
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut dm),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn restricted_mana_source_can_unlock_another_source_before_payment() {
        let (mut game, alice) = game();
        let first = restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        let second = mana_land(
            &mut game,
            alice,
            "Unlocked source",
            &[vec![ManaSymbol::Blue]],
            false,
            Some(false),
        );
        let mut tapped = crate::filter::ObjectFilter::land().you_control();
        tapped.tapped = true;
        let AbilityKind::Activated(ability) =
            &mut game.object_mut(second).unwrap().abilities_mut()[0].kind
        else {
            unreachable!()
        };
        ability.activation_condition = Some(crate::effect::Condition::ValueComparison {
            left: crate::effect::Value::Count(tapped),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(1),
        });
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );
        let witness = preview_sequence(
            &game,
            &request,
            vec![preview_choice(first, 0), preview_choice(second, 0)],
        );
        assert!(can_pay_request(&witness.0, &request));
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 2);
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut dm),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
    }

    #[test]
    fn conditional_tap_sources_cannot_both_use_an_initially_true_condition() {
        let (mut game, alice) = game();
        let mut tapped = crate::filter::ObjectFilter::land().you_control();
        tapped.tapped = true;
        for index in 0..2 {
            let land = mana_land(
                &mut game,
                alice,
                &format!("Conditional source {index}"),
                &[vec![ManaSymbol::Blue]],
                false,
                Some(false),
            );
            let AbilityKind::Activated(ability) =
                &mut game.object_mut(land).unwrap().abilities_mut()[0].kind
            else {
                unreachable!()
            };
            ability.activation_condition = Some(crate::effect::Condition::ValueComparison {
                left: crate::effect::Value::Count(tapped.clone()),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: crate::effect::Value::Fixed(0),
            });
        }
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; 2]),
        );
        assert!(matches!(
            plan_first_mana_payment(&game, &request),
            Err(ManaPaymentFailure::NoLegalPlan)
        ));
    }

    #[test]
    fn complete_search_conditional_filter_chain_reaches_payment_beyond_old_depth_cutoff() {
        let (mut game, alice) = game();
        game.player_mut(alice).unwrap().mana_pool.green = 1;
        let mut tapped = crate::filter::ObjectFilter::land().you_control();
        tapped.tapped = true;
        for index in 0..=10 {
            let output = if index == 10 {
                ManaSymbol::Blue
            } else {
                ManaSymbol::Green
            };
            let land = mana_land(
                &mut game,
                alice,
                &format!("Conditional filter {index}"),
                &[vec![output]],
                false,
                Some(true),
            );
            let AbilityKind::Activated(ability) =
                &mut game.object_mut(land).unwrap().abilities_mut()[0].kind
            else {
                unreachable!()
            };
            ability.mana_cost = crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::tap(),
                crate::costs::Cost::life(1),
                crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Green]])),
            ]);
            ability.activation_condition = Some(crate::effect::Condition::ValueComparison {
                left: crate::effect::Value::Count(tapped.clone()),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: crate::effect::Value::Fixed(index),
            });
        }
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );
        let mut witness = game.clone();
        for index in 0..=10 {
            let choices = collect_activation_choices(&witness, &request);
            assert_eq!(
                choices.len(),
                1,
                "one conditional filter at step {index}: {choices:?}"
            );
            let (_, next, _) = prepare_owned_activation(witness, &request, choices[0].clone())
                .unwrap_or_else(|| panic!("conditional filter must execute at step {index}"));
            witness = next;
        }
        assert!(can_pay_request(&witness, &request));
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 11);
        assert!(
            plan.mana_ability_steps.len() > expanded_pip_count(&request) + MAX_EXTRA_ACTIVATIONS
        );
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut dm),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().life, 9);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }

    #[test]
    fn complete_search_reversible_zero_net_mana_filter_exhausts_equivalent_states() {
        let (mut game, alice) = game();
        game.player_mut(alice).unwrap().mana_pool.green = 1;
        let card = CardBuilder::new(CardId::new(), "Repeatable filter")
            .card_types(vec![CardType::Artifact])
            .build();
        let filter = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(filter)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility::mana_with_costs(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::mana(
                        ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
                    )),
                    vec![],
                    vec![ManaSymbol::Green],
                )),
                functional_zones: vec![Zone::Battlefield],
            });
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]; 2]),
        );
        let mut search = CandidateSearch::new(game, &request, 1, true, true);
        let result = (0..30)
            .find_map(|_| search.step(&request, &mut 1))
            .expect("equivalent reversible filter states must terminate")
            .unwrap();
        assert!(result.is_empty());
        assert!(
            (2..5).contains(&search.visited),
            "the first provenance change is retained before cycle deduplication"
        );
    }

    #[test]
    fn sliced_search_retains_and_resumes_depth_frontier() {
        let (mut game, alice) = game();
        for index in 0..3 {
            mana_land(
                &mut game,
                alice,
                &format!("Source {index}"),
                &[vec![ManaSymbol::Green]],
                false,
                Some(false),
            );
        }
        let source = game.new_object_id();
        let request = request(&game, alice, source, ManaCost::new().add_generic(3));
        // Start below the known witness depth to exercise continuation rather
        // than relying on the planner's initial cost-based ordering hint.
        let mut search = CandidateSearch::new(game.clone(), &request, 1, true, true);
        let result = (0..200)
            .find_map(|_| search.step(&request, &mut 1))
            .expect("a finite tap-only search must finish across slices")
            .unwrap();
        assert_eq!(result[0].1.len(), 3);
        assert!(search.depth_limit > 1);
        assert!(game.battlefield.iter().all(|id| !game.is_tapped(*id)));
    }

    #[test]
    fn restricted_source_can_pay_an_effect_with_unrestricted_triggered_mana() {
        let (mut game, alice) = game();
        let source = restricted_mana_land(&mut game, alice, vec![CardType::Creature]);
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::You,
                    crate::target::ObjectFilter::land(),
                ),
                vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Green])],
            ));
        game.refresh_continuous_state().unwrap();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        );
        let projected = super::super::analytic::try_projected_candidates(&game, &request)
            .expect("the unrestricted trigger must remain usable on a restricted source");
        assert_eq!(projected[0].1.len(), 1);
        assert_eq!(
            projected[0].0.player(alice).unwrap().restricted_mana.len(),
            1
        );
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert!(!game.is_tapped(source));
        let mut dm = SelectFirstDecisionMaker;
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut dm),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
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
        // `Some(true)` adds life to tap; the mana constructor also adds tap
        // for `None`. Repeatable fixtures explicitly replace the built cost.
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

    fn preview_choice(source: ObjectId, ability_index: usize) -> ActivationChoice {
        ActivationChoice {
            source,
            ability_index,
            stored_color_choices: Vec::new(),
            color_restriction: None,
            flexibility: 1,
            replacement_witnesses: None,
        }
    }

    fn preview_sequence(
        game: &GameState,
        request: &ManaPaymentRequest,
        choices: Vec<ActivationChoice>,
    ) -> Candidate {
        let mut staged = game.clone();
        let mut steps = Vec::new();
        for choice in choices {
            let (_, next, activation) = prepare_owned_activation(staged, request, choice).unwrap();
            staged = next;
            steps.push(activation);
        }
        (staged, steps)
    }

    fn finish_cleanup(
        root: &GameState,
        request: &ManaPaymentRequest,
        candidate: Candidate,
    ) -> Candidate {
        let mut cleanup = ProposalCleanup::new(root.clone(), candidate.0, candidate.1);
        for _ in 0..66 {
            if let Some(candidate) = cleanup.step(request) {
                return candidate;
            }
        }
        panic!("cleanup exceeded its deterministic work bound");
    }

    #[test]
    fn greedy_preview_pays_one_red_red_with_three_lands_and_saves_treasure() {
        use ManaSymbol::{Red as R, White as W};
        let (mut game, alice) = game();
        let plains = mana_land(
            &mut game,
            alice,
            "White land",
            &[vec![W]],
            false,
            Some(false),
        );
        let passage = mana_land(&mut game, alice, "Red land", &[vec![R]], false, Some(false));
        let foundry = mana_land(
            &mut game,
            alice,
            "Dual land",
            &[vec![W], vec![R]],
            false,
            Some(false),
        );
        let treasure = mana_land(
            &mut game,
            alice,
            "Consumable",
            &[vec![W]],
            false,
            Some(false),
        );
        game.object_mut(treasure).unwrap().abilities_mut()[0] = crate::Ability::mana(
            crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::tap(),
                crate::costs::Cost::sacrifice_self(),
            ]),
            vec![W],
        );
        let dragon_land = mana_land(
            &mut game,
            alice,
            "Other red land",
            &[vec![R]],
            false,
            Some(false),
        );
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![R], vec![R]]),
        );
        // Exercise the lazy fallback directly, irrespective of assignment coverage.
        let mut search = CandidateSearch::new(game.clone(), &request, 10, true, true);
        search.preview_root = Some(game.clone());
        let candidates = search.step(&request, &mut usize::MAX).unwrap().unwrap();
        assert_eq!(
            search.visited, 4,
            "follow only the three useful activations"
        );
        assert_eq!(candidates[0].1.len(), 3);
        assert!(candidates[0].1.iter().all(|step| step.source != treasure));
        assert_eq!(candidates[0].0.player(alice).unwrap().mana_pool.total(), 3);
        assert!(can_pay_request(&candidates[0].0, &request));
        assert!(
            game.battlefield
                .iter()
                .all(|source| !game.is_tapped(*source))
        );
        // Also repair the exact shape of the previously displayed five-source plan.
        let bad = preview_sequence(
            &game,
            &request,
            vec![
                preview_choice(plains, 0),
                preview_choice(passage, 0),
                preview_choice(foundry, 0),
                preview_choice(treasure, 0),
                preview_choice(dragon_land, 0),
            ],
        );
        let clean = finish_cleanup(&game, &request, bad);
        assert_eq!(clean.1.len(), 3);
        assert!(clean.1.iter().all(|step| step.source != treasure));
        assert!(can_pay_request(&clean.0, &request));
    }

    #[test]
    fn greedy_preview_prefers_a_bundle_that_covers_more_unpaid_pips() {
        let (mut game, alice) = game();
        mana_land(
            &mut game,
            alice,
            "Small",
            &[vec![ManaSymbol::Red]],
            false,
            Some(false),
        );
        let bundle = mana_land(
            &mut game,
            alice,
            "Bundle",
            &[vec![ManaSymbol::Red, ManaSymbol::Red, ManaSymbol::Green]],
            false,
            Some(false),
        );
        let source = game.new_object_id();
        let request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Red],
                vec![ManaSymbol::Green],
            ]),
        );
        let mut search = CandidateSearch::new(game.clone(), &request, 10, true, true);
        let candidates = search.step(&request, &mut usize::MAX).unwrap().unwrap();
        assert_eq!(candidates[0].1.len(), 1);
        assert_eq!(candidates[0].1[0].source, bundle);
        let projected = plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(
            projected.mana_ability_steps.len(),
            1,
            "compact assignment also removes redundant sources"
        );
        assert_eq!(projected.mana_ability_steps[0].source, bundle);
    }

    #[test]
    fn preview_cleanup_keeps_filter_dependencies_and_exact_user_selections() {
        let (mut game, alice) = game();
        let extra = mana_land(
            &mut game,
            alice,
            "Extra",
            &[vec![ManaSymbol::White]],
            false,
            Some(false),
        );
        let green = mana_land(
            &mut game,
            alice,
            "Seed",
            &[vec![ManaSymbol::Green]],
            false,
            Some(false),
        );
        let filter = mana_land(
            &mut game,
            alice,
            "Filter",
            &[vec![ManaSymbol::Red; 2]],
            false,
            Some(false),
        );
        game.object_mut(filter).unwrap().abilities_mut()[0] = crate::Ability::mana(
            crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::tap(),
                crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Green]])),
            ]),
            vec![ManaSymbol::Red; 2],
        );
        let source = game.new_object_id();
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Red]; 2]),
        );
        let sequence = vec![
            preview_choice(extra, 0),
            preview_choice(green, 0),
            preview_choice(filter, 0),
        ];
        let clean = finish_cleanup(
            &game,
            &request,
            preview_sequence(&game, &request, sequence.clone()),
        );
        assert_eq!(
            clean.1.iter().map(|step| step.source).collect::<Vec<_>>(),
            vec![green, filter]
        );
        assert!(can_pay_request(&clean.0, &request));
        request
            .preferences
            .required_activations
            .push(RequiredManaActivation {
                source: extra,
                ability_index: 0,
                color_restriction: None,
            });
        let pinned = finish_cleanup(&game, &request, preview_sequence(&game, &request, sequence));
        assert_eq!(
            pinned.1.len(),
            3,
            "user-selected surplus must not be removed"
        );
    }

    #[test]
    fn preview_cleanup_replays_triggered_mana_and_preserves_reserved_resources() {
        let (mut game, alice) = game();
        let extra = mana_land(
            &mut game,
            alice,
            "Extra",
            &[vec![ManaSymbol::White]],
            false,
            Some(false),
        );
        let bonus = mana_land(
            &mut game,
            alice,
            "Triggered land",
            &[vec![ManaSymbol::Red]],
            false,
            Some(false),
        );
        game.object_mut(bonus)
            .unwrap()
            .abilities_mut()
            .push(crate::Ability::triggered(
                crate::triggers::Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::You,
                    crate::target::ObjectFilter::land(),
                ),
                vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Red])],
            ));
        let reserved = mana_land(
            &mut game,
            alice,
            "Reserved for convoke",
            &[vec![ManaSymbol::Red]],
            false,
            Some(false),
        );
        let source = game.new_object_id();
        let mut request = request(
            &game,
            alice,
            source,
            ManaCost::from_pips(vec![vec![ManaSymbol::Red]; 2]),
        );
        request.reserved_tap_sources.push(reserved);
        let clean = finish_cleanup(
            &game,
            &request,
            preview_sequence(
                &game,
                &request,
                vec![preview_choice(extra, 0), preview_choice(bonus, 0)],
            ),
        );
        assert_eq!(clean.1.len(), 1);
        assert_eq!(clean.1[0].source, bonus);
        assert!(!clean.0.is_tapped(reserved));
        assert!(can_pay_request(&clean.0, &request));
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
        // These fixtures contain only independent fixed-output tap sources
        // (optionally paying life). Every subset/output combination is executed
        // in source order: permuting independent activations cannot create a
        // new payable pool, and ranking all permutations wastes enormous memory.
        // This remains an exhaustive oracle for this declared fixture domain,
        // rather than a node/depth cutoff in the production planner.
        fn independent_sources_can_pay(
            game: &GameState,
            request: &ManaPaymentRequest,
            sources: &[ObjectId],
            index: usize,
        ) -> bool {
            if can_pay_request(game, request) {
                return true;
            }
            let Some(&source) = sources.get(index) else {
                return false;
            };
            if independent_sources_can_pay(game, request, sources, index + 1) {
                return true;
            }
            let abilities = game.object(source).unwrap().abilities_vec();
            for (ability_index, ability) in abilities.iter().enumerate() {
                let AbilityKind::Activated(activated) = &ability.kind else {
                    panic!("non-independent fixture");
                };
                assert!(
                    activated.has_tap_cost()
                        && activated.effects.is_empty()
                        && activated.mana_output.is_some()
                );
                assert!(
                    activated.activation_condition.is_none()
                        && activated.choices.is_empty()
                        && activated.activation_restrictions.is_empty()
                        && activated.additional_restrictions.is_empty()
                );
                assert!(
                    activated
                        .mana_cost
                        .as_all()
                        .unwrap()
                        .iter()
                        .all(|cost| cost.requires_tap() || cost.life_amount().is_some())
                );
                if let Some((_, staged, _)) =
                    prepare_activation(game, request, preview_choice(source, ability_index))
                {
                    if independent_sources_can_pay(&staged, request, sources, index + 1) {
                        return true;
                    }
                }
            }
            false
        }
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
        let mut payable = 0usize;
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
                                checked += 1;
                                let searched = independent_sources_can_pay(
                                    &game,
                                    &request,
                                    &game.battlefield,
                                    0,
                                );
                                payable += usize::from(searched);
                                // Retain independent comparison against the
                                // complete planner on the smaller board matrix.
                                if lands <= 3 {
                                    let full = ManaPaymentPlanner {
                                        skip_affordability_gate: true,
                                        ..Default::default()
                                    }
                                    .first_plan(&game, &request);
                                    assert_eq!(searched, full.is_ok());
                                }
                                if veto {
                                    vetoed += 1;
                                    assert!(
                                        !searched,
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
            payable > 50,
            "oracle must also find executable positives, found {payable}"
        );
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
        let candidates = super::super::analytic::try_projected_candidates(&game, &request)
            .expect("qualified restricted mana should use compact assignment");
        assert_eq!(candidates[0].1[0].source, land);
        assert_eq!(
            candidates[0].0.player(alice).unwrap().restricted_mana.len(),
            1
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
    fn improvise_checks_projected_assignments_before_unknown_mana_fallback() {
        let (mut game, alice) = game();
        let artifact = CardBuilder::new(CardId::new(), "Payment artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let spell = game.create_object_from_card(&artifact, alice, Zone::Hand);
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::improvise(),
            ),
        );
        for _ in 0..8 {
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }
        for pay_life in [false, false, true] {
            let land = CardBuilder::new(CardId::new(), "Payment land")
                .card_types(vec![CardType::Land])
                .build();
            let land = game.create_object_from_card(&land, alice, Zone::Battlefield);
            let costs = if pay_life {
                crate::cost::TotalCost::from_costs(vec![
                    crate::costs::Cost::tap(),
                    crate::costs::Cost::life(1),
                ])
            } else {
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap())
            };
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(costs, vec![ManaSymbol::Blue]));
        }
        game.refresh_continuous_state().unwrap();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)], vec![ManaSymbol::Blue]]),
        );
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        let perf = last_mana_payment_perf();
        assert_eq!(
            perf.searched_selections, 0,
            "unknown life-cost source must not force exhaustive mana-only search before trying improvise"
        );
        assert!(plan.payable);
        assert_eq!(plan.mana_ability_steps.len(), 2);
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn keyword_assignment_stream_reaches_every_resource_subset_beyond_prefix() {
        let (mut game, alice) = game();
        let artifact = CardBuilder::new(CardId::new(), "Alternative resource")
            .card_types(vec![CardType::Artifact])
            .build();
        let spell = game.create_object_from_card(&artifact, alice, Zone::Hand);
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::improvise(),
            ),
        );
        for _ in 0..12 {
            game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        }
        game.refresh_continuous_state().unwrap();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::new().add_generic(5),
        );
        let mut stream = alternative_payment_selections(&game, &request);
        let mut subsets = HashSet::new();
        while let Some(selection) = stream.next_preferred().or_else(|| stream.next_complete()) {
            let mut sources: Vec<_> = selection
                .allocations
                .iter()
                .map(|allocation| match allocation.payment {
                    super::super::PlannedPipPayment::Improvise(source) => source.0,
                    _ => panic!("unexpected keyword resource"),
                })
                .collect();
            sources.sort_unstable();
            subsets.insert(sources);
        }
        // Sum C(12,k), k=0..5; the former two 128-entry prefixes cannot cover it.
        assert_eq!(subsets.len(), 1586);
    }

    #[test]
    fn fifth_monocolored_hybrid_can_be_paid_with_keyword_resources() {
        let (mut game, alice) = game();
        let artifact = CardBuilder::new(CardId::new(), "Hybrid resource")
            .card_types(vec![CardType::Artifact])
            .build();
        let spell = game.create_object_from_card(&artifact, alice, Zone::Hand);
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::improvise(),
            ),
        );
        let resources: Vec<_> = (0..10)
            .map(|_| game.create_object_from_card(&artifact, alice, Zone::Battlefield))
            .collect();
        game.refresh_continuous_state().unwrap();
        let mut request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2), ManaSymbol::Blue]; 5]),
        );
        request.allow_mana_abilities = false;
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        assert!(plan.payable);
        assert_eq!(plan.allocations.len(), 10);
        assert_eq!(
            execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker),
            Ok(super::super::ManaPaymentExecution::Paid)
        );
        assert!(resources.into_iter().all(|source| game.is_tapped(source)));
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
    fn delve_preview_rejects_unfundable_residual_costs_before_search() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Delve resource").build();
        let spell = game.create_object_from_card(&card, alice, Zone::Stack);
        game.object_mut(spell).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::delve()));
        for _ in 0..30 {
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }
        for _ in 0..2 {
            mana_land(
                &mut game,
                alice,
                "Blue source",
                &[vec![ManaSymbol::Blue]],
                false,
                Some(false),
            );
        }
        game.refresh_continuous_state().unwrap();
        let request = ManaPaymentRequest::new(
            alice,
            spell,
            crate::costs::PaymentReason::CastSpell,
            ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(12)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
            ]),
        );
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        let perf = last_mana_payment_perf();
        assert_eq!(plan.mana_ability_steps.len(), 2);
        assert_eq!(
            plan.allocations
                .iter()
                .filter(|allocation| matches!(
                    allocation.payment,
                    super::super::PlannedPipPayment::Delve(_)
                ))
                .count(),
            12
        );
        assert_eq!(
            perf.searched_selections, 0,
            "unfundable residual costs must not launch state search"
        );
        assert_eq!(perf.analytic_selections, 1);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 30);
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
    fn exact_activation_constraint_survives_equivalent_pool_paths() {
        let (mut game, alice) = game();
        let card = CardBuilder::new(CardId::new(), "Equivalent output source")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&card, alice, Zone::Battlefield);
        for _ in 0..2 {
            game.object_mut(land)
                .unwrap()
                .abilities_mut()
                .push(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![ManaSymbol::Blue],
                ));
        }
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
        let plan = plan_first_mana_payment(&game, &request)
            .expect("the second ability remains legal even when the first produces the same pool");
        assert!(plan.payable);
        assert_eq!(plan.mana_ability_steps.len(), 1);
        assert_eq!(plan.mana_ability_steps[0].source, land);
        assert_eq!(plan.mana_ability_steps[0].ability_index, 1);
        assert!(
            !game.is_tapped(land),
            "planning must not activate the real source"
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
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
    fn activation_inventory_reports_finite_and_once_per_turn_capacity() {
        let (mut game, alice) = game();
        game.player_mut(alice).unwrap().life = 5;
        let card = CardBuilder::new(CardId::new(), "Limited mana source")
            .card_types(vec![CardType::Artifact])
            .build();
        let finite = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(finite)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: crate::ability::AbilityKind::Activated(
                    crate::ability::ActivatedAbility::mana_with_costs(
                        crate::cost::TotalCost::from_cost(crate::costs::Cost::life(2)),
                        vec![],
                        vec![ManaSymbol::Green],
                    ),
                ),
                functional_zones: vec![Zone::Battlefield],
            });
        let battery = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(battery)
            .unwrap()
            .add_counters(crate::CounterType::Charge, 3);
        game.object_mut(battery)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: crate::ability::AbilityKind::Activated(
                    crate::ability::ActivatedAbility::mana_with_costs(
                        crate::cost::TotalCost::from_cost(crate::costs::Cost::remove_counters(
                            crate::CounterType::Charge,
                            1,
                        )),
                        vec![],
                        vec![ManaSymbol::Green],
                    ),
                ),
                functional_zones: vec![Zone::Battlefield],
            });
        let once = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(once)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: crate::ability::AbilityKind::Activated(
                    crate::ability::ActivatedAbility::mana_with_costs(
                        crate::cost::TotalCost::free(),
                        vec![],
                        vec![ManaSymbol::Green],
                    )
                    .once_per_turn(),
                ),
                functional_zones: vec![Zone::Battlefield],
            });
        let source = game.new_object_id();
        let request = request(&game, alice, source, ManaCost::new().add_generic(4));
        let options = mana_payment_activation_inventory(&game, &request);
        let finite_option = options
            .iter()
            .find(|option| option.source == finite)
            .unwrap();
        assert!(finite_option.repeatable);
        assert_eq!(finite_option.max_activations, 2);
        let once_option = options.iter().find(|option| option.source == once).unwrap();
        assert!(!once_option.repeatable);
        assert_eq!(once_option.max_activations, 1);
        assert_eq!(
            options
                .iter()
                .find(|option| option.source == battery)
                .unwrap()
                .max_activations,
            3
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            5,
            "capacity probes must not pay real costs"
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

#[cfg(test)]
mod token_resource_failure_tests {
    use super::*;
    use crate::effects::{EffectContext, EffectExecutor, ExecutionError};
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    fn fixture() -> (GameState, PlayerId, ObjectId, ManaPaymentRequest) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId::from_index(0);
        game.turn.active_player = player;
        game.turn.priority_player = Some(player);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        let card = crate::card::CardBuilder::new(CardId::new(), "Mana resource fixture")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let source = game.create_object_from_card(&card, player, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                vec![ManaSymbol::Green],
            ));
        game.effect_store.replacement_effects.add_resolution_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                player,
                crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                    crate::target::ObjectFilter::specific(source),
                ),
                crate::replacement::ReplacementAction::Additionally(vec![
                    crate::effect::Effect::new(crate::effects::CreateTokenEffect::you(
                        crate::cards::tokens::treasure_token_definition(),
                        2,
                    )),
                ]),
            ),
        );
        game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
            max_created_tokens: 1,
            ..Default::default()
        });
        let request = ManaPaymentRequest::new(
            player,
            source,
            crate::costs::PaymentReason::Effect,
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Green]),
        );
        game.take_pending_trigger_events();
        (game, player, source, request)
    }

    #[test]
    fn synchronous_and_sliced_queries_preserve_resource_unknown_not_unpayable() {
        for mode in 0..4 {
            let (game, _, source, request) = fixture();
            let result = match mode {
                0 => plan_first_mana_payment(&game, &request).map(|_| ()),
                1 => plan_mana_payment(&game, &request).map(|_| ()),
                2 => check_mana_payment(&game, &request),
                _ => {
                    let mut analysis = ManaPaymentAnalysis::new(&game, request);
                    let mut result = None;
                    for _ in 0..256 {
                        if let Some(done) = analysis.step(1) {
                            result = Some(done.map(|_| ()));
                            break;
                        }
                    }
                    result.expect("one-source bounded query must reach its explicit result")
                }
            };
            assert!(matches!(
                result,
                Err(ManaPaymentFailure::EffectExecutionFailed(
                    ExecutionError::ResourceLimitExceeded { .. }
                ))
            ));
            assert!(!game.is_tapped(source));
            assert_eq!(game.battlefield.len(), 1);
        }
    }

    #[test]
    fn actual_manual_mana_activation_keeps_error_and_restores_tap_and_tokens() {
        let (mut game, _, source, request) = fixture();
        let next = game.next_object_id_counter();
        let error = crate::mana_payment::activate_mana_during_payment(
            &mut game,
            &request,
            source,
            0,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            crate::special_actions::ActionError::ExecutionFailure {
                error: ExecutionError::ResourceLimitExceeded { .. },
                ..
            }
        ));
        assert!(!game.is_tapped(source));
        assert_eq!(game.battlefield.len(), 1);
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn usefulness_preview_does_not_double_charge_exact_affordable_token_creation() {
        let (mut game, _, source, request) = fixture();
        game.set_token_creation_limits(crate::effects::tokens::TokenCreationLimits {
            max_created_tokens: 2,
            ..Default::default()
        });
        assert!(
            crate::mana_payment::manual_mana_abilities_checked(&game, &request)
                .unwrap()
                .contains(&(source, 0))
        );
        assert_eq!(game.battlefield.len(), 1);
        assert!(!game.is_tapped(source));
        assert!(
            crate::mana_payment::activate_mana_during_payment(
                &mut game,
                &request,
                source,
                0,
                &mut SelectFirstDecisionMaker
            )
            .unwrap()
        );
        assert!(game.is_tapped(source));
        assert_eq!(game.battlefield.len(), 3);
    }

    #[test]
    fn bounded_x_resource_unknown_is_not_a_smaller_affordable_maximum() {
        let (mut game, player, source, _) = fixture();
        let effect = crate::effects::PayManaEffect::new(
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::X]),
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You),
        )
        .with_x_maximum(crate::effect::Value::Fixed(1));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, player, &mut dm);
        assert!(matches!(
            effect.execute(&mut game, &mut ctx),
            Err(ExecutionError::ResourceLimitExceeded { .. })
        ));
        assert!(!game.is_tapped(source));
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn pay_mana_effect_does_not_report_declined_after_resource_unknown() {
        let (mut game, player, source, request) = fixture();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(source, player, &mut dm);
        let effect = crate::effects::PayManaEffect::new(
            request.cost,
            crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You),
        );
        assert!(matches!(
            effect.execute(&mut game, &mut ctx),
            Err(ExecutionError::ResourceLimitExceeded { .. })
        ));
        assert!(!game.is_tapped(source));
        assert_eq!(game.battlefield.len(), 1);
    }
}

#[cfg(test)]
mod producer_characteristic_failure_tests {
    use super::*;
    use crate::effects::ExecutionError;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    fn fixture() -> (GameState, PlayerId, ObjectId, ManaPaymentRequest) {
        fixture_with_condition(crate::ConditionExpr::SourceIsTapped)
    }
    fn fixture_with_condition(
        condition: crate::ConditionExpr,
    ) -> (GameState, PlayerId, ObjectId, ManaPaymentRequest) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let player = PlayerId(0);
        game.turn.active_player = player;
        game.turn.priority_player = Some(player);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        let card = crate::card::CardBuilder::new(CardId::new(), "Tap-sensitive producer")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        let source = game.create_object_from_card(&card, player, Zone::Battlefield);
        game.object_mut(source)
            .unwrap()
            .abilities_mut()
            .push(crate::ability::Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                vec![ManaSymbol::Green],
            ));
        let mut model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::haste();
        for _ in 0..140 {
            model = ironsmith_core::StaticAbility::grant_object_ability_for_filter(
                crate::target::ObjectFilter::source(),
                ironsmith_core::Ability::static_ability(model),
                "Source gains a finite child",
            );
        }
        model = model.with_condition(condition);
        game.object_mut(source).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::from_model(model),
            ),
        );
        game.refresh_continuous_state().unwrap();
        assert!(game.continuous_effects_are_tap_sensitive());
        let rule = ironsmith_core::mana::ManaSpendingRestriction::ProducedBy(
            ironsmith_core::mana::ManaProducerFilter::CardType(crate::types::CardType::Land),
        );
        let request = ManaPaymentRequest::new(
            player,
            source,
            crate::costs::PaymentReason::Effect,
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Green])
                .with_spending_restriction(rule),
        );
        (game, player, source, request)
    }
    #[test]
    fn source_discovery_failure_is_typed_for_native_sliced_and_checked_queries() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                for mode in 0..3 {
                    let (game, player, source, request) = fixture();
                    let before = game.next_object_id_counter();
                    let result = match mode {
                        0 => check_mana_payment(&game, &request),
                        1 => plan_first_mana_payment(&game, &request).map(|_| ()),
                        _ => {
                            let mut analysis = ManaPaymentAnalysis::new(&game, request.clone());
                            let mut result = None;
                            for _ in 0..256 {
                                if let Some(done) = analysis.step(1) {
                                    result = Some(done.map(|_| ()));
                                    break;
                                }
                            }
                            result.expect(
                                "one tap-sensitive producer must complete its bounded calculation",
                            )
                        }
                    };
                    assert!(
                        matches!(
                            result,
                            Err(ManaPaymentFailure::EffectExecutionFailed(
                                ExecutionError::ContinuousDiscovery(_)
                            ))
                        ),
                        "{result:?}"
                    );
                    let checked = crate::decision::with_complete_legality_query(&game, |checked| {
                        let view = crate::derived_view::DerivedGameView::new(checked);
                        Ok(view.can_potentially_pay_with_reason(
                            player,
                            Some(source),
                            &request.cost,
                            0,
                            request.reason,
                        ))
                    });
                    assert!(
                        matches!(checked, Err(ExecutionError::ContinuousDiscovery(_))),
                        "{checked:?}"
                    );
                    assert!(!game.is_tapped(source));
                    assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
                    assert_eq!(game.next_object_id_counter(), before);
                    assert!(game.effect_store.pending_trigger_events.is_empty());
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn post_credit_discovery_failure_is_not_an_unpayable_candidate() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let (game, player, source, request) =
                    fixture_with_condition(crate::ConditionExpr::ValueComparison {
                        left: crate::effect::Value::UnspentMana(crate::target::PlayerFilter::You),
                        operator: crate::effect::ValueComparisonOperator::GreaterThan,
                        right: crate::effect::Value::Fixed(0),
                    });
                // The producer snapshot is valid before credit. Only the newly
                // added mana activates the finite graph exceeding discovery work.
                let result = check_mana_payment(&game, &request);
                assert!(
                    matches!(
                        result,
                        Err(ManaPaymentFailure::EffectExecutionFailed(
                            ExecutionError::ContinuousDiscovery(_)
                        ))
                    ),
                    "{result:?}"
                );
                assert!(!game.is_tapped(source));
                assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
                assert!(game.effect_store.pending_trigger_events.is_empty());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn actual_production_discovery_failure_restores_tap_pool_and_history() {
        std::thread::Builder::new()
            .stack_size(128 * 1024 * 1024)
            .spawn(|| {
                let (mut game, player, source, _) = fixture();
                let before = game.next_object_id_counter();
                let history = game.turn_store.turn_history.event_records.len();
                let error =
                    crate::special_actions::perform_activate_mana_ability_restricted_colors(
                        &mut game,
                        player,
                        source,
                        0,
                        None,
                        &mut SelectFirstDecisionMaker,
                    )
                    .unwrap_err();
                assert!(
                    matches!(
                        error,
                        crate::special_actions::ActionError::ExecutionFailure {
                            error: ExecutionError::ContinuousDiscovery(_),
                            ..
                        }
                    ),
                    "{error:?}"
                );
                assert!(!game.is_tapped(source));
                assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
                assert_eq!(game.next_object_id_counter(), before);
                assert_eq!(game.turn_store.turn_history.event_records.len(), history);
                assert!(game.effect_store.pending_trigger_events.is_empty());
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
