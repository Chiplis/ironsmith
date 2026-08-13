use super::*;
use crate::ability::ActivatedAbilityRuntimeExt;
use crate::filter::ObjectFilterExt as _;
use crate::grant::DerivedAlternativeCastRuntimeExt as _;
use crate::perf::PerfTimer;

// ============================================================================
// Mana Payment Flow
// ============================================================================

pub(super) fn decision_context_name(
    ctx: &crate::decisions::context::DecisionContext,
) -> &'static str {
    use crate::decisions::context::DecisionContext;

    match ctx {
        DecisionContext::Boolean(_) => "boolean",
        DecisionContext::TextInput(_) => "text input",
        DecisionContext::SelectObjects(_) => "select objects",
        DecisionContext::SelectOptions(_) => "select options",
        DecisionContext::Targets(_) => "targets",
        DecisionContext::Number(_) => "number",
        DecisionContext::Priority(_) => "priority",
        DecisionContext::Attackers(_) => "attackers",
        DecisionContext::Blockers(_) => "blockers",
        DecisionContext::Order(_) => "order",
        DecisionContext::Modes(_) => "modes",
        DecisionContext::HybridChoice(_) => "hybrid choice",
        DecisionContext::Distribute(_) => "distribute",
        DecisionContext::Colors(_) => "colors",
        DecisionContext::Counters(_) => "counters",
        DecisionContext::Partition(_) => "partition",
        DecisionContext::Proliferate(_) => "proliferate",
        DecisionContext::ManaPayment(_) => "mana payment",
    }
}

fn pay_selected_cost(
    game: &mut GameState,
    cost: &crate::costs::Cost,
    source: ObjectId,
    payer: PlayerId,
    reason: crate::costs::PaymentReason,
    provenance: crate::provenance::ProvNodeId,
    chosen_id: ObjectId,
    choice_tag: Option<&crate::tag::TagKey>,
    tagged_objects: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let processing_mode = cost.processing_mode();
    let effective_choice_tag = choice_tag.cloned().or_else(|| match &processing_mode {
        crate::costs::CostProcessingMode::ExileFromHand { .. }
        | crate::costs::CostProcessingMode::ExileFromGraveyard { .. }
        | crate::costs::CostProcessingMode::ExileObjects { .. } => {
            Some(crate::tag::TagKey::from("exile_cost"))
        }
        _ => None,
    });
    let preserve_chosen_snapshot = matches!(
        processing_mode,
        crate::costs::CostProcessingMode::SacrificeTarget { .. }
    );

    let mut cost_ctx = crate::costs::CostContext::new(source, payer, decision_maker)
        .with_reason(reason)
        .with_pre_chosen_cards(vec![chosen_id])
        .with_provenance(provenance);
    cost_ctx.tagged_objects = tagged_objects.clone();
    let chosen_snapshot = game.object(chosen_id).map(|obj| {
        if preserve_chosen_snapshot {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
        } else {
            crate::snapshot::ObjectSnapshot::from_object(obj, game)
        }
    });
    if let Some(tag) = effective_choice_tag.as_ref()
        && let Some(snapshot) = chosen_snapshot.clone()
    {
        cost_ctx
            .tagged_objects
            .entry(tag.clone())
            .or_default()
            .push(snapshot);
    }

    match cost.pay(game, &mut cost_ctx) {
        Ok(crate::costs::CostPaymentResult::Paid) => {
            if !preserve_chosen_snapshot
                && let Some(tag) = effective_choice_tag.as_ref()
                && let Some(snapshot) = chosen_snapshot.as_ref()
                && let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id)
                && let Some(current) = game.object(current_id)
            {
                let current_snapshot = crate::snapshot::ObjectSnapshot::from_object(current, game);
                let tagged = cost_ctx.tagged_objects.entry(tag.clone()).or_default();
                tagged.retain(|existing| existing.stable_id != snapshot.stable_id);
                tagged.push(current_snapshot);
            }
            *tagged_objects = cost_ctx.tagged_objects;
            Ok(())
        }
        Ok(crate::costs::CostPaymentResult::NeedsChoice(_)) => Err(GameLoopError::InvalidState(
            "Cost still needed a choice after preselection".to_string(),
        )),
        Err(err) => Err(GameLoopError::InvalidState(format!(
            "Failed to pay cost: {err}"
        ))),
    }
}

/// Expand a ManaCost into individual pips, expanding X pips by the chosen value.
/// Also applies hybrid_choices to replace multi-symbol pips with the chosen symbol.
pub(super) fn expand_mana_cost_to_pips(
    cost: &crate::mana::ManaCost,
    x_value: usize,
    hybrid_choices: &[(usize, crate::mana::ManaSymbol)],
) -> Vec<Vec<crate::mana::ManaSymbol>> {
    use crate::mana::ManaSymbol;

    let mut colored_pips = Vec::new();
    let mut generic_pips = Vec::new();

    for (pip_idx, pip) in cost.pips().iter().enumerate() {
        // Check if this is an X pip
        if pip.iter().any(|s| matches!(s, ManaSymbol::X)) {
            // Expand X into x_value generic pips
            for _ in 0..x_value {
                generic_pips.push(vec![ManaSymbol::Generic(1)]);
            }
        } else if pip.iter().all(|s| matches!(s, ManaSymbol::Generic(0))) {
            // Skip Generic(0) pips - they represent zero cost
            continue;
        } else if pip.len() == 1 {
            // Single-symbol pip - check if it's Generic(N) that needs expansion
            if let ManaSymbol::Generic(n) = pip[0] {
                if n > 1 {
                    // Expand Generic(N) into N individual Generic(1) pips
                    for _ in 0..n {
                        generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    }
                    continue;
                } else if n == 1 {
                    generic_pips.push(pip.clone());
                    continue;
                }
            }
            // Colored pip
            colored_pips.push(pip.clone());
        } else {
            // Multi-symbol pip (e.g., hybrid like {B/P} or {W/U})
            // Check if a choice was made during announcement stage
            if let Some((_, chosen_symbol)) = hybrid_choices.iter().find(|(idx, _)| *idx == pip_idx)
            {
                // Use the chosen symbol instead of the full alternatives
                colored_pips.push(vec![*chosen_symbol]);
            } else {
                // No choice made, keep all alternatives (shouldn't happen if announcement worked)
                colored_pips.push(pip.clone());
            }
        }
    }

    // Return colored pips first (more constrained), then generic pips (more flexible)
    colored_pips.extend(generic_pips);
    colored_pips
}

/// Expand a ManaCost into display pips for the UI overlay.
///
/// This keeps original hybrid/Phyrexian symbols intact so the UI can render the
/// printed-looking cost while still following the engine's payment order
/// (colored/constrained pips first, generic pips last).
pub fn expand_mana_cost_to_display_pips(
    cost: &crate::mana::ManaCost,
    x_value: usize,
) -> Vec<Vec<crate::mana::ManaSymbol>> {
    use crate::mana::ManaSymbol;

    let mut colored_pips = Vec::new();
    let mut generic_pips = Vec::new();

    for pip in cost.pips() {
        if pip.iter().any(|s| matches!(s, ManaSymbol::X)) {
            for _ in 0..x_value {
                generic_pips.push(vec![ManaSymbol::Generic(1)]);
            }
            continue;
        }

        if pip.iter().all(|s| matches!(s, ManaSymbol::Generic(0))) {
            continue;
        }

        if pip.len() == 1 {
            if let ManaSymbol::Generic(n) = pip[0] {
                if n > 1 {
                    for _ in 0..n {
                        generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    }
                    continue;
                }
                if n == 1 {
                    generic_pips.push(vec![ManaSymbol::Generic(1)]);
                    continue;
                }
            }
        }

        colored_pips.push(pip.clone());
    }

    colored_pips.extend(generic_pips);
    colored_pips
}

pub fn mana_ability_is_undo_safe(game: &GameState, source: ObjectId, ability_index: usize) -> bool {
    use crate::ability::AbilityKind;

    let Some(object) = game.object(source) else {
        return false;
    };
    let Some(ability) = game.current_ability(source, ability_index) else {
        return false;
    };
    let AbilityKind::Activated(mana_ability) = &ability.kind else {
        return false;
    };
    if !mana_ability.is_runtime_mana_ability(game, source, game.controller_of(object)) {
        return false;
    }

    let costs = mana_ability.mana_cost.costs();
    if costs.is_empty() || !costs.iter().all(|cost| cost.requires_tap()) {
        return false;
    }

    mana_ability.effects.iter().all(|effect| {
        effect
            .producible_mana_symbols(game, source, game.controller_of(object))
            .is_some()
    })
}

pub(super) fn record_immediate_cost_payment(
    trace: &mut Vec<CostStep>,
    cost: &crate::costs::Cost,
    source: ObjectId,
) {
    let _ = trace;
    let _ = cost;
    let _ = source;
}

fn add_spent_pool_delta(spent: &mut ManaPool, before: &ManaPool, after: &ManaPool) {
    spent.white += before.white.saturating_sub(after.white);
    spent.blue += before.blue.saturating_sub(after.blue);
    spent.black += before.black.saturating_sub(after.black);
    spent.red += before.red.saturating_sub(after.red);
    spent.green += before.green.saturating_sub(after.green);
    spent.colorless += before.colorless.saturating_sub(after.colorless);
}

fn execute_planned_mana_activations(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    payer: PlayerId,
    payment: &mut crate::mana_payment::PendingManaPayment,
    undo_locked_by_mana: &mut bool,
    decision_maker: &mut impl DecisionMaker,
) -> Result<bool, GameLoopError> {
    while let Some(step) = payment
        .plan
        .mana_ability_steps
        .get(payment.next_activation)
        .cloned()
    {
        let activation_cost_has_tap =
            activated_ability_has_tap_cost(game, step.source, step.ability_index);
        let events =
            crate::special_actions::perform_activate_mana_ability_restricted_colors_with_events(
                game,
                payer,
                step.source,
                step.ability_index,
                step.color_restriction.clone(),
                decision_maker,
            )
            .map_err(|error| {
                GameLoopError::InvalidState(format!(
                    "planned mana ability is no longer legal: {error}"
                ))
            })?;
        if decision_maker.awaiting_choice() {
            // Replay-based decision makers will rerun this same activation
            // from the enclosing action checkpoint with the captured answer.
            // Do not advance the plan cursor until that replay completes.
            return Ok(true);
        }
        for event in events {
            queue_triggers_from_event(game, trigger_queue, event, false);
        }
        queue_ability_activated_event(
            game,
            trigger_queue,
            decision_maker,
            step.source,
            payer,
            true,
            None,
            activation_cost_has_tap,
        );
        *undo_locked_by_mana |= !step.undo_safe;
        payment.next_activation += 1;
        drain_pending_trigger_events(game, trigger_queue);
    }
    Ok(false)
}

fn execute_planned_keyword_payments(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &mut PendingCast,
    payment: &crate::mana_payment::PendingManaPayment,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    for allocation in &payment.plan.allocations {
        let (permanent_id, effect) = match allocation.payment {
            crate::mana_payment::PlannedPipPayment::Convoke(permanent_id) => {
                (permanent_id, AlternativePaymentEffect::Convoke)
            }
            crate::mana_payment::PlannedPipPayment::Improvise(permanent_id) => {
                (permanent_id, AlternativePaymentEffect::Improvise)
            }
            _ => continue,
        };
        if game.object(permanent_id).is_none() || game.is_tapped(permanent_id) {
            return Err(GameLoopError::InvalidState(format!(
                "planned {effect:?} permanent {permanent_id:?} is no longer available"
            )));
        }
        tap_permanent_with_trigger(game, trigger_queue, permanent_id);
        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::KeywordAction);
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    keyword_action_from_alternative_effect(effect),
                    pending.caster,
                    pending.spell_id,
                    1,
                ),
                event_provenance,
            ),
            true,
        );
        record_keyword_payment_contribution(
            &mut pending.keyword_payment_contributions,
            permanent_id,
            effect,
        );
    }
    drain_pending_trigger_events(game, trigger_queue);
    Ok(())
}

pub(super) fn prompt_pending_mana_ability_payment(
    game: &mut GameState,
    state: &mut PriorityLoopState,
    mut pending: PendingManaAbility,
    subject: String,
) -> Result<GameProgress, GameLoopError> {
    let spend_policy = game.mana_spend_policy(pending.activator, Some(pending.source));
    let mut request = crate::mana_payment::ManaPaymentRequest::new(
        pending.activator,
        pending.source,
        crate::costs::PaymentReason::ActivateManaAbility,
        pending.mana_cost.clone(),
    )
    .with_spend_policy(spend_policy);
    request.preferences.excluded_sources.push(pending.source);
    if let Some(existing) = pending.pending_mana_payment.as_ref() {
        request.preferences = existing.request.preferences.clone();
        if !request
            .preferences
            .excluded_sources
            .contains(&pending.source)
        {
            request.preferences.excluded_sources.push(pending.source);
        }
    }
    request.preferences.normalize();
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(
            pending.activator,
            Some(pending.source),
            crate::costs::PaymentReason::ActivateManaAbility,
        );
    let plan = crate::mana_payment::plan_mana_payment(game, &request)
        .map_err(|failure| {
            state.rollback_action(game);
            GameLoopError::ActionCancelled(format!(
                "the mana ability's activation cost has no legal payment plan: {failure:?}"
            ))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            GameLoopError::InvalidState("planner returned no mana-ability payment plan".to_string())
        })?;
    pending.pending_mana_payment = Some(crate::mana_payment::PendingManaPayment::new(
        request.clone(),
        plan.clone(),
    ));
    state.pending_mana_ability = Some(pending);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::ManaPayment(
            crate::decisions::context::ManaPaymentContext::new(
                request.payer,
                request.source,
                subject,
                request,
                plan,
            ),
        ),
    ))
}

fn refresh_prepared_spell_payment(
    game: &GameState,
    pending: &PendingCast,
    payment: &mut crate::mana_payment::PendingManaPayment,
) -> Result<(), GameLoopError> {
    let mut request = spell_mana_payment_request(game, pending)?;
    request.allow_mana_abilities = false;
    request.preferences = payment.request.preferences.clone();
    for activated in &payment.plan.mana_ability_steps {
        request
            .preferences
            .required_sources
            .retain(|source| *source != activated.source);
    }
    request.preferences.normalize();
    let plan = crate::mana_payment::plan_mana_payment(game, &request)
        .map_err(|failure| {
            GameLoopError::ActionCancelled(format!(
                "the prepared spell payment can no longer pay the selected costs: {failure:?}"
            ))
        })?
        .into_iter()
        .next()
        .ok_or_else(|| {
            GameLoopError::InvalidState("planner returned no prepared spell plan".to_string())
        })?;
    payment.request = request;
    payment.plan = plan;
    payment.next_activation = payment.plan.mana_ability_steps.len();
    Ok(())
}

pub(super) fn commit_prepared_spell_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingCast,
    mut payment: crate::mana_payment::PendingManaPayment,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if let Err(error) = refresh_prepared_spell_payment(game, &pending, &mut payment) {
        state.rollback_action(game);
        return Err(error);
    }
    if let Err(error) = execute_planned_keyword_payments(
        game,
        trigger_queue,
        &mut pending,
        &payment,
        decision_maker,
    ) {
        state.rollback_action(game);
        return Err(error);
    }
    let Some(pool_before) = game
        .player(pending.caster)
        .map(|player| player.mana_pool.clone())
    else {
        state.rollback_action(game);
        return Err(GameLoopError::InvalidState(
            "spell payer is missing".to_string(),
        ));
    };
    if !game.try_pay_mana_cost_with_payment_options(
        payment.request.payer,
        Some(payment.request.source),
        &payment.plan.mana_cost_after_alternatives,
        payment.request.x_value,
        payment.request.reason,
        &payment.request.spend_policy,
        payment.request.allow_life_payment,
        payment.request.allow_black_life,
        payment.request.preferences.prefer_life,
    ) {
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
            "spell payment failed validation and was rolled back".to_string(),
        ));
    }
    let pool_after = game
        .player(pending.caster)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    add_spent_pool_delta(&mut pending.mana_spent_to_cast, &pool_before, &pool_after);
    pending.mana_cost_to_pay = None;
    pending.pending_mana_payment = None;
    pending.stage = spell_stage_after_targets(&pending);
    continue_spell_next_cost_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

pub(super) fn commit_prepared_activation_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    mut payment: crate::mana_payment::PendingManaPayment,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut request = match activation_mana_payment_request(game, &pending) {
        Ok(request) => request,
        Err(error) => {
            state.rollback_action(game);
            return Err(error);
        }
    };
    request.allow_mana_abilities = false;
    request.preferences = payment.request.preferences.clone();
    for activated in &payment.plan.mana_ability_steps {
        request
            .preferences
            .required_sources
            .retain(|source| *source != activated.source);
    }
    request.preferences.normalize();
    let plan = match crate::mana_payment::plan_mana_payment(game, &request) {
        Ok(plans) => plans.into_iter().next().ok_or_else(|| {
            GameLoopError::InvalidState("planner returned no prepared activation plan".to_string())
        })?,
        Err(failure) => {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(format!(
                "the prepared activation payment can no longer pay the selected costs: {failure:?}"
            )));
        }
    };
    payment.request = request;
    payment.plan = plan;
    let Some(pool_before) = game
        .player(pending.activator)
        .map(|player| player.mana_pool.clone())
    else {
        state.rollback_action(game);
        return Err(GameLoopError::InvalidState(
            "activation payer is missing".to_string(),
        ));
    };
    if !game.try_pay_mana_cost_with_payment_options(
        payment.request.payer,
        Some(payment.request.source),
        &payment.plan.mana_cost_after_alternatives,
        payment.request.x_value,
        payment.request.reason,
        &payment.request.spend_policy,
        payment.request.allow_life_payment,
        payment.request.allow_black_life,
        payment.request.preferences.prefer_life,
    ) {
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
            "activation payment failed validation and was rolled back".to_string(),
        ));
    }
    let pool_after = game
        .player(pending.activator)
        .map(|player| player.mana_pool.clone())
        .unwrap_or_default();
    add_spent_pool_delta(
        &mut pending.mana_spent_on_activation,
        &pool_before,
        &pool_after,
    );
    pending.mana_cost_to_pay = None;
    pending.pending_mana_payment = None;
    pending.stage = activation_stage_after_targets(&pending);
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

fn revalidate_authoritative_payment_plan(
    game: &GameState,
    payment: &crate::mana_payment::PendingManaPayment,
    label: &str,
) -> Result<crate::mana_payment::ManaPaymentPlan, GameLoopError> {
    crate::mana_payment::plan_mana_payment(game, &payment.request)
        .map_err(|failure| {
            GameLoopError::ActionCancelled(format!(
                "{label} payment became illegal before confirmation: {failure:?}"
            ))
        })?
        .into_iter()
        .find(|plan| plan.id == payment.plan.id && plan.request_hash == payment.plan.request_hash)
        .ok_or_else(|| {
            GameLoopError::ActionCancelled(format!(
                "{label} payment state changed; request a new plan"
            ))
        })
}

/// Apply a whole-cost payment response. Plan identity and legality are checked
/// again immediately before any irreversible step is executed.
pub(super) fn apply_mana_payment_plan_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &crate::mana_payment::ManaPaymentResponse,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::mana_payment::ManaPaymentResponse;

    if matches!(response, ManaPaymentResponse::Cancel) {
        state.rollback_action(game);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    if let Some(mut pending) = state.pending_mana_ability.take() {
        let mut payment = pending.pending_mana_payment.take().ok_or_else(|| {
            GameLoopError::InvalidState(
                "mana ability has no authoritative payment proposal".to_string(),
            )
        })?;
        match response {
            ManaPaymentResponse::Replan { preferences } => {
                let mut preferences = preferences.clone();
                if !preferences.excluded_sources.contains(&pending.source) {
                    preferences.excluded_sources.push(pending.source);
                }
                preferences.normalize();
                payment.request.preferences = preferences;
                pending.pending_mana_payment = Some(payment);
                let subject = game
                    .object(pending.source)
                    .map(|object| format!("{}'s mana ability", object.name))
                    .unwrap_or_else(|| "mana ability".to_string());
                return prompt_pending_mana_ability_payment(game, state, pending, subject);
            }
            ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if *plan_id == payment.plan.id && *request_hash == payment.plan.request_hash => {}
            ManaPaymentResponse::Confirm { .. } => {
                pending.pending_mana_payment = Some(payment);
                state.pending_mana_ability = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "stale or client-authored mana-ability payment plan".to_string(),
                ));
            }
            ManaPaymentResponse::Cancel => unreachable!(),
        }
        payment.plan = match revalidate_authoritative_payment_plan(game, &payment, "mana-ability") {
            Ok(plan) => plan,
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        };
        match execute_planned_mana_activations(
            game,
            trigger_queue,
            pending.activator,
            &mut payment,
            &mut pending.undo_locked_by_mana,
            decision_maker,
        ) {
            Ok(true) => {
                pending.pending_mana_payment = Some(payment);
                state.pending_mana_ability = Some(pending);
                return Ok(GameProgress::Continue);
            }
            Ok(false) => {}
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        }
        if !game.try_pay_mana_cost_with_payment_options(
            payment.request.payer,
            Some(payment.request.source),
            &payment.plan.mana_cost_after_alternatives,
            payment.request.x_value,
            payment.request.reason,
            &payment.request.spend_policy,
            payment.request.allow_life_payment,
            payment.request.allow_black_life,
            payment.request.preferences.prefer_life,
        ) {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "mana-ability payment failed validation and was rolled back".to_string(),
            ));
        }
        pending.mana_cost = crate::mana::ManaCost::new();
        pending.pending_mana_payment = None;
        if let Err(error) =
            execute_pending_mana_ability(game, trigger_queue, &pending, decision_maker)
        {
            state.rollback_action(game);
            return Err(error);
        }
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    if let Some(mut pending) = state.pending_activation.take() {
        let mut payment = pending.pending_mana_payment.take().ok_or_else(|| {
            GameLoopError::InvalidState(
                "activation has no authoritative mana payment proposal".to_string(),
            )
        })?;
        match response {
            ManaPaymentResponse::Replan { preferences } => {
                let mut preferences = preferences.clone();
                preferences.normalize();
                payment.request.preferences = preferences;
                pending.pending_mana_payment = Some(payment);
                return prompt_activation_mana_ability_window(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                );
            }
            ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if *plan_id == payment.plan.id && *request_hash == payment.plan.request_hash => {}
            ManaPaymentResponse::Confirm { .. } => {
                pending.pending_mana_payment = Some(payment);
                state.pending_activation = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "stale or client-authored activation payment plan".to_string(),
                ));
            }
            ManaPaymentResponse::Cancel => unreachable!(),
        }

        payment.plan = match revalidate_authoritative_payment_plan(game, &payment, "activation") {
            Ok(plan) => plan,
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        };
        match execute_planned_mana_activations(
            game,
            trigger_queue,
            pending.activator,
            &mut payment,
            &mut pending.undo_locked_by_mana,
            decision_maker,
        ) {
            Ok(true) => {
                pending.pending_mana_payment = Some(payment);
                state.pending_activation = Some(pending);
                return Ok(GameProgress::Continue);
            }
            Ok(false) => {}
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        }
        if !pending.remaining_cost_steps.is_empty() {
            pending.pending_mana_payment = Some(payment);
            pending.stage = ActivationStage::ChoosingNextCost;
            return continue_activation(game, trigger_queue, state, pending, decision_maker);
        }
        return commit_prepared_activation_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            payment,
            decision_maker,
        );
    }

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("no cast or activation is awaiting mana payment".to_string())
    })?;
    let mut payment = pending.pending_mana_payment.take().ok_or_else(|| {
        GameLoopError::InvalidState("spell has no authoritative mana payment proposal".to_string())
    })?;
    let is_assist_payment =
        pending.stage == CastStage::PayingAssistMana && payment.request.payer != pending.caster;
    if is_assist_payment {
        match response {
            ManaPaymentResponse::Replan { preferences } => {
                let mut preferences = preferences.clone();
                preferences.normalize();
                payment.request.preferences = preferences;
                pending.pending_mana_payment = Some(payment);
                return prompt_spell_assist_payment_plan(game, state, pending);
            }
            ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if *plan_id == payment.plan.id && *request_hash == payment.plan.request_hash => {}
            ManaPaymentResponse::Confirm { .. } => {
                pending.pending_mana_payment = Some(payment);
                state.pending_cast = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "stale or client-authored Assist payment plan".to_string(),
                ));
            }
            ManaPaymentResponse::Cancel => unreachable!(),
        }
        payment.plan = match revalidate_authoritative_payment_plan(game, &payment, "Assist") {
            Ok(plan) => plan,
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        };
        match execute_planned_mana_activations(
            game,
            trigger_queue,
            payment.request.payer,
            &mut payment,
            &mut pending.undo_locked_by_mana,
            decision_maker,
        ) {
            Ok(true) => {
                pending.pending_mana_payment = Some(payment);
                state.pending_cast = Some(pending);
                return Ok(GameProgress::Continue);
            }
            Ok(false) => {}
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        }
        let Some(pool_before) = game
            .player(payment.request.payer)
            .map(|player| player.mana_pool.clone())
        else {
            state.rollback_action(game);
            return Err(GameLoopError::InvalidState(
                "Assist payer is missing".to_string(),
            ));
        };
        if !game.try_pay_mana_cost_with_payment_options(
            payment.request.payer,
            Some(payment.request.source),
            &payment.plan.mana_cost_after_alternatives,
            payment.request.x_value,
            payment.request.reason,
            &payment.request.spend_policy,
            payment.request.allow_life_payment,
            payment.request.allow_black_life,
            payment.request.preferences.prefer_life,
        ) {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "Assist payment failed validation and was rolled back".to_string(),
            ));
        }
        let pool_after = game
            .player(payment.request.payer)
            .map(|player| player.mana_pool.clone())
            .unwrap_or_default();
        add_spent_pool_delta(
            &mut pending.assist_mana_spent_to_cast,
            &pool_before,
            &pool_after,
        );
        add_spent_pool_delta(&mut pending.mana_spent_to_cast, &pool_before, &pool_after);
        pending.pending_mana_payment = None;
        pending.assist_payment_complete = true;
        pending.display_mana_pips.clear();
        return begin_spell_mana_payment(game, trigger_queue, state, pending, decision_maker);
    }
    match response {
        ManaPaymentResponse::Replan { preferences } => {
            let mut preferences = preferences.clone();
            preferences.normalize();
            payment.request.preferences = preferences;
            pending.pending_mana_payment = Some(payment);
            return prompt_spell_mana_ability_window(
                game,
                trigger_queue,
                state,
                pending,
                decision_maker,
            );
        }
        ManaPaymentResponse::Confirm {
            plan_id,
            request_hash,
        } if *plan_id == payment.plan.id && *request_hash == payment.plan.request_hash => {}
        ManaPaymentResponse::Confirm { .. } => {
            pending.pending_mana_payment = Some(payment);
            state.pending_cast = Some(pending);
            return Err(GameLoopError::InvalidState(
                "stale or client-authored spell payment plan".to_string(),
            ));
        }
        ManaPaymentResponse::Cancel => unreachable!(),
    }

    payment.plan = match revalidate_authoritative_payment_plan(game, &payment, "spell") {
        Ok(plan) => plan,
        Err(error) => {
            state.rollback_action(game);
            return Err(error);
        }
    };
    match execute_planned_mana_activations(
        game,
        trigger_queue,
        pending.caster,
        &mut payment,
        &mut pending.undo_locked_by_mana,
        decision_maker,
    ) {
        Ok(true) => {
            pending.pending_mana_payment = Some(payment);
            state.pending_cast = Some(pending);
            return Ok(GameProgress::Continue);
        }
        Ok(false) => {}
        Err(error) => {
            state.rollback_action(game);
            return Err(error);
        }
    }
    if !pending.remaining_cost_steps.is_empty() {
        pending.pending_mana_payment = Some(payment);
        pending.stage = CastStage::ChoosingNextCost;
        return continue_spell_next_cost_or_finalize(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }
    commit_prepared_spell_mana_payment(game, trigger_queue, state, pending, payment, decision_maker)
}

pub(super) fn apply_modes_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    modes: &[usize],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if state.pending_cast.is_none()
        && let Some(mut pending) = state.pending_activation.take()
    {
        let has_legal_targets = spell_program_has_legal_targets_with_modes(
            game,
            &pending.effects,
            pending.activator,
            Some(pending.source),
            Some(modes),
        );

        if !has_legal_targets {
            return Err(GameLoopError::InvalidState(
                "Selected mode combination has no legal targets".to_string(),
            ));
        }

        pending.chosen_modes = Some(modes.to_vec());
        pending.remaining_requirements = extract_target_requirements_from_program_with_modes(
            game,
            &pending.effects,
            pending.activator,
            Some(pending.source),
            Some(modes),
        );
        pending.stage = activation_stage_after_modes(&pending);
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast or activation for modes response".to_string())
    })?;

    let required_optional_cost =
        match cast_mode_selection_required_optional_cost(game, &pending, modes) {
            Ok(required) => required,
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        };
    let mut hypothetical_game = required_optional_cost.map(|optional_cost_index| {
        let mut hypothetical = game.clone();
        if let Some(spell) = hypothetical.object_mut(pending.spell_id) {
            spell.optional_costs_paid.pay_times(optional_cost_index, 1);
        }
        hypothetical.refresh_continuous_state();
        hypothetical
    });
    let proposal_game = hypothetical_game.as_mut().map_or(&*game, |game| &*game);

    let has_legal_targets = proposal_game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            spell_program_has_legal_targets_with_modes(
                proposal_game,
                program,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        })
        .unwrap_or_else(|| {
            let effects = proposal_game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .map(|program| &**program)
                .unwrap_or(&[]);
            spell_has_legal_targets_with_modes(
                proposal_game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        });

    if !has_legal_targets {
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
            "Selected mode combination has no legal targets".to_string(),
        ));
    }

    // Store the chosen modes
    pending.chosen_modes = Some(modes.to_vec());
    if let Some(optional_cost_index) = required_optional_cost
        && !pending
            .required_optional_cost_indices
            .contains(&optional_cost_index)
    {
        pending
            .required_optional_cost_indices
            .push(optional_cost_index);
    }
    pending.remaining_requirements = proposal_game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                proposal_game,
                program,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        })
        .unwrap_or_else(|| {
            let effects = proposal_game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .map(|program| &**program)
                .unwrap_or(&[]);
            extract_target_requirements_with_modes(
                proposal_game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                Some(modes),
            )
        });

    // Continue through splice and additional/optional costs before announcing X.
    check_splice_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Apply an optional costs response to the pending cast.
pub(super) fn apply_optional_costs_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choices: &[(usize, u32)],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for optional costs response".to_string())
    })?;

    let optional_costs = game
        .object(pending.spell_id)
        .map(|spell| spell.optional_costs.clone())
        .unwrap_or_default();
    let mut announced_counts = std::collections::HashMap::<usize, u32>::new();
    for &(index, times) in choices {
        if times == 0 || index >= optional_costs.len() {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "optional-cost response contains an invalid choice".to_string(),
            ));
        }
        let total = announced_counts.entry(index).or_default();
        *total = total.saturating_add(times);
        if !optional_costs[index].repeatable && *total > 1 {
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "a nonrepeatable optional cost was selected more than once".to_string(),
            ));
        }
    }
    if pending
        .required_optional_cost_indices
        .iter()
        .any(|index| announced_counts.get(index).copied().unwrap_or(0) == 0)
    {
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
            "the chosen modes require an optional cost that was not announced".to_string(),
        ));
    }

    // Store the optional costs paid
    for &(index, times) in choices {
        pending.optional_costs_paid.pay_times(index, times);
    }

    if let Some(spell) = game.object_mut(pending.spell_id) {
        spell.optional_costs_paid = pending.optional_costs_paid.clone();
    }
    // Optional-cost announcements mutate the stack object. Target legality
    // and requirement extraction below must observe one refreshed derived
    // state rather than rebuilding a dirty full-board baseline per candidate.
    game.refresh_continuous_state();

    if pending.optional_costs_paid.was_entwined()
        && let Some(modal_spec) =
            extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
    {
        pending.chosen_modes = Some((0..modal_spec.mode_descriptions.len()).collect());
    }

    let has_legal_targets = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            spell_program_has_legal_targets_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        })
        .unwrap_or_else(|| {
            let effects = game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .map(|program| &**program)
                .unwrap_or(&[]);
            spell_has_legal_targets_with_modes(
                game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        });

    if !has_legal_targets {
        return Err(GameLoopError::InvalidState(
            "Selected optional costs leave the spell with no legal targets".to_string(),
        ));
    }

    pending.remaining_requirements = game
        .object(pending.spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        })
        .unwrap_or_else(|| {
            let effects = game
                .object(pending.spell_id)
                .and_then(|obj| obj.spell_effect.as_deref())
                .map(|program| &**program)
                .unwrap_or(&[]);
            extract_target_requirements_with_modes(
                game,
                effects,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        });

    // CR 601.2b announces X after modes and alternative/additional costs.
    check_x_or_continue(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a hybrid/Phyrexian mana choice response to a pending cast or activation.
///
/// Per MTG rule 601.2b (and 602.2b for abilities), players announce how they'll pay
/// hybrid/Phyrexian costs before choosing targets. This handler stores the choice
/// and either prompts for the next pip or continues to target selection.
pub(super) fn apply_next_hybrid_choice(
    pending_hybrid_pips: &mut Vec<(usize, Vec<crate::mana::ManaSymbol>)>,
    hybrid_choices: &mut Vec<(usize, crate::mana::ManaSymbol)>,
    choice: usize,
    context_label: &str,
) -> Result<(), GameLoopError> {
    if pending_hybrid_pips.is_empty() {
        return Err(GameLoopError::InvalidState(format!(
            "No pending hybrid pips for hybrid choice response{context_label}",
        )));
    }

    let (pip_idx, alternatives) = pending_hybrid_pips.remove(0);
    if choice >= alternatives.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid hybrid choice {} for pip with {} alternatives{context_label}",
            choice,
            alternatives.len()
        )));
    }

    hybrid_choices.push((pip_idx, alternatives[choice]));
    Ok(())
}

pub(super) fn apply_hybrid_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check if this is for a pending cast (spell) or pending activation (ability)
    if let Some(mut pending) = state.pending_cast.take() {
        if let Err(err) = apply_next_hybrid_choice(
            &mut pending.pending_hybrid_pips,
            &mut pending.hybrid_choices,
            choice,
            "",
        ) {
            state.pending_cast = Some(pending);
            return Err(err);
        }

        if !pending.pending_hybrid_pips.is_empty() {
            return prompt_for_next_hybrid_pip(game, state, pending);
        }

        return continue_to_targets_or_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    if let Some(mut pending) = state.pending_activation.take() {
        if let Err(err) = apply_next_hybrid_choice(
            &mut pending.pending_hybrid_pips,
            &mut pending.hybrid_choices,
            choice,
            " (activation)",
        ) {
            state.pending_activation = Some(pending);
            return Err(err);
        }

        // Keep stage as AnnouncingCost and let continue_activation handle the transition
        // This ensures the validation logic runs when all pips have been announced
        pending.stage = ActivationStage::AnnouncingCost;
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    Err(GameLoopError::InvalidState(
        "No pending cast or activation for hybrid choice response".to_string(),
    ))
}

/// Apply a non-mana Assist setup choice for a pending spell cast.
pub(super) fn apply_assist_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for Assist choice".to_string())
    })?;

    match pending.stage.clone() {
        CastStage::ChoosingAssistPlayer => {
            let eligible = eligible_assist_players(game, pending.caster);
            if choice > eligible.len() {
                state.pending_cast = Some(pending);
                return Err(GameLoopError::InvalidState(format!(
                    "Invalid Assist player choice: {choice} > {}",
                    eligible.len()
                )));
            }
            pending.assist_player_choice_made = true;
            if choice == 0 {
                if !spell_mana_payment_is_legal(game, &pending) {
                    state.pending_cast = Some(pending);
                    return Err(GameLoopError::ActionCancelled(
                        "the caster cannot complete this payment without Assist".to_string(),
                    ));
                }
                pending.assist_player = None;
                pending.assist_payment_complete = true;
                return begin_spell_mana_payment(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                );
            }
            pending.assist_player = Some(eligible[choice - 1]);
            if max_assist_generic_contribution(game, &pending) == 0 {
                state.pending_cast = Some(pending);
                return Err(GameLoopError::ActionCancelled(
                    "the selected player cannot complete an Assist payment".to_string(),
                ));
            }
            prompt_spell_assist_contribution(game, state, pending)
        }
        CastStage::ChoosingAssistContribution => {
            let contribution = u32::try_from(choice).map_err(|_| {
                GameLoopError::InvalidState("Assist contribution does not fit in u32".to_string())
            })?;
            let assistant = pending.assist_player.ok_or_else(|| {
                GameLoopError::InvalidState("Assist contribution has no chosen player".to_string())
            })?;
            if !assist_generic_contribution_is_legal(game, &pending, assistant, contribution) {
                state.pending_cast = Some(pending);
                return Err(GameLoopError::ActionCancelled(format!(
                    "Assist contribution {contribution} cannot complete the spell's mana payment"
                )));
            }
            pending.assist_generic_contribution = contribution;
            if contribution == 0 {
                pending.assist_payment_complete = true;
                begin_spell_mana_payment(game, trigger_queue, state, pending, decision_maker)
            } else {
                prompt_spell_assist_payment_plan(game, state, pending)
            }
        }
        stage => {
            state.pending_cast = Some(pending);
            Err(GameLoopError::InvalidState(format!(
                "Assist choice received during {stage}"
            )))
        }
    }
}

pub(super) fn execute_pending_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    use crate::costs::CostContext;
    use crate::effects::ExecutionContext;

    let source_snapshot = game
        .object(pending.source)
        .map(|obj| ObjectSnapshot::from_object(obj, game));

    // Pay the mana cost
    if !game.try_pay_mana_cost_with_reason(
        pending.activator,
        Some(pending.source),
        &pending.mana_cost,
        0,
        crate::costs::PaymentReason::ActivateManaAbility,
    ) {
        return Err(GameLoopError::InvalidState(
            "Failed to pay mana cost".to_string(),
        ));
    }

    // Pay other costs from TotalCost
    let mut cost_ctx = CostContext::new(pending.source, pending.activator, decision_maker)
        .with_reason(crate::costs::PaymentReason::ActivateManaAbility)
        .with_provenance(pending.provenance);
    for c in &pending.other_costs {
        crate::special_actions::pay_cost_component_with_choice(game, c, &mut cost_ctx)
            .map_err(|e| GameLoopError::InvalidState(format!("Failed to pay cost: {e}")))?;
    }
    drain_pending_trigger_events(game, trigger_queue);

    // Add fixed mana to player's pool
    let mana_to_add = crate::events::mana::apply_mana_replacements(
        game,
        pending.source,
        pending.activator,
        pending.activator,
        pending.mana_to_add.clone(),
        pending.mana_production_provenance,
        source_snapshot.clone(),
        decision_maker,
    );
    if !mana_to_add.is_empty() {
        if let Some(player_obj) = game.player_mut(pending.activator) {
            for symbol in &mana_to_add {
                if pending.mana_usage_restrictions.is_empty() {
                    player_obj.add_unrestricted_mana(
                        *symbol,
                        pending.source,
                        source_snapshot.clone(),
                    );
                } else {
                    player_obj.add_restricted_mana_with_snapshot(
                        crate::ability::RestrictedManaUnit {
                            symbol: *symbol,
                            source: pending.source,
                            source_chosen_creature_type: pending.mana_source_chosen_creature_type,
                            restrictions: pending.mana_usage_restrictions.clone(),
                        },
                        source_snapshot.clone(),
                    );
                }
            }
        }
        let event = crate::events::ManaAddedEvent::new(
            pending.source,
            pending.activator,
            pending.activator,
            mana_to_add,
        )
        .with_production_provenance(pending.mana_production_provenance)
        .with_snapshot(source_snapshot.clone())
        .into_trigger_event();
        queue_triggers_from_event(game, trigger_queue, event, false);
    }

    // Execute additional effects (for complex mana abilities)
    if !pending.effects.is_empty() {
        let mut ctx = ExecutionContext::new(pending.source, pending.activator, decision_maker)
            .with_provenance(pending.provenance)
            .with_mana_usage_restrictions(pending.mana_usage_restrictions.clone())
            .with_mana_source_chosen_creature_type(pending.mana_source_chosen_creature_type)
            .with_mana_production_provenance(pending.mana_production_provenance);
        if let Some(snapshot) = source_snapshot.clone() {
            ctx = ctx.with_source_snapshot(snapshot);
        }
        let emitted_events = crate::game_loop::execute_resolution_program(
            game,
            &mut ctx,
            pending.activator,
            pending.source,
            &pending.effects,
            None,
            &[],
        )
        .map_err(|err| GameLoopError::InvalidState(err.to_string()))?;
        queue_triggers_for_events(game, trigger_queue, emitted_events);
        drain_pending_trigger_events(game, trigger_queue);
    }

    game.record_ability_activation(pending.source, pending.ability_index);
    let activation_cost_has_tap =
        activated_ability_has_tap_cost(game, pending.source, pending.ability_index);

    queue_ability_activated_event(
        game,
        trigger_queue,
        &mut *decision_maker,
        pending.source,
        pending.activator,
        true,
        None,
        activation_cost_has_tap,
    );

    Ok(())
}

/// Apply a mana payment response for a pending activation.
pub(super) fn apply_next_cost_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if state
        .pending_activation
        .as_ref()
        .is_some_and(|pending| matches!(pending.stage, ActivationStage::ChoosingAlternativeCost))
    {
        return apply_alternative_activation_cost_response(
            game,
            trigger_queue,
            state,
            choice,
            decision_maker,
        );
    }

    if let Some(mut pending) = state.pending_activation.take() {
        if !matches!(pending.stage, ActivationStage::ChoosingNextCost) {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(
                "Activation next-cost response outside choosing-next-cost stage".to_string(),
            ));
        }

        let has_mana_option = pending.mana_cost_to_pay.is_some();
        if has_mana_option && choice == 0 {
            let payment = pending.pending_mana_payment.take().ok_or_else(|| {
                GameLoopError::InvalidState(
                    "activation mana sources were not prepared before cost payment".to_string(),
                )
            })?;
            return commit_prepared_activation_mana_payment(
                game,
                trigger_queue,
                state,
                pending,
                payment,
                decision_maker,
            );
        }

        let cost_index = choice.saturating_sub(usize::from(has_mana_option));
        if cost_index >= pending.remaining_cost_steps.len() {
            return Err(GameLoopError::InvalidState(format!(
                "Invalid activation next-cost choice: {} >= {}",
                cost_index,
                pending.remaining_cost_steps.len()
            )));
        }

        pending.remaining_cost_steps.swap(0, cost_index);
        pending.stage = ActivationStage::ProcessingCosts;
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState(
            "No pending cast or activation for next-cost response".to_string(),
        )
    })?;
    if !matches!(pending.stage, CastStage::ChoosingNextCost) {
        state.pending_cast = Some(pending);
        return Err(GameLoopError::InvalidState(
            "Spell next-cost response outside choosing-next-cost stage".to_string(),
        ));
    }

    let has_mana_option = pending.mana_cost_to_pay.is_some();
    if has_mana_option && choice == 0 {
        pending
            .remaining_cost_steps
            .retain(|step| delve_generic_reduction(step) == 0);
        let payment = pending.pending_mana_payment.take().ok_or_else(|| {
            GameLoopError::InvalidState(
                "spell mana sources were not prepared before cost payment".to_string(),
            )
        })?;
        return commit_prepared_spell_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            payment,
            decision_maker,
        );
    }

    let cost_index = choice.saturating_sub(usize::from(has_mana_option));
    if cost_index >= pending.remaining_cost_steps.len() {
        return Err(GameLoopError::InvalidState(format!(
            "Invalid spell next-cost choice: {} >= {}",
            cost_index,
            pending.remaining_cost_steps.len()
        )));
    }

    pending.remaining_cost_steps.swap(0, cost_index);
    pending.stage = CastStage::ProcessingCosts;
    continue_spell_cost_payment(game, trigger_queue, state, pending, decision_maker)
}

pub(super) fn apply_alternative_activation_cost_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState(
            "No pending activation for alternative-cost response".to_string(),
        )
    })?;
    if !matches!(pending.stage, ActivationStage::ChoosingAlternativeCost) {
        state.pending_activation = Some(pending);
        return Err(GameLoopError::InvalidState(
            "Alternative-cost response outside choosing-alternative-cost stage".to_string(),
        ));
    }

    let branch = pending
        .alternative_cost_branches
        .get(choice)
        .cloned()
        .ok_or_else(|| {
            GameLoopError::InvalidState(format!(
                "Invalid activation cost branch: {choice} >= {}",
                pending.alternative_cost_branches.len()
            ))
        })?;
    let view = crate::derived_view::DerivedGameView::new(game);
    if !crate::decision::activation_total_cost_branch_is_payable_with_view(
        game,
        pending.activator,
        pending.source,
        &branch,
        &view,
    ) {
        state.pending_activation = Some(pending);
        return Err(GameLoopError::ActionCancelled(
            "the selected activation cost branch cannot be paid".to_string(),
        ));
    }

    assign_pending_activation_cost(game, &mut pending, &branch, decision_maker)?;
    pending.selected_alternative_cost = Some(choice);
    pending.stage = activation_stage_after_modes(&pending);
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply an object-selection response for a pending activation.
pub(super) fn apply_sacrifice_target_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    target_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_activation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending activation for object-choice response".to_string())
    })?;

    match pending.stage {
        ActivationStage::ChoosingSacrifice => {
            let (cost, filter, choice_tag) = match pending.remaining_cost_steps.first() {
                Some(ActivationCostStep::Sacrifice {
                    cost,
                    filter,
                    choice_tag,
                    ..
                }) => (cost.clone(), filter.clone(), choice_tag.clone()),
                _ => {
                    return Err(GameLoopError::InvalidState(
                        "No pending sacrifice cost for activation".to_string(),
                    ));
                }
            };
            let legal_targets = get_legal_sacrifice_targets(
                game,
                pending.activator,
                pending.source,
                &filter,
                pending.payment_reason,
            );
            if !legal_targets.contains(&target_id) {
                return Err(GameLoopError::InvalidState(
                    "Selected permanent is not a legal sacrifice cost choice".to_string(),
                ));
            }

            let choice_tag = choice_tag.unwrap_or_else(|| {
                let tag = format!("sacrifice_cost_{}", pending.next_sacrifice_cost_tag_index);
                pending.next_sacrifice_cost_tag_index += 1;
                crate::tag::TagKey::from(tag)
            });
            pay_selected_cost(
                game,
                &cost,
                pending.source,
                pending.activator,
                pending.payment_reason,
                pending.provenance,
                target_id,
                Some(&choice_tag),
                &mut pending.tagged_objects,
                decision_maker,
            )?;

            drain_pending_trigger_events(game, trigger_queue);

            pending.remaining_cost_steps.remove(0);
            pending.stage = activation_stage_after_targets(&pending);
        }
        ActivationStage::ChoosingCardCost => {
            let next_cost = pending
                .remaining_cost_steps
                .first()
                .and_then(|step| match step {
                    ActivationCostStep::CardChoice(choice) => Some(choice.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "No pending card choice cost for activation".to_string(),
                    )
                })?;

            match next_cost {
                ActivationCardCostChoice::Discard {
                    cost, card_types, ..
                } => {
                    let legal_cards = get_legal_discard_cards(
                        game,
                        pending.activator,
                        pending.source,
                        &card_types,
                    );
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal discard cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromHand {
                    cost, color_filter, ..
                } => {
                    let legal_cards = get_legal_exile_from_hand_cards(
                        game,
                        pending.activator,
                        pending.source,
                        color_filter,
                    );
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal exile-from-hand cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromGraveyard {
                    cost, card_type, ..
                } => {
                    let legal_cards =
                        get_legal_exile_from_graveyard_cards(game, pending.activator, card_type);
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal graveyard exile cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileChosenObject {
                    cost,
                    filter,
                    zone,
                    top_only,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.activator,
                        pending.source,
                        &filter,
                        zone,
                        top_only,
                    );
                    if !legal_objects.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal exile cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::RevealFromHand {
                    cost,
                    card_type,
                    color_filter,
                    ..
                } => {
                    let legal_cards = get_legal_reveal_from_hand_cards(
                        game,
                        pending.activator,
                        pending.source,
                        card_type,
                        color_filter,
                    );
                    if !legal_cards.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal reveal cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;
                }
                ActivationCardCostChoice::ReturnToHand {
                    cost,
                    filter,
                    choice_tag,
                    ..
                } => {
                    let legal_targets = get_legal_return_to_hand_targets(
                        game,
                        pending.activator,
                        pending.source,
                        &filter,
                    );
                    if !legal_targets.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected permanent is not a legal return-to-hand cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        choice_tag.as_ref(),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::MoveChosenObjectToZone {
                    cost,
                    filter,
                    source_zone,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.activator,
                        pending.source,
                        &filter,
                        source_zone,
                        false,
                    );
                    if !legal_objects.contains(&target_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal move-to-zone cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.source,
                        pending.activator,
                        pending.payment_reason,
                        pending.provenance,
                        target_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
            }

            pending.remaining_cost_steps.remove(0);
            pending.stage = activation_stage_after_targets(&pending);
        }
        _ => {
            return Err(GameLoopError::InvalidState(
                "Object-choice response outside activation object-cost stages".to_string(),
            ));
        }
    }

    // Continue activation process
    continue_activation(game, trigger_queue, state, pending, decision_maker)
}

/// Apply a card/object choice response for a pending spell cast cost.
pub(super) fn apply_card_cost_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    chosen_id: ObjectId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for card-cost response".to_string())
    })?;

    match pending.stage {
        CastStage::ChoosingSacrifice => {
            let (cost, filter, choice_tag) = match pending.remaining_cost_steps.first() {
                Some(ActivationCostStep::Sacrifice {
                    cost,
                    filter,
                    choice_tag,
                    ..
                }) => (cost.clone(), filter.clone(), choice_tag.clone()),
                _ => {
                    return Err(GameLoopError::InvalidState(
                        "No pending sacrifice cost for spell cast".to_string(),
                    ));
                }
            };
            let legal_targets = get_legal_sacrifice_targets(
                game,
                pending.caster,
                pending.spell_id,
                &filter,
                crate::costs::PaymentReason::CastSpell,
            );
            if !legal_targets.contains(&chosen_id) {
                return Err(GameLoopError::InvalidState(
                    "Selected permanent is not a legal spell sacrifice cost choice".to_string(),
                ));
            }

            let choice_tag = choice_tag.unwrap_or_else(|| {
                let tag = format!("sacrifice_cost_{}", pending.next_sacrifice_cost_tag_index);
                pending.next_sacrifice_cost_tag_index += 1;
                crate::tag::TagKey::from(tag)
            });
            pay_selected_cost(
                game,
                &cost,
                pending.spell_id,
                pending.caster,
                crate::costs::PaymentReason::CastSpell,
                pending.provenance,
                chosen_id,
                Some(&choice_tag),
                &mut pending.tagged_objects,
                decision_maker,
            )?;

            drain_pending_trigger_events(game, trigger_queue);

            pending.remaining_cost_steps.remove(0);
            pending.stage = CastStage::ChoosingNextCost;
            continue_spell_next_cost_or_finalize(
                game,
                trigger_queue,
                state,
                pending,
                decision_maker,
            )
        }
        CastStage::ChoosingCardCost => {
            let next_cost = pending
                .remaining_cost_steps
                .first()
                .and_then(|step| match step {
                    ActivationCostStep::CardChoice(choice) => Some(choice.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "No pending card choice cost for spell cast".to_string(),
                    )
                })?;
            let selected_delve_reduction =
                delve_generic_reduction(&ActivationCostStep::CardChoice(next_cost.clone()));

            match next_cost {
                ActivationCardCostChoice::Discard {
                    cost, card_types, ..
                } => {
                    let legal_cards = get_legal_discard_cards(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &card_types,
                    );
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell discard cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromHand {
                    cost, color_filter, ..
                } => {
                    let legal_cards = get_legal_exile_from_hand_cards(
                        game,
                        pending.caster,
                        pending.spell_id,
                        color_filter,
                    );
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell exile-from-hand cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileFromGraveyard {
                    cost, card_type, ..
                } => {
                    let legal_cards =
                        get_legal_exile_from_graveyard_cards(game, pending.caster, card_type);
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell graveyard exile cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::ExileChosenObject {
                    cost,
                    filter,
                    zone,
                    top_only,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &filter,
                        zone,
                        top_only,
                    );
                    if !legal_objects.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal spell exile cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::RevealFromHand {
                    cost,
                    card_type,
                    color_filter,
                    ..
                } => {
                    let legal_cards = get_legal_reveal_from_hand_cards(
                        game,
                        pending.caster,
                        pending.spell_id,
                        card_type,
                        color_filter,
                    );
                    if !legal_cards.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected card is not a legal spell reveal cost choice".to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;
                }
                ActivationCardCostChoice::ReturnToHand {
                    cost,
                    filter,
                    choice_tag,
                    ..
                } => {
                    let legal_targets = get_legal_return_to_hand_targets(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &filter,
                    );
                    if !legal_targets.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected permanent is not a legal spell return-to-hand cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        choice_tag.as_ref(),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
                ActivationCardCostChoice::MoveChosenObjectToZone {
                    cost,
                    filter,
                    source_zone,
                    choice_tag,
                    ..
                } => {
                    let legal_objects = get_legal_cost_choice_objects(
                        game,
                        pending.caster,
                        pending.spell_id,
                        &filter,
                        source_zone,
                        false,
                    );
                    if !legal_objects.contains(&chosen_id) {
                        return Err(GameLoopError::InvalidState(
                            "Selected object is not a legal spell move-to-zone cost choice"
                                .to_string(),
                        ));
                    }

                    pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        Some(&choice_tag),
                        &mut pending.tagged_objects,
                        decision_maker,
                    )?;

                    drain_pending_trigger_events(game, trigger_queue);
                }
            }

            if selected_delve_reduction > 0 {
                pending.mana_cost_to_pay = pending
                    .mana_cost_to_pay
                    .take()
                    .map(|cost| cost.reduce_generic(selected_delve_reduction))
                    .filter(|cost| !cost.is_empty());
            }
            pending.remaining_cost_steps.remove(0);
            if selected_delve_reduction > 0
                && pending
                    .mana_cost_to_pay
                    .as_ref()
                    .is_some_and(|cost| cost.generic_mana_total() > 0)
                && game
                    .player(pending.caster)
                    .is_some_and(|player| !player.graveyard.is_empty())
            {
                pending.remaining_cost_steps.push(delve_cost_step());
            }
            pending.stage = CastStage::ChoosingNextCost;
            continue_spell_next_cost_or_finalize(
                game,
                trigger_queue,
                state,
                pending,
                decision_maker,
            )
        }
        _ => Err(GameLoopError::InvalidState(
            "Object-choice response outside spell object-cost stages".to_string(),
        )),
    }
}

/// Apply a casting method choice response for a pending spell with multiple methods.
pub(super) fn apply_casting_method_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice_idx: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = state.pending_method_selection.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending method selection for choice response".to_string())
    })?;

    // Get the chosen method
    let chosen_option = pending
        .available_methods
        .get(choice_idx)
        .ok_or_else(|| ResponseError::IllegalChoice("Invalid casting method choice".to_string()))?;

    let casting_method = chosen_option.method.clone();

    // Now continue with the normal spell casting flow using the chosen method
    // This is essentially a copy of the CastSpell handling logic
    let player = pending.caster;
    let spell_id = pending.spell_id;
    let from_zone = pending.from_zone;

    // Move spell to stack immediately per MTG rule 601.2a
    let stack_id = propose_spell_cast(game, spell_id, from_zone, player, &casting_method)?;
    let cast_provenance =
        game.provenance_graph_mut()
            .alloc_root(ProvenanceNodeKind::EffectExecution {
                source: stack_id,
                controller: player,
            });

    let effects = game
        .object(stack_id)
        .map(|obj| obj.spell_effect_owned().unwrap_or_default())
        .unwrap_or_default();
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        &effects,
        player,
        Some(stack_id),
        None,
    );
    let optional_costs_paid = game
        .object(stack_id)
        .map(|obj| obj.optional_costs_paid.clone())
        .unwrap_or_default();
    let pending_cast = PendingCast::new(
        stack_id,
        from_zone,
        player,
        cast_provenance,
        CastStage::ChoosingModes,
        None,
        requirements,
        casting_method,
        optional_costs_paid,
        None,
        stack_id,
    );

    check_modes_or_continue(game, trigger_queue, state, pending_cast, decision_maker)
}

/// Move a spell to the stack at the start of casting (per MTG rule 601.2a).
///
/// This is called during the proposal phase, before any choices are made.
/// If casting fails later (e.g., can't pay costs), the spell should be reverted.
///
/// Returns the new ObjectId on the stack.
pub(crate) fn propose_spell_cast(
    game: &mut GameState,
    spell_id: ObjectId,
    _from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
) -> Result<ObjectId, GameLoopError> {
    let cast_during_main_phase = game.is_active_player(caster)
        && matches!(
            game.turn.phase,
            crate::game_state::Phase::FirstMain | crate::game_state::Phase::NextMain
        );
    // Capture the exact announcement-time fact before moving the proposed
    // spell to the stack. "During your main phase" is not sufficient here:
    // a nonempty stack means a sorcery still could not have been cast.
    let cast_at_sorcery_timing =
        game.is_active_player(caster) && crate::turn::is_sorcery_timing(game);
    let selected_method = game.object(spell_id).and_then(|obj| match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx),
        _ => None,
    });
    let selected_method_for_overlay = selected_method.clone();
    let cast_origin_snapshot = game.object(spell_id).map(|obj| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
    });
    let shared_usage_to_consume = match casting_method {
        CastingMethod::PlayFrom { source, zone, .. }
        | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. } => game
            .effect_store
            .grant_registry
            .shared_usage_to_consume_for_play_from(game, spell_id, *zone, caster, Some(*source)),
        _ => None,
    };

    let new_id = game
        .move_object_by_effect(spell_id, Zone::Stack)
        .ok_or_else(|| {
            GameLoopError::InvalidState("Failed to move spell to stack during proposal".to_string())
        })?;
    if let Some(shared_usage_id) = shared_usage_to_consume {
        let consumed = game
            .effect_store
            .grant_registry
            .consume_shared_usage(shared_usage_id);
        debug_assert!(
            consumed,
            "selected shared play permission should be available"
        );
    }
    if let Some(snapshot) = cast_origin_snapshot {
        game.set_cast_origin_snapshot(new_id, snapshot);
    }
    let disturb_other_def = if matches!(
        selected_method,
        Some(crate::alternative_cast::AlternativeCastingMethod::Disturb { .. })
    ) {
        let obj = game.object(new_id).ok_or_else(|| {
            GameLoopError::InvalidState(
                "Disturb spell should exist before cast overlays".to_string(),
            )
        })?;
        Some(
            game.linked_face_definition_by_name_or_id(
                obj.other_face_name.as_deref(),
                obj.other_face,
            )
            .ok_or_else(|| {
                GameLoopError::InvalidState(
                    "Disturb back face definition could not be resolved".to_string(),
                )
            })?,
        )
    } else {
        None
    };
    let split_other_def = match casting_method {
        CastingMethod::SplitOtherHalf
        | CastingMethod::SplitOtherHalfPlayFrom { .. }
        | CastingMethod::Fuse => {
            let obj = game.object(new_id).ok_or_else(|| {
                GameLoopError::InvalidState(
                    "Split spell should exist before cast overlays".to_string(),
                )
            })?;
            Some(
                game.linked_face_definition_by_name_or_id(
                    obj.other_face_name.as_deref(),
                    obj.other_face,
                )
                .ok_or_else(|| {
                    GameLoopError::InvalidState(
                        match casting_method {
                            CastingMethod::SplitOtherHalf
                            | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
                                "Split back face definition could not be resolved"
                            }
                            CastingMethod::Fuse => {
                                "Fused split back face definition could not be resolved"
                            }
                            _ => unreachable!(),
                        }
                        .to_string(),
                    )
                })?,
            )
        }
        _ => None,
    };

    let mut mark_face_down = false;
    game.set_current_controller(new_id, caster);
    if let Some(obj) = game.object_mut(new_id) {
        if let Some(method) = selected_method {
            obj.cast_alternative_method = Some(Box::new(method.clone()));
            // CR 702.140a: Mutate is an alternative cost whose spell targets a
            // non-Human creature with the same owner.  Keep this requirement in
            // the ordinary resolution program so every casting path, legality
            // preview, retargeting effect, and resolution-time target check uses
            // the same target machinery as authored spell text.
            if method.is_mutate() {
                let mutate_target =
                    crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                        crate::target::ObjectFilter::creature()
                            .owned_by(crate::target::PlayerFilter::Specific(obj.owner))
                            .without_subtype(crate::types::Subtype::Human),
                    ));
                let mut program = obj.spell_effect_owned().unwrap_or_default();
                program.insert(
                    0,
                    crate::effect::Effect::new(crate::effects::TargetOnlyEffect::new(
                        mutate_target,
                    )),
                );
                obj.spell_effect = Some(program.into());
            }
            if method.is_bestow() {
                obj.apply_bestow_cast_overlay();
            }
            if let Some(power_toughness) = method.prototype_power_toughness()
                && let Some(cost) = method.mana_cost().cloned()
            {
                obj.apply_prototype_cast_overlay(cost, power_toughness);
            }

            if let crate::alternative_cast::AlternativeCastingMethod::Disturb { .. } = method {
                let other_def = disturb_other_def
                    .as_ref()
                    .expect("disturb linked face should be resolved before mutating the spell");
                let front_colors = obj.colors();
                obj.apply_definition_face(&other_def);
                obj.cast_alternative_method = Some(Box::new(method.clone()));
                if obj.mana_cost.is_none()
                    && obj.color_override.is_none()
                    && !front_colors.is_empty()
                {
                    obj.color_override = Some(front_colors);
                }
            }

            if let crate::alternative_cast::AlternativeCastingMethod::Overload {
                ref effects, ..
            } = method
            {
                obj.spell_effect = Some(
                    crate::resolution::ResolutionProgram::from_effects(effects.clone()).into(),
                );
            }
            if let crate::alternative_cast::AlternativeCastingMethod::Cleave {
                ref effects, ..
            } = method
            {
                obj.spell_effect = Some(
                    crate::resolution::ResolutionProgram::from_effects(effects.clone()).into(),
                );
            }
            if let crate::alternative_cast::AlternativeCastingMethod::Awaken {
                ref effects, ..
            } = method
            {
                obj.spell_effect = Some(
                    crate::resolution::ResolutionProgram::from_effects(effects.clone()).into(),
                );
            }
        }

        match casting_method {
            CastingMethod::FaceDown => {
                obj.apply_face_down_cast_overlay();
                mark_face_down = true;
            }
            CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
                let other_def = split_other_def
                    .as_ref()
                    .expect("split linked face should be resolved before mutating the spell");
                obj.apply_definition_face(&other_def);
                if let CastingMethod::SplitOtherHalfPlayFrom { .. } = casting_method
                    && let Some(method) = selected_method_for_overlay.clone()
                {
                    obj.cast_alternative_method = Some(Box::new(method));
                }
            }
            CastingMethod::Fuse => {
                let other_def = split_other_def
                    .as_ref()
                    .expect("fuse linked face should be resolved before mutating the spell");
                obj.apply_fused_split_spell_overlay(&other_def);
            }
            _ => {}
        }

        obj.ensure_aura_cast_spell_effect();

        // Initialize announcement metadata while the proposal object is
        // already mutably borrowed. Keeping this before the proposal's single
        // continuous-state refresh avoids immediately dirtying the freshly
        // rebuilt state in each caller, and keeps method-selection casts in
        // sync with direct casts.
        let mut optional_costs_paid = OptionalCostsPaid::from_costs(&obj.optional_costs);
        if cast_during_main_phase {
            optional_costs_paid.mark_label_paid("CastDuringYourMainPhase");
        }
        if cast_at_sorcery_timing {
            optional_costs_paid.mark_cast_at_sorcery_timing();
        }
        obj.optional_costs_paid = optional_costs_paid;
    }

    if mark_face_down {
        game.set_face_down(new_id);
    }

    apply_play_from_cast_this_way_grants(game, new_id, caster, casting_method);

    // CR 601.2a / 610.5: one-shot effects that make the next matching spell
    // gain an ability apply while the spell is being put on the stack.  The
    // attached ability must therefore be visible to every later announcement,
    // legality, targeting, and cost query in this proposal.  The surrounding
    // cast checkpoint restores both the registry use and the card if the
    // proposal is rolled back under CR 601.6.
    game.apply_temporary_spell_ability_grants_for_cast_proposal(new_id, caster);

    // Moving the proposed spell and applying its cast overlay invalidates the
    // continuous state.  The remaining cast pipeline immediately performs
    // several independent legality, targeting, cost-modifier, and mana-source
    // queries.  Refresh once here so those views share the game-level
    // characteristic cache instead of each recalculating the same dirty board.
    game.refresh_continuous_state();

    Ok(new_id)
}

fn apply_play_from_cast_this_way_grants(
    game: &mut GameState,
    stack_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
) {
    let (source_id, zone) = match casting_method {
        CastingMethod::PlayFrom { source, zone, .. }
        | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. } => (*source, *zone),
        _ => return,
    };
    let source = game.object(source_id).or_else(|| game.object(stack_id));
    let Some(source) = source else {
        return;
    };
    let Some(mut spell_as_cast) = game.object(stack_id).cloned() else {
        return;
    };
    spell_as_cast.zone = zone;
    let selected_play_from_alternative = match casting_method {
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            ..
        } => crate::decision::resolve_play_from_alternative_method(
            game,
            caster,
            &spell_as_cast,
            zone,
            *idx,
        )
        .or_else(|| spell_as_cast.cast_alternative_method_owned()),
        _ => None,
    };
    let mut ctx = game.filter_context_for(caster, Some(source.id));
    // Moving a card from exile to the stack clears its live source-exile
    // linkage, but cast-this-way riders are selected immediately afterward.
    // Retain the proposal's origin snapshot under the same provenance tag so
    // a permission can prove that this exact spell used its source-linked
    // exile grant before adding any rider abilities.
    if zone == Zone::Exile
        && let Some(origin) = game.cast_origin_snapshot(stack_id).cloned()
    {
        ctx.tagged_objects
            .insert(crate::tag::SOURCE_EXILED_TAG.into(), vec![origin]);
    }
    let mut granted = Vec::new();
    for ability in source.abilities.iter() {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        if !static_ability.is_active(game, source.id) {
            continue;
        }
        let Some(spec) = static_ability.grant_spec() else {
            continue;
        };
        let grantable_matches_cast = match &spec.grantable {
            crate::grant::Grantable::PlayFrom => true,
            crate::grant::Grantable::AlternativeCast(method) => {
                selected_play_from_alternative.as_ref() == Some(method)
            }
            crate::grant::Grantable::DerivedAlternativeCast(derived) => {
                selected_play_from_alternative
                    .as_ref()
                    .is_some_and(|selected| {
                        derived.materialize_for(&spell_as_cast).as_ref() == Some(selected)
                    })
            }
            crate::grant::Grantable::Ability(_) => false,
        };
        if spec.zone == zone
            && grantable_matches_cast
            && !spec.cast_this_way_grants.is_empty()
            && spec.filter.matches(&spell_as_cast, &ctx, game)
            && spec
                .cast_this_way_filter
                .as_ref()
                .is_none_or(|filter| filter.matches(&spell_as_cast, &ctx, game))
        {
            granted.extend(spec.cast_this_way_grants.iter().cloned());
        }
    }
    for ability in granted {
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
            stack_id,
            ability.id(),
            Some(ability),
        );
    }
}

/// Revert a spell cast that failed during the casting process.
///
/// Per MTG rules, if casting fails at any point before completion,
/// the game state returns to before the cast was proposed.

/// Result of finalizing a spell cast, containing info needed for triggers.
pub(super) struct SpellCastResult {
    /// The new object ID of the spell on the stack
    pub(super) new_id: ObjectId,
    /// Who cast the spell
    pub(super) caster: PlayerId,
    /// Which zone the spell was cast from.
    pub(super) from_zone: Zone,
}

fn casting_method_matches_alternative_name(
    game: &GameState,
    caster: PlayerId,
    obj: &crate::object::Object,
    casting_method: &CastingMethod,
    expected_name: &str,
) -> bool {
    let method = match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx),
        _ => None,
    };
    method.is_some_and(|method| method.name().eq_ignore_ascii_case(expected_name))
}

fn alternative_cast_label(
    game: &GameState,
    caster: PlayerId,
    obj_id: ObjectId,
    casting_method: &CastingMethod,
) -> Option<String> {
    let obj = game.object(obj_id)?;
    let method = match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx)
            .or_else(|| obj.cast_alternative_method_owned()),
        _ => None,
    }?;
    let name = method.name();
    (!name.is_empty()).then(|| name.to_string())
}

fn selected_alternative_cost_reference(
    game: &GameState,
    caster: PlayerId,
    obj_id: ObjectId,
    casting_method: &CastingMethod,
) -> Option<crate::cost::OptionalCostRef> {
    let obj = game.object(obj_id)?;
    let method = match casting_method {
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx)
            .or_else(|| obj.cast_alternative_method_owned()),
        _ => None,
    }?;
    let reference = ironsmith_core::AlternativeCostReference::paid_marker(
        method.name(),
        method.mana_cost(),
    );
    Some(crate::cost::OptionalCostRef::new(
        crate::cost::OptionalCostKind::AlternativeCast(reference),
    ))
}

/// Finalize a spell cast by paying remaining costs and creating the stack entry.
/// Returns the spell cast info for trigger checking.
///
/// `stack_id` is the spell already moved to stack during proposal (per 601.2a).
pub(super) fn finalize_spell_cast(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    _state: &mut PriorityLoopState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    targets: Vec<Target>,
    target_assignments: Vec<crate::game_state::TargetAssignment>,
    target_distributions: Vec<crate::game_state::TargetDistribution>,
    x_value: Option<u32>,
    casting_method: CastingMethod,
    mut optional_costs_paid: OptionalCostsPaid,
    chosen_modes: Option<Vec<usize>>,
    spliced_cards: Vec<crate::ids::StableId>,
    mut mana_spent_to_cast: ManaPool,
    assist_mana_spent_to_cast: Option<(PlayerId, ManaPool)>,
    keyword_payment_contributions: Vec<KeywordPaymentContribution>,
    mut stack_entry_tagged_objects: std::collections::HashMap<
        crate::tag::TagKey,
        Vec<ObjectSnapshot>,
    >,
    stack_entry_effect_outcomes: std::collections::HashMap<
        crate::effect::EffectId,
        crate::effect::EffectOutcome,
    >,
    payment_trace: &mut Vec<CostStep>,
    mana_already_paid: bool,
    base_mana_cost_waived: bool,
    stack_id: ObjectId,
    provenance: ProvNodeId,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<SpellCastResult, GameLoopError> {
    use crate::decision::calculate_effective_mana_cost_with_chosen_targets_for_casting_method_from_zone;
    let _ = payment_trace;

    // All nonmana components have already been paid by the staged transaction.
    let mut base_mana_cost = game.object(spell_id).and_then(|obj| {
        crate::decision::spell_mana_cost_for_cast(game, caster, obj, &casting_method, from_zone)
    });
    if base_mana_cost_waived {
        base_mana_cost = Some(crate::mana::ManaCost::new());
    }

    let effective_cost = if let Some(ref base_cost) = base_mana_cost {
        if let Some(obj) = game.object(spell_id) {
            let eff_cost =
                calculate_effective_mana_cost_with_chosen_targets_for_casting_method_from_zone(
                    game,
                    caster,
                    obj,
                    base_cost,
                    &targets,
                    &casting_method,
                    from_zone,
                );
            Some(eff_cost)
        } else {
            base_mana_cost.clone()
        }
    } else {
        None
    };

    // Pay the mana cost unless the authoritative payment plan already committed it.
    if !mana_already_paid && let Some(cost) = effective_cost {
        let x = x_value.unwrap_or(0);
        let before_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if !game.try_pay_mana_cost_with_reason(
            caster,
            Some(spell_id),
            &cost,
            x,
            crate::costs::PaymentReason::CastSpell,
        ) {
            return Err(GameLoopError::InvalidState(
                "Cannot pay mana cost".to_string(),
            ));
        }
        let after_pool = game.player(caster).map(|player| player.mana_pool.clone());
        if let (Some(before), Some(after)) = (before_pool, after_pool) {
            mana_spent_to_cast.white += before.white.saturating_sub(after.white);
            mana_spent_to_cast.blue += before.blue.saturating_sub(after.blue);
            mana_spent_to_cast.black += before.black.saturating_sub(after.black);
            mana_spent_to_cast.red += before.red.saturating_sub(after.red);
            mana_spent_to_cast.green += before.green.saturating_sub(after.green);
            mana_spent_to_cast.colorless += before.colorless.saturating_sub(after.colorless);
        }
    }

    // Spell was already moved to stack during proposal (601.2a compliant).
    let mana_spent_total = mana_spent_to_cast.total();
    let new_id = stack_id;
    if let Some(spell_obj) = game.object_mut(new_id) {
        spell_obj.mana_spent_to_cast = mana_spent_to_cast;
        spell_obj.x_value = x_value;
    }
    let escaped = game.object(new_id).is_some_and(|spell_obj| {
        crate::decision::casting_method_matches_alternative_kind(
            game,
            caster,
            spell_obj,
            &casting_method,
            crate::filter::AlternativeCastKind::Escape,
        )
    });
    if escaped {
        optional_costs_paid.mark_label_paid("Escape");
    }
    let blitzed = game.object(new_id).is_some_and(|spell_obj| {
        crate::decision::casting_method_matches_alternative_kind(
            game,
            caster,
            spell_obj,
            &casting_method,
            crate::filter::AlternativeCastKind::Blitz,
        )
    });
    if blitzed {
        optional_costs_paid.mark_label_paid("Blitz");
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid("Blitz");
        }
    }
    let evoked = game.object(new_id).is_some_and(|spell_obj| {
        casting_method_matches_alternative_name(game, caster, spell_obj, &casting_method, "Evoke")
    });
    if evoked {
        optional_costs_paid.mark_label_paid("Evoke");
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid("Evoke");
        }
    }
    let warped = game.object(new_id).is_some_and(|spell_obj| {
        casting_method_matches_alternative_name(game, caster, spell_obj, &casting_method, "Warp")
    });
    if warped {
        game.turn_store.turn_history.spell_warped_this_turn = true;
    }
    let selected_alternative_label = alternative_cast_label(game, caster, new_id, &casting_method);
    if let Some(reference) =
        selected_alternative_cost_reference(game, caster, new_id, &casting_method)
    {
        optional_costs_paid.mark_label_paid(reference.clone());
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid(reference);
        }
    }
    if let Some(label) = selected_alternative_label.as_deref()
        && !label.eq_ignore_ascii_case("Parsed alternative cost")
        && !matches!(
            label.to_ascii_lowercase().as_str(),
            "escape" | "blitz" | "evoke"
        )
    {
        optional_costs_paid.mark_label_paid(label);
        if let Some(spell_obj) = game.object_mut(new_id) {
            spell_obj.optional_costs_paid.mark_label_paid(label);
        }
    }

    if let CastingMethod::PlayFrom { source, .. } = &casting_method {
        let source_has_selected_once_grant = game.object(*source).is_some_and(|source_obj| {
            source_obj.abilities.iter().any(|ability| {
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    return false;
                };
                static_ability.grant_spec().is_some_and(|spec| {
                    if matches!(
                        spec.usage_limit,
                        Some(
                            crate::grant::GrantUsageLimit::OnceEachTurn
                                | crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
                        )
                    ) && matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
                    {
                        return true;
                    }

                    let Some(label) = selected_alternative_label.as_deref() else {
                        return false;
                    };
                    matches!(
                        spec.grantable,
                        crate::grant::Grantable::DerivedAlternativeCast(ref derived)
                            if matches!(
                                derived.usage_limit(),
                                Some(
                                    crate::grant::GrantUsageLimit::OnceEachTurn
                                        | crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
                                )
                            ) && derived.display_name().eq_ignore_ascii_case(label)
                    )
                })
            })
        });
        if source_has_selected_once_grant {
            game.turn_store
                .grant_cast_uses_this_turn
                .insert((caster, *source));
        }
    }

    // Preserve mana-source LKI on the stack entry so the resolved permanent can
    // evaluate "for each mana from ... spent to cast it" replacement effects.
    let mana_sources_tag = crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG);
    let spent_mana_sources = game
        .object(new_id)
        .and_then(|spell_obj| spell_obj.cast_tagged_objects.get(&mana_sources_tag))
        .cloned()
        .unwrap_or_default();
    if !spent_mana_sources.is_empty() {
        stack_entry_tagged_objects.insert(mana_sources_tag, spent_mana_sources);
    }

    // Freeze the cast-time set used by values such as "for each modified
    // creature you controlled as you cast this spell".  Re-evaluating the
    // battlefield at resolution would be observably wrong after a creature
    // gains/loses a counter, an Aura, or Equipment, or changes zones.
    let cast_filter = crate::target::ObjectFilter::creature()
        .modified()
        .controlled_by(crate::target::PlayerFilter::You);
    let cast_filter_ctx = crate::filter::FilterContext::new(caster)
        .with_source(new_id)
        .with_caster(Some(caster));
    let cast_modified_creatures = game
        .object_ids_in_deterministic_order()
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| cast_filter.matches(object, &cast_filter_ctx, game))
        .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
        .collect();
    stack_entry_tagged_objects.insert(
        crate::tag::TagKey::from(ironsmith_core::CAST_MODIFIED_CREATURES_TAG),
        cast_modified_creatures,
    );

    // Preserve the complete controlled-object set for cast-time aggregates.
    // The snapshots retain calculated characteristics even if an object
    // changes characteristics or leaves the battlefield before resolution.
    let cast_controlled_filter = crate::target::ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .controlled_by(crate::target::PlayerFilter::You);
    let cast_controlled_objects = game
        .object_ids_in_deterministic_order()
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| cast_controlled_filter.matches(object, &cast_filter_ctx, game))
        .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
        .collect();
    stack_entry_tagged_objects.insert(
        crate::tag::TagKey::from(ironsmith_core::CAST_CONTROLLED_OBJECTS_TAG),
        cast_controlled_objects,
    );

    // Create stack entry with targets, X value, casting method, optional costs, and chosen modes
    let mut entry = StackEntry::new(new_id, caster)
        .with_provenance(provenance)
        .with_targets(targets.clone())
        .with_target_assignments(target_assignments)
        .with_target_distributions(target_distributions)
        .with_casting_method(casting_method)
        .with_optional_costs_paid(optional_costs_paid)
        .with_chosen_player(game.chosen_player(new_id))
        .with_chosen_modes(chosen_modes)
        .with_spliced_cards(spliced_cards)
        .with_tagged_objects(stack_entry_tagged_objects)
        .with_effect_outcomes(stack_entry_effect_outcomes)
        .with_keyword_payment_contributions(keyword_payment_contributions);
    if let Some(spell_obj) = game.object(new_id).cloned() {
        entry = entry.with_source_info(spell_obj.stable_id, spell_obj.name.to_string());
    }
    if let Some(x) = x_value {
        entry = entry.with_x(x);
    }
    game.push_to_stack(entry);

    if let Some(spell_obj) = game.object(new_id).cloned() {
        let ctx = crate::filter::FilterContext::new(caster)
            .with_source(new_id)
            .with_active_player(game.turn.active_player)
            .with_opponents(
                game.turn_store
                    .turn_order
                    .iter()
                    .copied()
                    .filter(|player_id| *player_id != caster)
                    .collect(),
            )
            .with_caster(Some(caster));
        let matching_effects = game
            .effect_store
            .temporary_spell_cost_reductions
            .iter()
            .enumerate()
            .filter_map(|(idx, effect)| {
                if effect.player != caster || effect.is_expired(game) {
                    return None;
                }
                let mut cast_filter = effect.filter.clone();
                cast_filter.targets_player = None;
                cast_filter.targets_object = None;
                cast_filter.alternative_cast = None;
                // A dynamic filter such as `{chosen name}` is relative to
                // the permanent/effect that established the reduction, not
                // to the spell currently being evaluated.
                let effect_ctx = ctx.clone().with_source(effect.source);
                cast_filter
                    .matches(&spell_obj, &effect_ctx, game)
                    .then_some(idx)
            })
            .collect::<Vec<_>>();
        for idx in matching_effects {
            if let Some(effect) = game
                .effect_store
                .temporary_spell_cost_reductions
                .get_mut(idx)
                && effect.remaining_uses > 0
                && !effect.applies_to_all_matching_this_turn
            {
                effect.remaining_uses -= 1;
            }
        }
    }
    queue_becomes_targeted_events(
        game,
        trigger_queue,
        &targets,
        new_id,
        caster,
        false,
        provenance,
    );

    if from_zone == Zone::Command {
        game.record_commander_cast_from_command_zone(new_id);
    }

    // Expend belongs to the player who actually spent each mana unit. Assist
    // can split that spending between the caster and one other player.
    let assisted_total = assist_mana_spent_to_cast
        .as_ref()
        .map(|(_, pool)| pool.total())
        .unwrap_or(0);
    record_spell_mana_spending_for_expend(
        game,
        trigger_queue,
        caster,
        new_id,
        mana_spent_total.saturating_sub(assisted_total),
        provenance,
    );
    if let Some((assistant, spent)) = assist_mana_spent_to_cast {
        record_spell_mana_spending_for_expend(
            game,
            trigger_queue,
            assistant,
            new_id,
            spent.total(),
            provenance,
        );
    }

    Ok(SpellCastResult {
        new_id,
        caster,
        from_zone,
    })
}

fn record_spell_mana_spending_for_expend(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    payer: PlayerId,
    spell: ObjectId,
    amount: u32,
    provenance: ProvNodeId,
) {
    if amount == 0 {
        return;
    }
    let previous = game
        .turn_store
        .turn_history
        .mana_spent_to_cast_spells_this_turn
        .get(&payer)
        .copied()
        .unwrap_or(0);
    let current = previous.saturating_add(amount);
    game.turn_store
        .turn_history
        .mana_spent_to_cast_spells_this_turn
        .insert(payer, current);
    for threshold in previous.saturating_add(1)..=current {
        let event_provenance =
            game.alloc_child_event_provenance(provenance, crate::events::EventKind::KeywordAction);
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(KeywordActionKind::Expend, payer, spell, threshold),
                event_provenance,
            ),
            true,
        );
    }
}

/// Run the priority loop using a DecisionMaker (convenience wrapper).
///
/// This drives the priority loop to completion using the provided decision maker.
/// Auto-passes priority when PassPriority is the only available action.
#[allow(clippy::never_loop)] // Loop structure is intentional for clarity
pub fn run_priority_loop_with<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    let mut state = PriorityLoopState::new(game.players_in_game());

    loop {
        // Use decision maker for triggered ability target selection
        let progress = advance_priority_with_dm(game, trigger_queue, decision_maker)?;

        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                // Handle context-based decisions in a loop
                let mut current_ctx = ctx;
                loop {
                    let auto_passed = should_auto_pass_ctx(&current_ctx);
                    let result = if auto_passed {
                        apply_priority_action_with_dm(
                            game,
                            trigger_queue,
                            &mut state,
                            &LegalAction::PassPriority,
                            decision_maker,
                        )
                    } else {
                        apply_decision_context_with_dm(
                            game,
                            trigger_queue,
                            &mut state,
                            &current_ctx,
                            decision_maker,
                        )
                    };

                    // Notify decision maker about auto-pass
                    if auto_passed && let Some(player) = get_priority_player_from_ctx(&current_ctx)
                    {
                        decision_maker.on_auto_pass(game, player);
                    }

                    // Handle errors with checkpoint rollback
                    let result = match result {
                        Ok(progress) => progress,
                        Err(e) => {
                            // Check if we have a checkpoint to restore
                            if let Some(checkpoint) = state.checkpoint.take() {
                                // Notify the decision maker about the rollback
                                decision_maker.on_action_cancelled(game, &format!("{}", e));
                                // Restore game state from checkpoint
                                *game = checkpoint;
                                // Clear any pending action state
                                state.pending_cast = None;
                                state.pending_activation = None;
                                state.pending_method_selection = None;
                                state.pending_mana_ability = None;
                                // Break from inner loop to restart with fresh priority
                                break;
                            } else if matches!(e, GameLoopError::ActionCancelled(_)) {
                                // The transaction already restored and cleared its
                                // checkpoint at the CR 601.6 cancellation boundary.
                                decision_maker.on_action_cancelled(game, &format!("{}", e));
                                break;
                            } else {
                                // No checkpoint - propagate the error
                                return Err(e);
                            }
                        }
                    };

                    match result {
                        GameProgress::Continue => return Ok(GameProgress::Continue),
                        GameProgress::GameOver(result) => {
                            return Ok(GameProgress::GameOver(result));
                        }
                        GameProgress::NeedsDecisionCtx(next_ctx) => {
                            current_ctx = next_ctx; // Continue the context loop
                        }
                        GameProgress::StackResolved => {
                            // Stack resolved, break from inner loop to re-run advance_priority_with_dm
                            // in the outer loop with the proper decision maker for trigger targeting
                            break;
                        }
                    }
                }
            }
            GameProgress::Continue => return Ok(GameProgress::Continue),
            GameProgress::GameOver(result) => return Ok(GameProgress::GameOver(result)),
            GameProgress::StackResolved => {
                // This shouldn't happen from advance_priority_with_dm, but handle it by continuing
                continue;
            }
        }
    }
}

/// Apply a context-based decision directly using typed decision primitives.
pub fn apply_decision_context_with_dm<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    ctx: &crate::decisions::context::DecisionContext,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    use crate::decisions::context::DecisionContext;

    if !matches!(ctx, DecisionContext::Priority(_)) {
        state.mandatory_loop.observe_player_action();
    }

    match ctx {
        DecisionContext::ManaPayment(payment_ctx) => {
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                payment_ctx.player,
                Some(payment_ctx.source),
                format!("Confirm mana payment for {}", payment_ctx.subject),
                vec![
                    crate::decisions::context::SelectableOption::new(1, "Confirm payment"),
                    crate::decisions::context::SelectableOption::new(0, "Cancel"),
                ],
                1,
                1,
            );
            let result = decision_maker.decide_options(game, &select_ctx);
            let response = if result.first().copied() == Some(1) {
                crate::mana_payment::ManaPaymentResponse::Confirm {
                    plan_id: payment_ctx.plan.id,
                    request_hash: payment_ctx.plan.request_hash,
                }
            } else {
                crate::mana_payment::ManaPaymentResponse::Cancel
            };
            apply_mana_payment_plan_response(game, trigger_queue, state, &response, decision_maker)
        }
        DecisionContext::Priority(priority_ctx) => {
            let action = decision_maker.decide_priority(game, priority_ctx);
            apply_priority_action_with_dm(game, trigger_queue, state, &action, decision_maker)
        }
        DecisionContext::Number(number_ctx) => {
            let value = decision_maker.decide_number(game, number_ctx);
            apply_x_value_response(game, trigger_queue, state, value, decision_maker)
        }
        DecisionContext::Targets(targets_ctx) => {
            let targets = decision_maker.decide_targets(game, targets_ctx);
            apply_targets_response(game, trigger_queue, state, &targets, decision_maker)
        }
        DecisionContext::Modes(modes_ctx) => {
            let options: Vec<crate::decisions::context::SelectableOption> = modes_ctx
                .spec
                .modes
                .iter()
                .map(|m| {
                    crate::decisions::context::SelectableOption::with_legality(
                        m.index,
                        m.description.clone(),
                        m.legal,
                    )
                    .with_point_cost(m.point_cost)
                    .with_repeatability(
                        modes_ctx.spec.allow_repeated_modes,
                        Some(modes_ctx.spec.max_modes.min(u32::MAX as usize) as u32),
                    )
                })
                .collect();
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                modes_ctx.player,
                modes_ctx.source,
                format!("Choose mode for {}", modes_ctx.spell_name),
                options,
                modes_ctx.spec.min_modes,
                modes_ctx.spec.max_modes,
            );
            let modes = decision_maker.decide_options(game, &select_ctx);
            apply_modes_response(game, trigger_queue, state, &modes, decision_maker)
        }
        DecisionContext::HybridChoice(hybrid_ctx) => {
            let options: Vec<crate::decisions::context::SelectableOption> = hybrid_ctx
                .options
                .iter()
                .map(|o| crate::decisions::context::SelectableOption::new(o.index, o.label.clone()))
                .collect();
            let select_ctx = crate::decisions::context::SelectOptionsContext::new(
                hybrid_ctx.player,
                hybrid_ctx.source,
                format!(
                    "Choose how to pay pip {} of {}",
                    hybrid_ctx.pip_number, hybrid_ctx.spell_name
                ),
                options,
                1,
                1,
            );
            let result = decision_maker.decide_options(game, &select_ctx);
            let choice = result.first().copied().ok_or_else(|| {
                GameLoopError::InvalidState("No hybrid payment choice selected".to_string())
            })?;
            apply_hybrid_choice_response(game, trigger_queue, state, choice, decision_maker)
        }
        DecisionContext::SelectObjects(objects_ctx) => {
            let result = decision_maker.decide_objects(game, objects_ctx);
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingSplices))
            {
                return apply_splice_response(game, trigger_queue, state, &result, decision_maker);
            }
            let chosen = result.first().copied().ok_or_else(|| {
                GameLoopError::ActionCancelled("No object selected for required choice".to_string())
            })?;

            if state.pending_activation.as_ref().is_some_and(|pending| {
                matches!(
                    pending.stage,
                    ActivationStage::ChoosingSacrifice | ActivationStage::ChoosingCardCost
                )
            }) {
                apply_sacrifice_target_response(game, trigger_queue, state, chosen, decision_maker)
            } else if state.pending_cast.as_ref().is_some_and(|pending| {
                matches!(
                    pending.stage,
                    CastStage::ChoosingSacrifice | CastStage::ChoosingCardCost
                )
            }) {
                apply_card_cost_choice_response(game, trigger_queue, state, chosen, decision_maker)
            } else {
                Err(GameLoopError::InvalidState(
                    "Unsupported SelectObjects decision in priority loop".to_string(),
                ))
            }
        }
        DecisionContext::SelectOptions(options_ctx) => {
            let result = decision_maker.decide_options(game, options_ctx);

            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| pending.stage == CastStage::ChoosingTargetChooser)
                || state
                    .pending_activation
                    .as_ref()
                    .is_some_and(|pending| pending.stage == ActivationStage::ChoosingTargetChooser)
            {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "target chooser selection requires one player".to_string(),
                    ));
                };
                return apply_target_chooser_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }

            if game.effect_store.pending_replacement_choice.is_some() {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "replacement effect choice requires one selected option".to_string(),
                    ));
                };
                return apply_replacement_choice_response(
                    game,
                    trigger_queue,
                    choice,
                    decision_maker,
                );
            }
            if state.pending_method_selection.is_some() {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "casting method choice requires one selected option".to_string(),
                    ));
                };
                return apply_casting_method_choice_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingOptionalCosts))
            {
                let choices: Vec<(usize, u32)> = result.into_iter().map(|idx| (idx, 1)).collect();
                return apply_optional_costs_response(
                    game,
                    trigger_queue,
                    state,
                    &choices,
                    decision_maker,
                );
            }
            if state.pending_cast.as_ref().is_some_and(|pending| {
                matches!(
                    pending.stage,
                    CastStage::ChoosingAssistPlayer | CastStage::ChoosingAssistContribution
                )
            }) {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "Assist setup requires one selected option".to_string(),
                    ));
                };
                return apply_assist_choice_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            if state.pending_activation.as_ref().is_some_and(|pending| {
                matches!(
                    pending.stage,
                    ActivationStage::ChoosingAlternativeCost | ActivationStage::ChoosingNextCost
                )
            }) || state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingNextCost))
            {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "next cost choice requires one selected option".to_string(),
                    ));
                };
                return apply_next_cost_choice_response(
                    game,
                    trigger_queue,
                    state,
                    choice,
                    decision_maker,
                );
            }
            Err(GameLoopError::InvalidState(
                "Unsupported SelectOptions decision in priority loop".to_string(),
            ))
        }
        DecisionContext::Distribute(distribute_ctx)
            if state.pending_cast.as_ref().is_some_and(|pending| {
                matches!(pending.stage, CastStage::ChoosingDistribution)
            }) || state.pending_activation.as_ref().is_some_and(|pending| {
                matches!(pending.stage, ActivationStage::ChoosingDistribution)
            }) =>
        {
            let distribution = decision_maker.decide_distribute(game, distribute_ctx);
            apply_target_distribution_response(
                game,
                trigger_queue,
                state,
                &distribution,
                decision_maker,
            )
        }
        DecisionContext::Distribute(_) | DecisionContext::Counters(_) => {
            if state.pending_activation.as_ref().is_some_and(|pending| {
                pending.pending_remove_counters_among.is_some()
                    || matches!(
                        pending.remaining_cost_steps.first(),
                        Some(ActivationCostStep::Cost(cost))
                            if remove_any_counters_among_effect(cost).is_some()
                    )
            }) {
                let pending = state.pending_activation.take().ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "No pending activation for staged counter-cost decision".to_string(),
                    )
                })?;
                return continue_activation_remove_counters_among_payment(
                    game,
                    trigger_queue,
                    state,
                    pending,
                    decision_maker,
                    Some(ctx),
                );
            }

            let activation_debug = state.pending_activation.as_ref().map(|pending| {
                format!(
                    "stage={}, staged_remove={}, remaining_costs={}",
                    pending.stage,
                    pending.pending_remove_counters_among.is_some(),
                    pending.remaining_cost_steps.len()
                )
            });
            Err(GameLoopError::InvalidState(format!(
                "Unsupported decision context in priority loop: {} (pending_activation={activation_debug:?}, pending_cast={}, pending_mana_ability={})",
                decision_context_name(ctx),
                state.pending_cast.is_some(),
                state.pending_mana_ability.is_some()
            )))
        }
        DecisionContext::Boolean(_)
        | DecisionContext::TextInput(_)
        | DecisionContext::Order(_)
        | DecisionContext::Attackers(_)
        | DecisionContext::Blockers(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Partition(_)
        | DecisionContext::Proliferate(_) => Err(GameLoopError::InvalidState(format!(
            "Unsupported decision context in priority loop: {}",
            decision_context_name(ctx)
        ))),
    }
}

pub(super) fn apply_priority_action_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    action: &LegalAction,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    match action {
        LegalAction::PassPriority => {
            let total_started_at = PerfTimer::start();
            let forced_pass = game.turn.priority_player.is_some_and(|player| {
                let ordinary_actions = compute_legal_actions(game, player);
                let commander_actions = compute_commander_actions(game, player);
                ordinary_actions
                    .iter()
                    .chain(commander_actions.iter())
                    .all(|action| matches!(action, LegalAction::PassPriority))
            });
            state.mandatory_loop.observe_priority_window(forced_pass);
            let pass_started_at = PerfTimer::start();
            let result = pass_priority(game, &mut state.tracker);
            let mut perf = PriorityActionPerfMetrics {
                action_kind: "pass_priority".to_string(),
                pass_priority_ms: pass_started_at.elapsed_ms(),
                ..PriorityActionPerfMetrics::default()
            };

            match result {
                PriorityResult::Continue => {
                    // Next player gets priority, advance again
                    // Use decision maker for triggered ability targeting if available
                    let advance_started_at = PerfTimer::start();
                    let result = advance_priority_with_dm(game, trigger_queue, decision_maker);
                    perf.advance_priority_ms = advance_started_at.elapsed_ms();
                    perf.priority_result = "continue".to_string();
                    perf.nested_priority_advance = crate::game_loop::last_priority_advance_perf();
                    perf.total_ms = total_started_at.elapsed_ms();
                    super::priority_apply::store_priority_action_perf(perf);
                    result
                }
                PriorityResult::StackResolves => {
                    let resolved_signature = game.stack.last().and_then(|entry| {
                        super::mandatory_loop::MandatoryProcedureObservation::from_stack_entry(
                            game, entry,
                        )
                    });
                    let queued_before_resolution = trigger_queue.entries.len();
                    // Resolve top of stack, passing decision maker for ETB replacements, choices, etc.
                    let resolve_started_at = PerfTimer::start();
                    resolve_stack_entry_with_dm_and_triggers(game, decision_maker, trigger_queue)?;
                    perf.resolve_stack_entry_ms = resolve_started_at.elapsed_ms();
                    if game.turn_store.end_turn_procedure_pending
                        || game.turn_store.end_combat_phase_procedure_pending
                    {
                        // CR 724.1/724.2: ending procedures grant no priority.
                        // Yield to TurnRunner for the ordered scheduler work.
                        perf.priority_result = "ending_procedure".to_string();
                        perf.total_ms = total_started_at.elapsed_ms();
                        super::priority_apply::store_priority_action_perf(perf);
                        return Ok(GameProgress::Continue);
                    }
                    let queued_signatures = trigger_queue
                        .entries
                        .iter()
                        .skip(queued_before_resolution)
                        .map(|entry| {
                            super::mandatory_loop::MandatoryProcedureObservation::from_trigger_entry(
                                game, entry,
                            )
                        })
                        .collect::<Vec<_>>();
                    if let Some(controllers) = state
                        .mandatory_loop
                        .observe_resolution(resolved_signature, queued_signatures)
                    {
                        game.mark_mandatory_loop_draw_for(controllers);
                        perf.priority_result = "game_over".to_string();
                        perf.total_ms = total_started_at.elapsed_ms();
                        super::priority_apply::store_priority_action_perf(perf);
                        return super::priority_core::finish_mandatory_loop_draw(
                            game,
                            decision_maker,
                        );
                    }
                    // Reset priority to active player
                    let reset_started_at = PerfTimer::start();
                    reset_priority(game, &mut state.tracker);
                    perf.reset_priority_ms = reset_started_at.elapsed_ms();
                    // Signal that stack resolved - outer loop will call advance_priority_with_dm
                    // with the proper decision maker for trigger target selection
                    perf.priority_result = "stack_resolves".to_string();
                    perf.total_ms = total_started_at.elapsed_ms();
                    super::priority_apply::store_priority_action_perf(perf);
                    Ok(GameProgress::StackResolved)
                }
                PriorityResult::PhaseEnds => {
                    perf.priority_result = "phase_ends".to_string();
                    perf.total_ms = total_started_at.elapsed_ms();
                    super::priority_apply::store_priority_action_perf(perf);
                    Ok(GameProgress::Continue)
                }
            }
        }
        _ => apply_priority_response_with_dm(
            game,
            trigger_queue,
            state,
            &PriorityResponse::PriorityAction(action.clone()),
            decision_maker,
        ),
    }
}

/// Check if we should auto-pass priority for a context-based decision.
/// Returns true if this is a Priority decision with only PassPriority available.
pub(super) fn should_auto_pass_ctx(ctx: &crate::decisions::context::DecisionContext) -> bool {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        pctx.actions.len() == 1 && matches!(pctx.actions[0], LegalAction::PassPriority)
    } else {
        false
    }
}

/// Get the player from a context-based decision, if it's a Priority decision.
pub(super) fn get_priority_player_from_ctx(
    ctx: &crate::decisions::context::DecisionContext,
) -> Option<PlayerId> {
    if let crate::decisions::context::DecisionContext::Priority(pctx) = ctx {
        Some(pctx.player)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
