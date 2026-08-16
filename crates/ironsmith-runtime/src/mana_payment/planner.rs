use std::collections::{HashSet, VecDeque};
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

/// Stateless entry point used by legality, runtime, and UI snapshot code.
pub fn plan_mana_payment(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<Vec<ManaPaymentPlan>, ManaPaymentFailure> {
    ManaPaymentPlanner::default().plan(game, request)
}

/// Return the first legal proposal discovered by the planner's ordered search.
///
/// This is intended for latency-sensitive previews. Callers that need the
/// best bounded-search result should continue to use [`plan_mana_payment`].
pub fn plan_first_mana_payment(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<ManaPaymentPlan, ManaPaymentFailure> {
    ManaPaymentPlanner::default().first_plan(game, request)
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
                let kinds = by_source.entry(source).or_default();
                if !kinds.contains(&super::ManaPaymentSourceKind::Convoke) {
                    kinds.push(super::ManaPaymentSourceKind::Convoke);
                }
            }
        }
        if crate::decision::has_improvise(spell) {
            for source in crate::decision::get_improvise_artifacts(game, request.payer) {
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

/// Validate and execute a plan outside the priority cast/activation pipeline.
/// The caller's enclosing replay checkpoint remains responsible for surfacing
/// any nested decision made by a complex mana ability.
pub fn execute_mana_payment_plan(
    game: &mut GameState,
    request: &ManaPaymentRequest,
    expected_plan: &ManaPaymentPlan,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> Result<super::ManaPaymentExecution, ManaPaymentFailure> {
    let current = plan_mana_payment(game, request)?
        .into_iter()
        .find(|plan| plan.id == expected_plan.id && plan.request_hash == expected_plan.request_hash)
        .ok_or(ManaPaymentFailure::StalePlan)?;
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
    Ok(super::ManaPaymentExecution::Paid)
}

#[derive(Debug, Default)]
pub struct ManaPaymentPlanner {
    visited_nodes: usize,
}

#[derive(Debug, Clone)]
struct SearchStep {
    activation: PlannedManaActivation,
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
        self.plan_internal(game, request, true)?
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

        let pool_before = player.mana_pool.clone();
        let mut plans = Vec::new();
        for selection in alternative_payment_selections(game, request) {
            if plans.len() >= MAX_TOTAL_PLANS {
                break;
            }
            let mut staged = game.clone();
            for allocation in &selection.allocations {
                match allocation.payment {
                    super::PlannedPipPayment::Convoke(source)
                    | super::PlannedPipPayment::Improvise(source) => staged.tap(source),
                    _ => {}
                }
            }

            let mut payment_request = request.clone();
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
                    | super::PlannedPipPayment::Improvise(source) => source,
                    _ => continue,
                };
                payment_request
                    .preferences
                    .required_sources
                    .retain(|required| *required != source);
            }

            let payable_without_activations = can_pay_request(&staged, &payment_request);
            let seek_zero_life_plan = payment_request.allow_mana_abilities
                && payable_without_activations
                && !payment_request.preferences.prefer_life
                && preview_life_to_pay(&staged, &payment_request) > 0;
            let candidates = if payable_without_activations
                && payment_request.preferences.required_sources.is_empty()
                && payment_request.preferences.required_activations.is_empty()
                && !seek_zero_life_plan
            {
                vec![(staged, Vec::new())]
            } else if request.allow_mana_abilities {
                self.visited_nodes = 0;
                let depth_limit = expanded_pip_count(&payment_request)
                    .saturating_add(MAX_EXTRA_ACTIVATIONS)
                    .max(payment_request.preferences.required_activations.len())
                    .max(1);
                let found = self.search_candidates(
                    staged,
                    &payment_request,
                    depth_limit,
                    stop_after_first,
                )?;
                if found.is_empty() {
                    continue;
                }
                found
            } else {
                continue;
            };
            for (final_game, steps) in candidates {
                let pool_after = final_game
                    .player(request.payer)
                    .map(|player| player.mana_pool.clone())
                    .ok_or(ManaPaymentFailure::MissingPlayer)?;
                plans.push(build_plan(
                    &final_game,
                    request,
                    &payment_request,
                    &selection,
                    pool_before.clone(),
                    pool_after,
                    steps,
                ));
                if stop_after_first || plans.len() >= MAX_TOTAL_PLANS {
                    break;
                }
            }
            if stop_after_first && !plans.is_empty() {
                break;
            }
        }
        plans.sort_by_key(|plan| plan.score);
        plans.dedup_by_key(|plan| plan.id);
        (!plans.is_empty())
            .then_some(plans)
            .ok_or(ManaPaymentFailure::NoLegalPlan)
    }

    fn search_candidates(
        &mut self,
        game: GameState,
        request: &ManaPaymentRequest,
        depth_limit: usize,
        stop_after_first: bool,
    ) -> Result<Vec<(GameState, Vec<PlannedManaActivation>)>, ManaPaymentFailure> {
        let mut queue = VecDeque::from([(game, Vec::<SearchStep>::new())]);
        let mut seen_safe_states = HashSet::new();
        if let Some((initial, _)) = queue.front() {
            seen_safe_states.insert(safe_search_state_key(initial, request.payer));
        }
        let mut enqueued_nodes = 1usize;
        let mut out = Vec::<(ManaPaymentScore, GameState, Vec<PlannedManaActivation>)>::new();
        let mut search_limit_reached = false;

        while let Some((game, path)) = queue.pop_front() {
            self.visited_nodes += 1;
            if self.visited_nodes > MAX_SEARCH_NODES {
                search_limit_reached = true;
                break;
            }

            if can_pay_request(&game, request) && required_activations_are_present(request, &path) {
                let life_to_pay = preview_life_to_pay(&game, request);
                let activations = path
                    .iter()
                    .map(|step| step.activation.clone())
                    .collect::<Vec<_>>();
                let score = search_candidate_score(&game, request, &activations);
                out.push((score, game.clone(), activations));
                // Every score field is non-negative. Breadth-first traversal
                // has already exhausted every shorter activation sequence, so
                // a candidate at the absolute floor of the higher-priority
                // fields cannot be improved by continuing this selection.
                if stop_after_first || score_reaches_search_floor(score) {
                    break;
                }
                out.sort_by_key(|candidate| candidate.0);
                out.truncate(MAX_PLANS_PER_SELECTION);

                // Once this path pays without unwanted life, adding another
                // activation can only make its score worse. Life-paying paths
                // still expand when mana is preferred so a zero-life plan can
                // displace them.
                if request.preferences.prefer_life || life_to_pay == 0 {
                    continue;
                }
            }
            if path.len() >= depth_limit {
                continue;
            }

            let mut prepared_choices = Vec::new();
            for choice in collect_activation_choices(&game, request) {
                let mut staged = game.clone();
                let before = staged
                    .player(request.payer)
                    .map(|player| player.mana_pool.clone())
                    .unwrap_or_default();
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
                    continue;
                }
                let after = staged
                    .player(request.payer)
                    .map(|player| player.mana_pool.clone())
                    .unwrap_or_default();
                if after == before {
                    continue;
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
                } else if !request.preferences.prefer_life
                    && preview_life_to_pay(&staged, request) > 0
                {
                    1
                } else {
                    0
                };
                prepared_choices.push(((completion_rank, preference_key), staged, activation));
            }
            // Breadth-first traversal guarantees that every shorter source
            // sequence is considered before any longer one. The local ordering
            // still makes immediately payable and user-preferred branches land
            // in the bounded queue first.
            prepared_choices.sort_by_key(|candidate| candidate.0);
            for (_, staged, activation) in prepared_choices {
                let mut next_path = path.clone();
                next_path.push(SearchStep { activation });
                // Undo-safe mana activations only mutate the source and mana
                // bookkeeping represented by this key. Deduplicating those
                // states collapses permutations such as A→B and B→A without
                // conflating sacrifice, life, counter, or other irreversible
                // mana abilities.
                if next_path.iter().all(|step| step.activation.undo_safe)
                    && !seen_safe_states.insert(safe_search_state_key(&staged, request.payer))
                {
                    continue;
                }
                if enqueued_nodes >= MAX_SEARCH_NODES {
                    search_limit_reached = true;
                    break;
                }
                queue.push_back((staged, next_path));
                enqueued_nodes += 1;
            }
        }

        if out.is_empty() && search_limit_reached {
            return Err(ManaPaymentFailure::SearchLimitReached);
        }
        out.sort_by_key(|candidate| candidate.0);
        Ok(out
            .into_iter()
            .map(|(_, game, activations)| (game, activations))
            .collect())
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
struct ActivationChoice {
    source: ObjectId,
    ability_index: usize,
    color_restriction: Option<Vec<Color>>,
    flexibility: usize,
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
    let pips = GameState::expanded_payment_pips(&request.cost, request.x_value, actual_black_life)
        .into_iter()
        .enumerate()
        .map(|(index, alternatives)| PaymentPipSlot {
            pip: ManaPipId(index as u32),
            printed_index: index,
            alternatives,
        })
        .collect::<Vec<_>>();

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
                .filter(|(source, _)| !request.preferences.excluded_sources.contains(source))
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
    if crate::decision::has_improvise(source) {
        for artifact in crate::decision::get_improvise_artifacts(game, request.payer) {
            if request.preferences.excluded_sources.contains(&artifact)
                || sources.iter().any(|candidate| candidate.source == artifact)
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

    let mut selected = vec![None; pips.len()];
    let mut selections = Vec::new();
    enumerate_alternative_selections(&pips, &sources, 0, &mut selected, &mut selections);
    selections.sort_by_key(|selection| {
        let selected_sources = selection
            .allocations
            .iter()
            .filter_map(|allocation| match allocation.payment {
                super::PlannedPipPayment::Convoke(source)
                | super::PlannedPipPayment::Improvise(source) => Some(source),
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

fn allocation_matches_required_alternative(
    required: &super::RequiredAlternativePayment,
    allocation: &PlannedPipAllocation,
) -> bool {
    match (required.kind, &allocation.payment) {
        (ManaPaymentSourceKind::Convoke, super::PlannedPipPayment::Convoke(source))
        | (ManaPaymentSourceKind::Improvise, super::PlannedPipPayment::Improvise(source)) => {
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
        for (pip_index, pip) in pips.iter().enumerate() {
            if selected[pip_index].is_some() || !alternative_can_pay(source.kind, &pip.alternatives)
            {
                continue;
            }
            selected[pip_index] = Some(source);
            enumerate_alternative_selections(pips, sources, source_index + 1, selected, out);
            selected[pip_index] = None;
            if out.len() >= MAX_ALTERNATIVE_SELECTIONS {
                break;
            }
        }
    };
    if source.required {
        include_source(selected, out);
    }
    if out.len() < MAX_ALTERNATIVE_SELECTIONS {
        enumerate_alternative_selections(pips, sources, source_index + 1, selected, out);
    }
    if !source.required && out.len() < MAX_ALTERNATIVE_SELECTIONS {
        include_source(selected, out);
    }
}

fn alternative_can_pay(kind: AlternativeKind, pip: &[ManaSymbol]) -> bool {
    pip.iter().any(|symbol| match (kind, symbol) {
        (AlternativeKind::Convoke(_), ManaSymbol::Generic(_)) => true,
        (AlternativeKind::Convoke(colors), ManaSymbol::White) => colors.contains(Color::White),
        (AlternativeKind::Convoke(colors), ManaSymbol::Blue) => colors.contains(Color::Blue),
        (AlternativeKind::Convoke(colors), ManaSymbol::Black) => colors.contains(Color::Black),
        (AlternativeKind::Convoke(colors), ManaSymbol::Red) => colors.contains(Color::Red),
        (AlternativeKind::Convoke(colors), ManaSymbol::Green) => colors.contains(Color::Green),
        (AlternativeKind::Improvise, ManaSymbol::Generic(_)) => true,
        _ => false,
    })
}

fn collect_activation_choices(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<ActivationChoice> {
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
            .unwrap_or_else(|| std::rc::Rc::new(object.abilities_vec()));
        for &ability_index in analysis.mana_ability_indices_for(source) {
            let Some(ability) = abilities.get(ability_index) else {
                continue;
            };
            let AbilityKind::Activated(mana_ability) = &ability.kind else {
                continue;
            };
            if !mana_ability.is_runtime_mana_ability(game, source, request.payer)
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

fn can_pay_request(game: &GameState, request: &ManaPaymentRequest) -> bool {
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
            | super::PlannedPipPayment::Improvise(source) => Some(source),
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

fn request_hash(request: &ManaPaymentRequest) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.payer.hash(&mut hasher);
    request.source.hash(&mut hasher);
    format!("{:?}", request.reason).hash(&mut hasher);
    request.cost.pips().hash(&mut hasher);
    request.x_value.hash(&mut hasher);
    request.allow_mana_abilities.hash(&mut hasher);
    request.allow_life_payment.hash(&mut hasher);
    request.allow_black_life.hash(&mut hasher);
    format!("{:?}", request.spend_policy).hash(&mut hasher);
    request.preferences.required_sources.hash(&mut hasher);
    request.preferences.required_activations.hash(&mut hasher);
    request.preferences.required_alternatives.hash(&mut hasher);
    request.preferences.excluded_sources.hash(&mut hasher);
    request.preferences.preserve_sources.hash(&mut hasher);
    request.preferences.prefer_life.hash(&mut hasher);
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

fn safe_search_state_key(game: &GameState, payer: crate::ids::PlayerId) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Some(player) = game.player(payer) {
        player.life.hash(&mut hasher);
        player.mana_pool.white.hash(&mut hasher);
        player.mana_pool.blue.hash(&mut hasher);
        player.mana_pool.black.hash(&mut hasher);
        player.mana_pool.red.hash(&mut hasher);
        player.mana_pool.green.hash(&mut hasher);
        player.mana_pool.colorless.hash(&mut hasher);
        let mut restricted = player
            .restricted_mana
            .iter()
            .map(|unit| format!("{unit:?}"))
            .collect::<Vec<_>>();
        restricted.sort();
        restricted.hash(&mut hasher);
        let mut provenance = player
            .mana_source_provenance
            .iter()
            .map(|unit| format!("{unit:?}"))
            .collect::<Vec<_>>();
        provenance.sort();
        provenance.hash(&mut hasher);
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

        let tropical =
            crate::cards::CardDefinitionBuilder::new(CardId::new(), "Test Tropical Island")
                .card_types(vec![CardType::Land])
                .parse_text("{T}: Add {G} or {U}.")
                .expect("flexible mana land should parse");
        let tropical = game.create_object_from_definition(&tropical, alice, Zone::Battlefield);
        let volcanic =
            crate::cards::CardDefinitionBuilder::new(CardId::new(), "Test Volcanic Island")
                .card_types(vec![CardType::Land])
                .parse_text("{T}: Add {R} or {U}.")
                .expect("flexible mana land should parse");
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
