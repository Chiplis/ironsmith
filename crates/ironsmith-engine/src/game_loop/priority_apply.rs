use super::*;
use crate::ability::ActivatedAbilityRuntimeExt as _;
use crate::perf::PerfTimer;

fn total_cost_contains_tap(cost: &crate::cost::TotalCost) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            costs.iter().any(crate::costs::Cost::requires_tap)
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            branches.iter().any(total_cost_contains_tap)
        }
    }
}

pub(super) fn stage_after_activation_announcements(pending: &PendingActivation) -> ActivationStage {
    if !pending.cost_references_ready {
        return ActivationStage::ChoosingCostReferences;
    }
    if !pending.remaining_requirements.is_empty() {
        ActivationStage::ChoosingTargets
    } else if !pending.pending_target_distributions.is_empty() {
        ActivationStage::ChoosingDistribution
    } else if !pending.remaining_cost_steps.is_empty() || pending.mana_cost_to_pay.is_some() {
        ActivationStage::ChoosingNextCost
    } else {
        ActivationStage::ReadyToFinalize
    }
}

fn build_target_assignments(
    requirements: &[TargetRequirement],
    targets: &[Target],
    offset: usize,
) -> Result<Vec<crate::game_state::TargetAssignment>, GameLoopError> {
    let requirement_contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                aggregate_constraint: requirement.aggregate_constraint.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
                shared_player_group: requirement.shared_player_group.clone(),
            },
        )
        .collect::<Vec<_>>();

    let Some(ranges) = crate::targeting::assigned_target_ranges(&requirement_contexts, targets)
    else {
        return Err(GameLoopError::ActionCancelled(
            "targets do not satisfy the stored targeting requirements".to_string(),
        ));
    };

    Ok(requirements
        .iter()
        .zip(ranges)
        .map(|(requirement, range)| crate::game_state::TargetAssignment {
            spec: requirement.spec.clone(),
            range: (offset + range.start)..(offset + range.end),
        })
        .collect())
}

#[cfg(feature = "serialization")]
use serde::Serialize;
use std::cell::RefCell;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize))]
pub struct PriorityActionPerfMetrics {
    pub action_kind: String,
    pub priority_result: String,
    pub pass_priority_ms: f64,
    pub response_apply_ms: f64,
    pub advance_priority_ms: f64,
    pub resolve_stack_entry_ms: f64,
    pub reset_priority_ms: f64,
    pub total_ms: f64,
    pub nested_priority_advance: Option<crate::game_loop::PriorityAdvancePerfMetrics>,
}

thread_local! {
    static LAST_PRIORITY_ACTION_PERF: RefCell<Option<PriorityActionPerfMetrics>> = const { RefCell::new(None) };
}

pub(super) fn store_priority_action_perf(metrics: PriorityActionPerfMetrics) {
    LAST_PRIORITY_ACTION_PERF.with(|slot| {
        *slot.borrow_mut() = Some(metrics);
    });
}

pub fn last_priority_action_perf() -> Option<PriorityActionPerfMetrics> {
    LAST_PRIORITY_ACTION_PERF.with(|slot| slot.borrow().clone())
}

pub fn apply_priority_response_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if !decision_maker.awaiting_choice() {
        game.clear_pending_decision_controllers();
    }
    // Entry and activation can suspend inside replacement programs. Their
    // priority actor and loop bookkeeping are part of the same operation.
    let checkpoint = matches!(
        response,
        PriorityResponse::PriorityAction(
            LegalAction::PlayLand { .. }
                | LegalAction::PlayLandBackFace { .. }
                | LegalAction::ActivateManaAbility { .. }
                | LegalAction::ActivateAbility { .. }
        ) | PriorityResponse::NextCostChoice(_)
            | PriorityResponse::SacrificeTarget(_)
            | PriorityResponse::CardCostChoice(_)
    )
    .then(|| (game.clone(), trigger_queue.clone(), state.clone()));
    let result =
        apply_priority_response_with_dm_inner(game, trigger_queue, state, response, decision_maker);
    if let Some((game_before, queue_before, state_before)) = checkpoint {
        if result.is_err() || decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(
                game_before,
                result.is_ok() && decision_maker.awaiting_choice(),
            );
            *trigger_queue = queue_before;
            *state = state_before;
        }
        if decision_maker.awaiting_choice() {
            return Ok(GameProgress::Continue);
        }
    }
    result
}

/// Prepare one mana activation for its actual payer. Explicit payment state
/// keeps completed child activations outside an unfinished parent's replay.
pub(super) fn begin_mana_ability_activation(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    source: &ObjectId,
    ability_index: &usize,
    player: PlayerId,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let checkpoint = (game.clone(), trigger_queue.clone(), state.clone());
    let mut saved_root_checkpoint = false;
    let result = (|| -> Result<GameProgress, GameLoopError> {
        if game.object(*source).is_some()
            && let Some(ability) = game.current_ability(*source, *ability_index)
            && let AbilityKind::Activated(mana_ability) = &ability.kind
            && mana_ability.is_runtime_mana_ability(game, *source, player)
        {
            // A root mana announcement can be canceled before paying. Its
            // checkpoint must precede exhaust and visibility registration alike.
            if state.checkpoint.is_none() {
                state.save_checkpoint(game);
                saved_root_checkpoint = true;
            }
            let mana_to_add = mana_ability.mana_output.clone().unwrap_or_default();
            let effects_to_run = mana_ability.effects.clone();
            let base_cost = mana_ability.mana_cost.clone();
            let mana_usage_restrictions = mana_ability.mana_usage_restrictions.clone();
            let mana_source_chosen_creature_type = game.chosen_creature_type(*source);
            let cost = crate::decision::calculate_effective_activation_total_cost_for_ability(
                game,
                player,
                *source,
                &base_cost,
                &[],
                Some(crate::decision::ActivationCostAbility::of(
                    game,
                    player,
                    *source,
                    mana_ability,
                )),
            );
            let activation_cost_has_tap = cost.costs().iter().any(|cost| cost.requires_tap());
            let mana_production_provenance =
                crate::special_actions::mana_production_provenance_for_activation_cost(&cost);

            // Separate mana costs from other costs
            let mut mana_cost: Option<crate::mana::ManaCost> = None;
            let mut other_costs: Vec<crate::costs::Cost> = Vec::new();

            for c in cost.costs() {
                if let Some(mc) = c.processing_mode().mana_cost() {
                    mana_cost = Some(mc.clone());
                } else {
                    other_costs.push(c.clone());
                }
            }

            // CR 602.2b / 601.2f: an {X} in a mana ability's activation
            // cost (Wizard's Rockets) is announced before the cost is
            // paid; the ability's effect then refers to that X.
            let mut announced_x: Option<u32> = None;
            if let Some(mc) = mana_cost.as_ref()
                && mc.has_x()
            {
                let policy = game.mana_spend_policy(player, Some(*source));
                let allow_black_life = crate::decision::mana_cost_has_black_symbol(mc)
                    && game.player_can_pay_black_with_life_for_reason(
                        player,
                        Some(*source),
                        crate::costs::PaymentReason::ActivateManaAbility,
                    );
                let max_x = crate::decision::compute_potential_mana(game, player)
                    .max_x_for_cost_with_mana_spend_policy_and_black_life(
                        mc,
                        &policy,
                        allow_black_life,
                    );
                let x_ctx =
                    crate::decisions::context::NumberContext::x_value(player, *source, max_x);
                let x = decision_maker.decide_number(game, &x_ctx).min(max_x);
                if decision_maker.awaiting_choice() {
                    return Ok(GameProgress::Continue);
                }
                announced_x = Some(x);
                let locked =
                    crate::decision::mana_cost_with_locked_x_and_generic_reduction(mc, x, 0);
                mana_cost = (!locked.pips().is_empty()).then_some(locked);
            }

            let mana_ability_provenance =
                game.provenance_graph_mut()
                    .alloc_root(ProvenanceNodeKind::EffectExecution {
                        source: *source,
                        controller: player,
                    });
            game.begin_library_top_announcement(
                crate::game_state::LibraryTopAnnouncement::Activation(mana_ability_provenance),
            );
            // Continuous effects have to be part of the snapshot: an
            // animated land (earthbend, Awaken, ...) is a creature only in
            // its calculated characteristics, and "whenever you tap a
            // creature for mana" matches against this snapshot.
            let source_snapshot = game
                .object(*source)
                .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));

            let view = crate::derived_view::DerivedGameView::new(game);
            if !crate::decision::exhaust_activation_allows(
                game,
                player,
                *source,
                *ability_index,
                mana_ability,
                &view,
            ) {
                return Err(GameLoopError::InvalidState(
                    "Exhaust ability was already activated".into(),
                ));
            }
            let exhaust_announcement = if mana_ability.is_exhaust_ability() {
                if state.checkpoint.is_none() {
                    state.save_checkpoint(game);
                }
                game.announce_exhaust_activation(*source, *ability_index)
            } else {
                None
            };

            if mana_cost.is_none() {
                // Pay all costs immediately
                let mut cost_ctx = CostContext::new(*source, player, &mut *decision_maker)
                    .with_reason(crate::costs::PaymentReason::ActivateManaAbility)
                    .with_provenance(mana_ability_provenance);
                cost_ctx.x_value = announced_x;
                let cost_summary =
                    crate::special_actions::pay_total_cost_without_preflight_with_choice(
                        game,
                        &cost,
                        &mut cost_ctx,
                    )
                    .map_err(super::priority_mana::activation_cost_error)?;
                let x_value_from_costs = cost_summary.x_value;
                // The effects may refer to objects the costs chose
                // ("the exiled creature's mana value", Food Chain).
                let cost_tagged_objects = cost_ctx.tagged_objects.clone();
                drop(cost_ctx);

                if decision_maker.awaiting_choice() {
                    return Ok(GameProgress::Continue);
                }
                game.finish_library_top_announcement(
                    crate::game_state::LibraryTopAnnouncement::Activation(mana_ability_provenance),
                );
                drain_pending_trigger_events(game, trigger_queue);

                let mut mana_ctx = ExecutionContext::new(*source, player, &mut *decision_maker)
                    .with_provenance(mana_ability_provenance)
                    .with_mana_usage_restrictions(mana_usage_restrictions.clone())
                    .with_mana_source_chosen_creature_type(mana_source_chosen_creature_type)
                    .with_mana_production_provenance(mana_production_provenance)
                    .with_tagged_objects(cost_tagged_objects.clone());
                if let Some(snapshot) = source_snapshot.clone() {
                    mana_ctx = mana_ctx.with_source_snapshot(snapshot);
                }
                if let Some(x) = x_value_from_costs {
                    mana_ctx = mana_ctx.with_x(x);
                }
                let outcome = crate::effects::EffectExecutor::execute(
                    &crate::effects::AddManaEffect::new(
                        mana_to_add.clone(),
                        crate::target::PlayerFilter::Specific(player),
                    ),
                    game,
                    &mut mana_ctx,
                )?;
                if mana_ctx.decision_maker.awaiting_choice() {
                    return Ok(GameProgress::Continue);
                }
                drop(mana_ctx);
                queue_triggers_for_events(game, trigger_queue, outcome.events);

                // Execute additional effects (for complex mana abilities)
                if !effects_to_run.is_empty() {
                    let mut ctx = ExecutionContext::new(*source, player, &mut *decision_maker)
                        .with_provenance(mana_ability_provenance)
                        .with_mana_usage_restrictions(mana_usage_restrictions.clone())
                        .with_mana_source_chosen_creature_type(mana_source_chosen_creature_type)
                        .with_mana_production_provenance(mana_production_provenance);
                    if let Some(snapshot) = source_snapshot.clone() {
                        ctx = ctx.with_source_snapshot(snapshot);
                    }
                    if let Some(x) = x_value_from_costs {
                        ctx = ctx.with_x(x);
                    }
                    ctx = ctx.with_tagged_objects(cost_tagged_objects);
                    let mut emitted_events = Vec::new();

                    for effect in &effects_to_run {
                        let outcome = execute_effect(game, effect, &mut ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(GameProgress::Continue);
                        }
                        emitted_events.extend(outcome.events);
                    }
                    queue_triggers_for_events(game, trigger_queue, emitted_events);
                    drain_pending_trigger_events(game, trigger_queue);
                }

                game.record_ability_activation(*source, *ability_index);

                queue_ability_activated_event(
                    game,
                    trigger_queue,
                    &mut *decision_maker,
                    *source,
                    player,
                    true,
                    None,
                    activation_cost_has_tap,
                );

                // Player retains priority after activating mana ability
                return advance_priority_with_dm(game, trigger_queue, decision_maker);
            } else {
                // Need to tap lands / activate mana abilities to pay the mana cost
                // Create a pending mana ability and show PayMana decision
                let source_name = game
                    .object(*source)
                    .map(|o| o.name.to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                let context = format!("{}'s ability", source_name);

                let pending = PendingManaAbility {
                    source: *source,
                    ability_index: *ability_index,
                    activator: player,
                    provenance: mana_ability_provenance,
                    mana_cost: mana_cost.unwrap_or_default(),
                    other_costs,
                    mana_to_add,
                    effects: effects_to_run,
                    mana_usage_restrictions,
                    mana_source_chosen_creature_type,
                    mana_production_provenance,
                    undo_locked_by_mana: !mana_ability_is_undo_safe(game, *source, *ability_index),
                    pending_mana_payment: None,
                    x_value: announced_x,
                    exhaust_announcement,
                };
                return prompt_pending_mana_ability_payment(game, state, pending, context);
            }
        }

        // Player retains priority after activating mana ability
        advance_priority_with_dm(game, trigger_queue, decision_maker)
    })();
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint.0;
        *trigger_queue = checkpoint.1;
        *state = checkpoint.2;
    } else if saved_root_checkpoint && !state.has_pending_action() {
        state.clear_checkpoint();
    }
    result
}

fn apply_priority_response_with_dm_inner(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if !matches!(
        response,
        PriorityResponse::PriorityAction(LegalAction::PassPriority)
    ) {
        state.mandatory_loop.observe_player_action();
    }

    if let PriorityResponse::Attackers(declarations) = response {
        if game.turn.step != Some(Step::DeclareAttackers) {
            return Err(GameLoopError::InvalidState(
                "Attackers response outside Declare Attackers step".to_string(),
            ));
        }
        let mut combat = game.combat.take().unwrap_or_default();
        let result = apply_attacker_declarations_with_dm(
            game,
            &mut combat,
            trigger_queue,
            declarations,
            decision_maker,
        );
        game.combat = Some(combat);
        result?;
        reset_priority(game, &mut state.tracker);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    if let PriorityResponse::Blockers {
        defending_player,
        declarations,
    } = response
    {
        if game.turn.step != Some(Step::DeclareBlockers) {
            return Err(GameLoopError::InvalidState(
                "Blockers response outside Declare Blockers step".to_string(),
            ));
        }
        let mut combat = game.combat.take().ok_or_else(|| {
            GameLoopError::InvalidState("Combat state missing at declare blockers".to_string())
        })?;
        let result = apply_blocker_declarations(
            game,
            &mut combat,
            trigger_queue,
            declarations,
            *defending_player,
        );
        game.combat = Some(combat);
        result?;
        reset_priority(game, &mut state.tracker);
        return advance_priority_with_dm(game, trigger_queue, decision_maker);
    }

    // Handle replacement effect choice
    if let PriorityResponse::ReplacementChoice(index) = response {
        return apply_replacement_choice_response(game, trigger_queue, *index, decision_maker);
    }

    // Handle target selection for a pending cast
    if let PriorityResponse::Targets(targets) = response {
        return apply_targets_response(game, trigger_queue, state, targets, &mut *decision_maker);
    }

    if let PriorityResponse::Distribution(distribution) = response {
        return apply_target_distribution_response(
            game,
            trigger_queue,
            state,
            distribution,
            &mut *decision_maker,
        );
    }

    // Handle X value selection for a pending cast
    if let PriorityResponse::XValue(x) | PriorityResponse::NumberChoice(x) = response {
        return apply_x_value_response(game, trigger_queue, state, *x, &mut *decision_maker);
    }

    // Handle mode selection for a pending cast or activated ability.
    if let PriorityResponse::Modes(modes) = response
        && (state.pending_cast.is_some() || state.pending_activation.is_some())
    {
        return apply_modes_response(game, trigger_queue, state, modes, &mut *decision_maker);
    }

    if let PriorityResponse::SpliceCards(cards) = response {
        return apply_splice_response(game, trigger_queue, state, cards, &mut *decision_maker);
    }

    // Handle optional costs selection for a pending cast
    if let PriorityResponse::OptionalCosts(choices) = response {
        return apply_optional_costs_response(
            game,
            trigger_queue,
            state,
            choices,
            &mut *decision_maker,
        );
    }

    if let PriorityResponse::ManaPaymentPlan(payment_response) = response {
        return apply_mana_payment_plan_response(
            game,
            trigger_queue,
            state,
            payment_response,
            decision_maker,
        );
    }

    if let PriorityResponse::AssistChoice(choice) = response {
        return apply_assist_choice_response(game, trigger_queue, state, *choice, decision_maker);
    }

    if let PriorityResponse::NextCostChoice(choice) = response {
        return apply_next_cost_choice_response(
            game,
            trigger_queue,
            state,
            *choice,
            &mut *decision_maker,
        );
    }

    // Handle sacrifice target selection for a pending activation
    if let PriorityResponse::SacrificeTarget(target_id) = response {
        return apply_sacrifice_target_response(
            game,
            trigger_queue,
            state,
            *target_id,
            &mut *decision_maker,
        );
    }

    // Handle card/object selection for a pending cast card-cost choice.
    if let PriorityResponse::CardCostChoice(card_id) = response {
        if state.pending_cast.is_some() {
            return apply_card_cost_choice_response(
                game,
                trigger_queue,
                state,
                *card_id,
                &mut *decision_maker,
            );
        }
        if state.pending_activation.is_some() {
            return apply_sacrifice_target_response(
                game,
                trigger_queue,
                state,
                *card_id,
                &mut *decision_maker,
            );
        }
        return Err(GameLoopError::InvalidState(
            "CardCostChoice response but no pending cast or activation".to_string(),
        ));
    }

    // Handle hybrid/Phyrexian mana choice for a pending cast (per MTG rule 601.2b)
    if let PriorityResponse::HybridChoice(choice) = response {
        return apply_hybrid_choice_response(
            game,
            trigger_queue,
            state,
            *choice,
            &mut *decision_maker,
        );
    }

    // Handle casting method selection for a pending spell with multiple methods
    if let PriorityResponse::CastingMethodChoice(choice_idx) = response {
        return apply_casting_method_choice_response(
            game,
            trigger_queue,
            state,
            *choice_idx,
            &mut *decision_maker,
        );
    }

    let PriorityResponse::PriorityAction(action) = response else {
        return Err(ResponseError::WrongResponseType.into());
    };

    if !matches!(action, LegalAction::PassPriority) {
        let actor =
            super::priority_core::priority_actor_for_action(game, action)?.ok_or_else(|| {
                GameLoopError::InvalidState(
                    "selected action is not legal for any member of the priority team".to_string(),
                )
            })?;
        game.turn.priority_player = Some(actor);
    }

    match action {
        LegalAction::PassPriority => super::priority_mana::apply_priority_action_with_dm(
            game,
            trigger_queue,
            state,
            action,
            decision_maker,
        ),
        LegalAction::KeepOpeningHand
        | LegalAction::TakeMulligan
        | LegalAction::ContinuePregame
        | LegalAction::BeginGame
        | LegalAction::UsePregameAction { .. } => Err(GameLoopError::InvalidState(
            "Pregame actions can't be used during the normal priority loop".to_string(),
        )),
        LegalAction::PlayLand { land_id } | LegalAction::PlayLandBackFace { land_id } => {
            let checkpoint = game.clone();
            let trigger_checkpoint = trigger_queue.clone();
            let instruction = (|| -> Result<(), GameLoopError> {
                // Play the land with ETB replacement handling
                let player = game
                    .turn
                    .priority_player
                    .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

                // CR 712.12: the land face chosen for a modal DFC.
                let back_face = matches!(action, LegalAction::PlayLandBackFace { .. });
                let action = if back_face {
                    crate::special_actions::SpecialAction::PlayLandBackFace { card_id: *land_id }
                } else {
                    crate::special_actions::SpecialAction::PlayLand { card_id: *land_id }
                };

                // Validate that the player can play the land
                crate::special_actions::can_perform(&action, game, player, &mut *decision_maker)
                    .map_err(|e| GameLoopError::InvalidState(format!("Cannot play land: {e}")))?;

                let old_zone = game.object(*land_id).map(|o| o.zone).unwrap_or(Zone::Hand);
                game.begin_library_top_announcement(
                    crate::game_state::LibraryTopAnnouncement::Land(*land_id),
                );
                crate::special_actions::apply_land_play_face(game, *land_id, back_face);
                let permission = crate::special_actions::choose_land_play_permission(
                    game,
                    player,
                    *land_id,
                    decision_maker,
                )?;
                if decision_maker.awaiting_choice() {
                    return Ok(());
                }
                permission.reserve(game, player)?;
                let permission_forces_tapped = permission.enters_tapped;
                let result = game
                    .move_object_with_etb_processing_with_cause_and_entry_options_and_controller(
                        *land_id,
                        Zone::Battlefield,
                        crate::events::cause::EventCause::from_special_action(
                            Some(*land_id),
                            player,
                        ),
                        decision_maker,
                        Some(player),
                        permission_forces_tapped,
                        true,
                    )?;
                if decision_maker.awaiting_choice() {
                    return Ok(());
                }
                if result.pending {
                    return Err(GameLoopError::ResolutionFailed(
                        "land entry pending without an outstanding choice".into(),
                    ));
                }
                let original_entry = match &result.original {
                    crate::events::processing::EventOutcome::Proceed(entry) => Some(entry.clone()),
                    crate::events::processing::EventOutcome::Prevented
                    | crate::events::processing::EventOutcome::Replaced => None,
                    crate::events::processing::EventOutcome::NotApplicable => {
                        return Err(GameLoopError::InvalidState("Failed to move land".into()));
                    }
                };
                // Check for ETB triggers only if the land entered the battlefield.
                if let Some(entry) = original_entry
                    && game
                        .object(entry.new_id)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                {
                    let new_id = entry.new_id;

                    let etb_event_provenance = game
                        .provenance_graph_mut()
                        .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                    let etb_event = if entry.enters_tapped {
                        TriggerEvent::new_with_provenance(
                            EnterBattlefieldEvent::tapped(new_id, old_zone),
                            etb_event_provenance,
                        )
                    } else {
                        TriggerEvent::new_with_provenance(
                            EnterBattlefieldEvent::new(new_id, old_zone),
                            etb_event_provenance,
                        )
                    };
                    let mut etb_event = game.ensure_trigger_event_provenance(etb_event);
                    game.freeze_completed_entry_events(std::iter::once(&mut etb_event))
                        .map_err(GameLoopError::ExecutionFailed)?;
                    drain_pending_trigger_events(game, trigger_queue);
                    let etb_triggers = check_triggers(game, &etb_event);
                    for trigger in etb_triggers {
                        trigger_queue.add(trigger);
                    }

                    let land_play_event_provenance = game
                        .provenance_graph_mut()
                        .alloc_root_event(crate::events::EventKind::LandPlayed);
                    let land_play_event =
                        game.ensure_trigger_event_provenance(TriggerEvent::new_with_provenance(
                            crate::events::LandPlayedEvent::new(new_id, player, old_zone),
                            land_play_event_provenance,
                        ));
                    let land_play_triggers = check_triggers(game, &land_play_event);
                    for trigger in land_play_triggers {
                        trigger_queue.add(trigger);
                    }

                    handle_saga_enters_battlefield(game, new_id, trigger_queue, decision_maker)
                        .map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?;
                    if decision_maker.awaiting_choice() {
                        return Ok(());
                    }
                }

                // Mark that the player has played a land this turn
                if let Some(player_data) = game.player_mut(player) {
                    player_data.record_land_play();
                }

                crate::special_actions::finish_land_play_receipt(
                    game,
                    *land_id,
                    player,
                    result,
                    decision_maker,
                )
                .map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?;
                if decision_maker.awaiting_choice() {
                    return Ok(());
                }
                permission.complete(game);
                drain_pending_trigger_events(game, trigger_queue);
                Ok(())
            })();
            if instruction.is_err() || decision_maker.awaiting_choice() {
                *game = checkpoint;
                *trigger_queue = trigger_checkpoint;
            }
            instruction?;
            if decision_maker.awaiting_choice() {
                return Ok(GameProgress::Continue);
            }
            // Player retains priority after playing a land
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => {
            // Save checkpoint before starting the action chain
            // This allows rollback if the player makes an invalid choice
            state.save_checkpoint(game);

            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            // Check if there are multiple available casting methods for this spell
            // and prompt for selection if the action uses the Normal method (i.e., user selected the spell generally)
            if matches!(casting_method, CastingMethod::Normal)
                && may_have_multiple_casting_methods(game, player, *spell_id, *from_zone)
            {
                let available_methods =
                    collect_available_casting_methods(game, player, *spell_id, *from_zone)?;
                if available_methods.len() > 1 {
                    // Store the pending selection and prompt user
                    state.pending_method_selection = Some(PendingMethodSelection {
                        spell_id: *spell_id,
                        from_zone: *from_zone,
                        caster: player,
                        available_methods: available_methods.clone(),
                    });

                    // Convert to SelectOptionsContext for casting method choice
                    let selectable_options: Vec<crate::decisions::context::SelectableOption> =
                        available_methods
                            .iter()
                            .enumerate()
                            .map(|(i, opt)| {
                                crate::decisions::context::SelectableOption::new(
                                    i,
                                    format!("{}: {}", opt.name, opt.cost_description),
                                )
                            })
                            .collect();
                    let spell_name = game
                        .object(*spell_id)
                        .map(|o| o.name.to_string())
                        .unwrap_or_else(|| "spell".to_string());
                    let ctx = crate::decisions::context::SelectOptionsContext::new(
                        player,
                        Some(*spell_id),
                        format!("Choose casting method for {}", spell_name),
                        selectable_options,
                        1,
                        1,
                    );
                    return Ok(GameProgress::NeedsDecisionCtx(
                        crate::decisions::context::DecisionContext::SelectOptions(ctx),
                    ));
                }
            }

            // Move spell to stack immediately per MTG rule 601.2a
            // This happens at the start of proposal, before any choices are made
            let stack_id = propose_spell_cast(game, *spell_id, *from_zone, player, casting_method)?;
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

            let optional_costs_paid = game
                .object(stack_id)
                .map(|obj| obj.optional_costs_paid.clone())
                .unwrap_or_default();

            let requirements = extract_target_requirements_from_program_with_modes(
                game,
                &effects,
                player,
                Some(stack_id),
                None,
            );
            let pending = PendingCast::new(
                stack_id,
                *from_zone,
                player,
                cast_provenance,
                CastStage::ChoosingModes,
                None,
                requirements,
                casting_method.clone(),
                optional_costs_paid,
                None,
                stack_id,
            );

            check_modes_or_continue(game, trigger_queue, state, pending, &mut *decision_maker)
        }
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => {
            // Re-check activation legality at execution time so stale actions can’t
            // bypass constraints discovered after action discovery.
            if game.object(*source).is_some() {
                if let Some(ability) = game.current_ability(*source, *ability_index) {
                    if let AbilityKind::Activated(activated) = &ability.kind {
                        if !can_activate_ability_with_restrictions(
                            game,
                            *source,
                            *ability_index,
                            activated,
                        ) {
                            return Err(GameLoopError::InvalidState(
                                "Ability activation restrictions are no longer satisfied"
                                    .to_string(),
                            ));
                        }
                    } else {
                        return Err(GameLoopError::InvalidState(
                            "Selected action is not an activated ability".to_string(),
                        ));
                    }
                } else {
                    return Err(GameLoopError::InvalidState(
                        "Ability index no longer valid".to_string(),
                    ));
                }
            } else {
                return Err(GameLoopError::InvalidState(
                    "Ability source no longer exists".to_string(),
                ));
            }

            // Save checkpoint before starting the action chain
            // This allows rollback if the player makes an invalid choice
            state.save_checkpoint(game);
            game.begin_exhaust_activation(*source, *ability_index);

            // Get the ability cost, effects, tracking info, and source info for the stack entry
            let (
                base_cost,
                effects,
                is_turn_capped,
                is_loyalty_ability,
                source_stable_id,
                source_name,
                source_snapshot,
                mana_usage_restrictions,
                mana_source_chosen_creature_type,
            ) = if let Some(obj) = game.object(*source) {
                let stable_id = obj.stable_id;
                let name = obj.name.to_string();
                let snapshot =
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game);
                let chosen_creature_type = game.chosen_creature_type(*source);
                if let Some(ability) = game.current_ability(*source, *ability_index) {
                    if let AbilityKind::Activated(activated) = &ability.kind {
                        let is_turn_capped = activated.max_activations_per_turn().is_some();
                        let is_loyalty_ability = activated.is_loyalty_ability();
                        (
                            activated.mana_cost.clone(),
                            activated.effects.clone(),
                            is_turn_capped,
                            is_loyalty_ability,
                            stable_id,
                            name,
                            snapshot,
                            activated.mana_usage_restrictions.clone(),
                            chosen_creature_type,
                        )
                    } else {
                        (
                            crate::cost::TotalCost::free(),
                            crate::resolution::ResolutionProgram::default(),
                            false,
                            false,
                            stable_id,
                            name,
                            snapshot,
                            Vec::new(),
                            chosen_creature_type,
                        )
                    }
                } else {
                    (
                        crate::cost::TotalCost::free(),
                        crate::resolution::ResolutionProgram::default(),
                        false,
                        false,
                        stable_id,
                        name,
                        snapshot,
                        Vec::new(),
                        chosen_creature_type,
                    )
                }
            } else {
                // Source doesn't exist - return error or use defaults
                return Err(GameLoopError::InvalidState(
                    "Ability source no longer exists".to_string(),
                ));
            };

            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;
            let needs_cost_references =
                crate::cost::prospective_references::needs_activation_reference_context(
                    &base_cost,
                    effects.flattened_default_effects(),
                );
            let cost = crate::decision::calculate_effective_activation_total_cost_for_ability(
                game,
                player,
                *source,
                &base_cost,
                &[],
                crate::decision::ActivationCostAbility::at(game, player, *source, *ability_index),
            );
            let activation_cost_has_tap = total_cost_contains_tap(&cost);
            let alternative_cost_branches = cost
                .as_one_of()
                .map(|branches| branches.to_vec())
                .unwrap_or_default();
            let payment_reason = crate::costs::PaymentReason::ActivateAbility;
            let activation_provenance =
                game.provenance_graph_mut()
                    .alloc_root(ProvenanceNodeKind::EffectExecution {
                        source: *source,
                        controller: player,
                    });

            game.begin_library_top_announcement(
                crate::game_state::LibraryTopAnnouncement::Activation(activation_provenance),
            );
            // Defer non-mana activation costs until after target selection.
            let mut mana_cost_to_pay: Option<crate::mana::ManaCost> = None;
            let mut remaining_cost_steps = Vec::new();
            let payment_trace: Vec<CostStep> = Vec::new();

            let flat_components = cost.as_all().unwrap_or(&[]);
            append_activation_cost_steps_from_components(
                flat_components,
                &mut remaining_cost_steps,
            );
            for cost_component in flat_components {
                if let Some(dynamic_mana) = cost_component.dynamic_mana_cost_ref() {
                    if needs_cost_references && dynamic_mana.mana_cost_of.is_some() {
                        continue;
                    }
                    let mut execution_ctx =
                        ExecutionContext::new(*source, player, &mut *decision_maker)
                            .with_provenance(activation_provenance);
                    let resolved = crate::special_actions::resolve_dynamic_mana_cost(
                        game,
                        dynamic_mana,
                        &mut execution_ctx,
                    )
                    .map_err(|err| {
                        GameLoopError::InvalidState(format!(
                            "failed to resolve dynamic activation mana cost: {err:?}"
                        ))
                    })?;
                    mana_cost_to_pay = Some(game.adjust_mana_cost_for_payment_reason(
                        player,
                        Some(*source),
                        &resolved,
                        payment_reason,
                    ));
                    continue;
                }
                if let crate::costs::CostProcessingMode::ManaPayment { cost } =
                    cost_component.processing_mode()
                {
                    mana_cost_to_pay = Some(cost);
                }
            }

            // Extract target requirements from the ability effects
            let target_requirements =
                extract_target_requirements(game, &effects, player, Some(*source));

            // Check if the activation has a modal effect or any cost references X.
            let has_modal =
                extract_modal_spec_from_program(game, &effects, player, *source).is_some();
            let activation_cost_has_x = mana_cost_to_pay
                .as_ref()
                .map(|c| c.has_x())
                .unwrap_or(false)
                || activation_cost_steps_reference_x(&remaining_cost_steps);
            let has_x = activation_cost_has_x;

            // Check for hybrid/Phyrexian pips requiring announcement (per MTG rule 601.2b via 602.2b)
            let pips_to_announce = mana_cost_to_pay
                .as_ref()
                .map(get_pips_requiring_announcement)
                .unwrap_or_default();
            let has_hybrid_pips = !pips_to_announce.is_empty();

            // Create pending activation if there are choices to make
            if needs_cost_references
                || has_x
                || has_modal
                || !alternative_cost_branches.is_empty()
                || !remaining_cost_steps.is_empty()
                || has_hybrid_pips
                || !target_requirements.is_empty()
                || mana_cost_to_pay.is_some()
            {
                // Determine starting stage (per MTG rule 602.2b, follows 601.2b-h order)
                // Order: modes → X value → Hybrid/Phyrexian announcement → Targets
                // → non-mana costs → Mana payment.
                let stage = if has_modal {
                    ActivationStage::ChoosingModes
                } else if !alternative_cost_branches.is_empty() {
                    ActivationStage::ChoosingAlternativeCost
                } else if has_x {
                    ActivationStage::ChoosingX
                } else if needs_cost_references {
                    ActivationStage::ChoosingCostReferences
                } else if has_hybrid_pips {
                    ActivationStage::AnnouncingCost
                } else if !target_requirements.is_empty() {
                    ActivationStage::ChoosingTargets
                } else if !remaining_cost_steps.is_empty() || mana_cost_to_pay.is_some() {
                    ActivationStage::ChoosingNextCost
                } else {
                    ActivationStage::ReadyToFinalize
                };

                let mut granting_source_tags = std::collections::HashMap::new();
                game.insert_granting_source_tag(*source, *ability_index, &mut granting_source_tags);
                let mut pending = PendingActivation::new(
                    *source,
                    *ability_index,
                    game.current_characteristics(*source)
                        .and_then(|chars| chars.abilities.origin(*ability_index).cloned()),
                    player,
                    activation_provenance,
                    stage,
                    effects,
                    target_requirements,
                    mana_cost_to_pay,
                    alternative_cost_branches,
                    payment_reason,
                    payment_trace,
                    remaining_cost_steps,
                    granting_source_tags,
                    0,
                    is_turn_capped,
                    is_loyalty_ability,
                    source_stable_id,
                    source_snapshot,
                    source_name,
                    None,
                    activation_cost_has_x,
                    activation_cost_has_tap,
                    mana_usage_restrictions,
                    mana_source_chosen_creature_type,
                    pips_to_announce,
                );

                if needs_cost_references {
                    pending.cost_reference_base = Some(base_cost.clone());
                    pending.cost_references_ready = false;
                    pending.tagged_objects =
                        crate::cost::prospective_references::activation_reference_context(
                            game,
                            *source,
                            *ability_index,
                        );
                    pending.cost_reference_choices =
                        crate::cost::prospective_references::activation_reference_choices(
                            &base_cost,
                            pending.effects.flattened_default_effects(),
                        )
                        .map_err(|error| {
                            GameLoopError::InvalidState(format!(
                                "cost reference announcement: {error:?}"
                            ))
                        })?;
                }
                continue_activation(game, trigger_queue, state, pending, &mut *decision_maker)
            } else {
                // No choices needed - put ability on stack directly
                let mut granting_source_tags = std::collections::HashMap::new();
                game.insert_granting_source_tag(*source, *ability_index, &mut granting_source_tags);
                game.record_ability_activation(*source, *ability_index);
                if is_loyalty_ability {
                    game.record_loyalty_ability_activation(*source);
                }

                let entry = StackEntry::ability(*source, player, effects.to_vec())
                    .with_ability_index(*ability_index)
                    .with_activation_cost_has_x(activation_cost_has_x)
                    .with_activation_cost_has_tap(activation_cost_has_tap)
                    .with_source_info(source_stable_id, source_name)
                    .with_source_snapshot(source_snapshot)
                    .with_mana_usage_restrictions(
                        mana_usage_restrictions,
                        mana_source_chosen_creature_type,
                    )
                    .with_tagged_objects(granting_source_tags);
                game.push_to_stack(entry);
                game.finish_library_top_announcement(
                    crate::game_state::LibraryTopAnnouncement::Activation(activation_provenance),
                );
                queue_ability_activated_event(
                    game,
                    trigger_queue,
                    &mut *decision_maker,
                    *source,
                    player,
                    false,
                    Some(source_stable_id),
                    activation_cost_has_tap,
                );

                priority_after_player_action(game, &mut state.tracker, player);
                advance_priority_with_dm(game, trigger_queue, decision_maker)
            }
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;
            begin_mana_ability_activation(
                game,
                trigger_queue,
                state,
                source,
                ability_index,
                player,
                decision_maker,
            )
        }
        LegalAction::TurnFaceUp {
            creature_id,
            method,
        } => {
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            let action = crate::special_actions::SpecialAction::TurnFaceUp {
                permanent_id: *creature_id,
                method: *method,
            };
            crate::special_actions::can_perform(&action, game, player, &mut *decision_maker)
                .map_err(|e| GameLoopError::InvalidState(format!("Cannot turn face up: {e}")))?;
            let performed = finish_special_action_response(
                crate::special_actions::perform(action, game, player, &mut *decision_maker),
                game,
                decision_maker,
            )?;
            drain_pending_trigger_events(game, trigger_queue);

            // CR 116.3 / 117.3c / 117.4: the player retains priority, and the
            // action breaks any run of passes in succession.
            if performed {
                priority_after_player_action(game, &mut state.tracker, player);
            }
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
        LegalAction::SpecialAction(special) => {
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            if crate::special_actions::can_perform(special, game, player, &mut *decision_maker)
                .is_ok()
            {
                let performed = finish_special_action_response(
                    crate::special_actions::perform(
                        special.clone(),
                        game,
                        player,
                        &mut *decision_maker,
                    ),
                    game,
                    decision_maker,
                )?;
                // CR 116.3 / 117.3c / 117.4: a special action breaks any run
                // of passes in succession; the player keeps priority. (Mana
                // abilities keep the existing pass count.)
                if performed
                    && !matches!(
                        special,
                        crate::special_actions::SpecialAction::ActivateManaAbility { .. }
                    )
                {
                    priority_after_player_action(game, &mut state.tracker, player);
                }
                if performed
                    && let crate::special_actions::SpecialAction::ActivateManaAbility {
                        permanent_id,
                        ability_index,
                    } = special
                {
                    let activation_cost_has_tap =
                        activated_ability_has_tap_cost(game, *permanent_id, *ability_index);
                    queue_ability_activated_event(
                        game,
                        trigger_queue,
                        &mut *decision_maker,
                        *permanent_id,
                        player,
                        true,
                        None,
                        activation_cost_has_tap,
                    );
                }
            }

            // Player retains priority after special actions
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
    }
}

/// Apply a replacement effect choice response.
///
/// When multiple replacement effects could apply to the same event,
/// the affected player must choose which one to apply first.
/// Reject a legacy choice that has no continuation for its originating
/// operation. Live replacement choices are replayed by their operation owner;
/// an event alone cannot authorize a damage/zone/draw commit or its additions.
pub(super) fn apply_replacement_choice_response(
    game: &mut GameState,
    _trigger_queue: &mut TriggerQueue,
    chosen_index: usize,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = game
        .effect_store
        .pending_replacement_choice
        .as_ref()
        .ok_or_else(|| GameLoopError::InvalidState("No pending replacement choice".into()))?;
    if pending.applicable_effects.get(chosen_index).is_none() {
        return Err(GameLoopError::InvalidState(format!(
            "replacement effect choice index {chosen_index} is invalid",
        )));
    }
    Err(GameLoopError::InvalidState(
        "legacy replacement choice has no originating operation continuation; resume replacement choices through their operation owner".into(),
    ))
}

/// Apply a Targets response for a pending spell cast.
pub(super) fn apply_targets_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    targets: &[Target],
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check for pending activation first
    if let Some(mut pending) = state.pending_activation.take() {
        let prompt_count = pending.active_target_requirement_count.max(1);
        let requirements = pending
            .remaining_requirements
            .iter()
            .take(prompt_count)
            .cloned()
            .collect::<Vec<_>>();
        let assignments =
            build_target_assignments(&requirements, targets, pending.chosen_targets.len())?;
        // Combine previously chosen targets with new ones
        pending.chosen_targets.extend(targets.iter().cloned());
        pending
            .chosen_target_assignments
            .extend(assignments.iter().cloned());
        if let Err(error) = append_target_distribution_requirements(
            game,
            pending.source,
            pending.activator,
            pending.x_value.and_then(|x| u32::try_from(x).ok()),
            &pending.chosen_targets,
            &pending.chosen_target_assignments,
            &requirements,
            &assignments,
            &mut pending.pending_target_distributions,
        ) {
            state.rollback_action(game);
            return Err(error);
        }
        pending.remaining_requirements.drain(..requirements.len());
        pending.active_target_requirement_count = 0;

        if let Some(ability) = game.current_ability(pending.source, pending.ability_index)
            && let crate::ability::AbilityKind::Activated(activated) = &ability.kind
        {
            // X was announced before targets (CR 602.2b, 601.2b); price the
            // cost with it locked so reductions apply to the X part too.
            let base_cost = match pending.x_value.and_then(|x| u32::try_from(x).ok()) {
                Some(x) => {
                    super::priority_cast::activation_cost_with_locked_x(&activated.mana_cost, x)
                }
                None => activated.mana_cost.clone(),
            };
            let base_cost = if pending.cost_reference_base.is_some() {
                match base_cost.as_one_of() {
                    Some(branches) => branches
                        .get(pending.selected_alternative_cost.ok_or_else(|| {
                            GameLoopError::InvalidState(
                                "reference price has no announced branch".into(),
                            )
                        })?)
                        .cloned()
                        .ok_or_else(|| {
                            GameLoopError::InvalidState(
                                "announced reference branch is absent".into(),
                            )
                        })?,
                    None => base_cost,
                }
            } else {
                base_cost
            };
            let base_cost = crate::cost::prospective_references::lock_activation_reference_cost(
                game,
                pending.source,
                pending.activator,
                &base_cost,
                &pending.tagged_objects,
                &pending.announced_cost_objects,
                pending.x_value.map(|x| x as u32),
            )
            .map_err(|error| {
                GameLoopError::InvalidState(format!("activation reference pricing: {error:?}"))
            })?;
            let repriced = crate::decision::calculate_effective_activation_total_cost_for_ability(
                game,
                pending.activator,
                pending.source,
                &base_cost,
                &pending.chosen_targets,
                Some(crate::decision::ActivationCostAbility::of(
                    game,
                    pending.activator,
                    pending.source,
                    activated,
                )),
            );
            let locked_cost = match repriced.kind() {
                ironsmith_core::TotalCostKind::All(_) => repriced.clone(),
                ironsmith_core::TotalCostKind::OneOf(branches) => {
                    pending.alternative_cost_branches = branches.clone();
                    let selected = pending.selected_alternative_cost.ok_or_else(|| {
                        GameLoopError::InvalidState(
                            "targets were chosen before an alternative activation cost".to_string(),
                        )
                    })?;
                    branches.get(selected).cloned().ok_or_else(|| {
                        GameLoopError::InvalidState(format!(
                            "selected activation cost branch {selected} no longer exists"
                        ))
                    })?
                }
            };
            assign_pending_activation_cost(game, &mut pending, &locked_cost, decision_maker)?;
        }

        pending.stage = if pending.remaining_requirements.is_empty() {
            stage_after_activation_announcements(&pending)
        } else {
            ActivationStage::ChoosingTargets
        };

        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for targets response".to_string())
    })?;

    let prompt_count = pending.active_target_requirement_count.max(1);
    let requirements = pending
        .remaining_requirements
        .iter()
        .take(prompt_count)
        .cloned()
        .collect::<Vec<_>>();
    let assignments =
        match build_target_assignments(&requirements, targets, pending.chosen_targets.len()) {
            Ok(assignments) => assignments,
            Err(error) => {
                state.rollback_action(game);
                return Err(error);
            }
        };

    // Combine previously chosen targets with new ones
    let mut pending = pending;
    let mut all_targets = pending.chosen_targets.clone();
    all_targets.extend(targets.iter().cloned());
    pending
        .chosen_target_assignments
        .extend(assignments.iter().cloned());
    pending.chosen_targets = all_targets.clone();
    if let Err(error) = append_target_distribution_requirements(
        game,
        pending.spell_id,
        pending.caster,
        pending.x_value,
        &pending.chosen_targets,
        &pending.chosen_target_assignments,
        &requirements,
        &assignments,
        &mut pending.pending_target_distributions,
    ) {
        state.rollback_action(game);
        return Err(error);
    }
    pending.remaining_requirements.drain(..requirements.len());
    pending.active_target_requirement_count = 0;

    if !pending.remaining_requirements.is_empty() {
        return continue_to_targets_or_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    // CR 601.2d announces divisions after targets and before total-cost locking.
    continue_cast_target_distributions_or_mana_payment(
        game,
        trigger_queue,
        state,
        pending,
        decision_maker,
    )
}

pub(super) fn apply_target_chooser_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if state
        .pending_activation
        .as_ref()
        .is_some_and(|pending| pending.stage == ActivationStage::ChoosingTargetChooser)
    {
        let mut pending = state
            .pending_activation
            .take()
            .expect("pending activation checked above");
        let chooser = pending
            .pending_target_chooser_candidates
            .get(choice)
            .copied()
            .ok_or_else(|| GameLoopError::InvalidState("Invalid target chooser".to_string()))?;
        pending.pending_target_chooser_candidates.clear();
        let requirement = pending.remaining_requirements.first_mut().ok_or_else(|| {
            GameLoopError::InvalidState("Missing delegated target requirement".to_string())
        })?;
        requirement.chooser = Some(crate::target::PlayerFilter::Specific(chooser));
        pending.stage = ActivationStage::ChoosingTargets;
        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    if state
        .pending_cast
        .as_ref()
        .is_some_and(|pending| pending.stage == CastStage::ChoosingTargetChooser)
    {
        let mut pending = state
            .pending_cast
            .take()
            .expect("pending cast checked above");
        let chooser = pending
            .pending_target_chooser_candidates
            .get(choice)
            .copied()
            .ok_or_else(|| GameLoopError::InvalidState("Invalid target chooser".to_string()))?;
        pending.pending_target_chooser_candidates.clear();
        let requirement = pending.remaining_requirements.first_mut().ok_or_else(|| {
            GameLoopError::InvalidState("Missing delegated target requirement".to_string())
        })?;
        requirement.chooser = Some(crate::target::PlayerFilter::Specific(chooser));
        pending.stage = CastStage::ChoosingTargets;
        return continue_to_targets_or_mana_payment(
            game,
            trigger_queue,
            state,
            pending,
            decision_maker,
        );
    }

    Err(GameLoopError::InvalidState(
        "No pending delegated target choice".to_string(),
    ))
}

/// Apply an X value response for a pending spell cast.
pub(super) fn apply_x_value_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    x_value: u32,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check for pending activation first
    if let Some(mut pending) = state.pending_activation.take() {
        let min_x = game
            .current_ability(pending.source, pending.ability_index)
            .and_then(|ability| match &ability.kind {
                crate::ability::AbilityKind::Activated(activated) => {
                    Some(activated.activation_x_minimum())
                }
                _ => None,
            })
            .unwrap_or(0);
        if x_value < min_x {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::InvalidState(format!(
                "X must be at least {min_x} for this activation"
            )));
        }
        // Store the X value
        pending.x_value = Some(x_value as usize);
        if let Some(obj) = game.object_mut(pending.source) {
            obj.x_value = Some(x_value);
        }

        // CR 602.2b / 601.2f: with X announced, the total cost is determined
        // with X as generic mana, so generic reductions (Training Grounds,
        // Heartstone) and the one-mana floor apply to it.
        if let Some(ability) = game.current_ability(pending.source, pending.ability_index)
            && let crate::ability::AbilityKind::Activated(activated) = &ability.kind
            && activated.mana_cost.as_all().is_none_or(|components| {
                components
                    .iter()
                    .any(|component| component.mana_cost_ref().is_some_and(|mana| mana.has_x()))
            })
        {
            let locked =
                super::priority_cast::activation_cost_with_locked_x(&activated.mana_cost, x_value);
            let repriced = crate::decision::calculate_effective_activation_total_cost_for_ability(
                game,
                pending.activator,
                pending.source,
                &locked,
                &pending.chosen_targets,
                Some(crate::decision::ActivationCostAbility::of(
                    game,
                    pending.activator,
                    pending.source,
                    activated,
                )),
            );
            let locked_cost = match repriced.kind() {
                ironsmith_core::TotalCostKind::All(_) => Some(repriced.clone()),
                ironsmith_core::TotalCostKind::OneOf(branches) => pending
                    .selected_alternative_cost
                    .and_then(|selected| branches.get(selected).cloned()),
            };
            if let Some(locked_cost) = locked_cost {
                assign_pending_activation_cost(game, &mut pending, &locked_cost, decision_maker)?;
            }
        }

        // Modes have already been announced. Continue with payment-symbol
        // announcements, targets, and the locked payment transaction.
        if !pending.pending_hybrid_pips.is_empty() {
            // Hybrid pips were populated at activation start
            pending.stage = ActivationStage::AnnouncingCost;
        } else if pending.hybrid_choices.is_empty() {
            // Check for hybrid pips now (in case X value changed the cost calculation)
            if let Some(ref mana_cost) = pending.mana_cost_to_pay {
                let pips_to_announce = get_pips_requiring_announcement(mana_cost);
                if !pips_to_announce.is_empty() {
                    pending.pending_hybrid_pips = pips_to_announce;
                    pending.stage = ActivationStage::AnnouncingCost;
                    return continue_activation(
                        game,
                        trigger_queue,
                        state,
                        pending,
                        decision_maker,
                    );
                }
            }
            pending.stage = stage_after_activation_announcements(&pending);
        } else {
            pending.stage = stage_after_activation_announcements(&pending);
        }

        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    // Otherwise handle pending cast
    let mut pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState(
            "No pending cast or activation for X value response".to_string(),
        )
    })?;

    // Store the X value
    pending.x_value = Some(x_value);
    if let Some(obj) = game.object_mut(pending.spell_id) {
        obj.x_value = Some(x_value);
    }

    // X can change a target's filter as well as a self-replacement's target
    // requirements. Re-read the complete program after announcing X and before
    // announcing targets (CR 601.2b-d).
    if let Some(requirements) = game
        .object(pending.spell_id)
        .and_then(|spell| spell.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        })
    {
        pending.remaining_requirements = requirements;
    }

    // Modes and alternative/additional costs were announced before X.
    continue_to_targeting_or_finalize(game, trigger_queue, state, pending, decision_maker)
}

// Cancel is an ordinary completed interaction after the special-action
// transaction restores its checkpoint, not a failed replay command.
fn finish_special_action_response(
    result: Result<(), crate::special_actions::ActionError>,
    game: &GameState,
    decision_maker: &mut impl DecisionMaker,
) -> Result<bool, GameLoopError> {
    match result {
        Ok(()) => Ok(true),
        Err(crate::special_actions::ActionError::Cancelled) => {
            decision_maker.on_action_cancelled(game, "Special action cancelled");
            Ok(false)
        }
        Err(error) => Err(GameLoopError::InvalidState(format!(
            "Failed special action: {error}"
        ))),
    }
}

/// Record a target-dependent type choice before constructing the target menu.
pub(super) fn apply_creature_type_announcement_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    choice: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let pending = state
        .pending_cast
        .as_ref()
        .filter(|pending| pending.stage == CastStage::ChoosingCreatureType)
        .ok_or_else(|| {
            GameLoopError::InvalidState("No pending creature-type announcement".into())
        })?;
    let subtype = crate::types::SubtypeFamily::Creature
        .all_subtypes()
        .get(choice)
        .copied()
        .filter(|subtype| {
            pending_spell_creature_type_options(game, pending)
                .is_some_and(|options| options.contains(subtype))
        })
        .ok_or_else(|| GameLoopError::InvalidState("Invalid creature-type announcement".into()))?;
    let mut pending = state.pending_cast.take().expect("validated pending cast");
    game.set_chosen_subtype(pending.spell_id, subtype);
    pending.remaining_requirements = game
        .object(pending.spell_id)
        .and_then(|spell| spell.spell_effect.as_ref())
        .map(|program| {
            extract_target_requirements_from_program_with_modes(
                game,
                program,
                pending.caster,
                Some(pending.spell_id),
                pending.chosen_modes.as_deref(),
            )
        })
        .unwrap_or_default();
    pending.stage = CastStage::ChoosingTargets;
    continue_to_targets_or_mana_payment(game, trigger_queue, state, pending, decision_maker)
}

#[cfg(test)]
mod replacement_ownerless_choice_contract_tests {
    use super::*;
    fn fixture(
        payload: bool,
    ) -> (
        GameState,
        TriggerQueue,
        PlayerId,
        crate::replacement::ReplacementEffectId,
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &crate::card::CardBuilder::new(
                crate::ids::CardId::new(),
                "Legacy choice diagnostic source",
            )
            .card_types(vec![crate::types::CardType::Enchantment])
            .build(),
            alice,
            crate::zone::Zone::Battlefield,
        );
        let action = if payload {
            crate::replacement::ReplacementAction::Instead(vec![
                crate::effect::Effect::gain_life(2),
                crate::effect::Effect::lose_life(crate::effect::Value::X),
            ])
        } else {
            crate::replacement::ReplacementAction::Double
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::cards::matchers::WouldDrawCardMatcher::you(),
                action,
            ),
        );
        game.effect_store.pending_replacement_choice =
            Some(crate::game_state::PendingReplacementChoice {
                event: crate::events::Event::draw(alice, 1, true),
                applicable_effects: vec![shield],
                applied_effects: Default::default(),
                applied_effect_keys: Default::default(),
                player: alice,
            });
        game.take_pending_trigger_events();
        (game, TriggerQueue::new(), alice, shield)
    }
    fn check_response(index: usize, payload: bool) {
        let (mut game, mut queue, alice, shield) = fixture(payload);
        let ids = game.next_object_id_counter();
        let result = apply_replacement_choice_response(
            &mut game,
            &mut queue,
            index,
            &mut crate::decision::SelectFirstDecisionMaker,
        );
        assert!(
            matches!(result, Err(GameLoopError::InvalidState(_))),
            "an ownerless choice must not report successful event application"
        );
        assert!(
            game.effect_store.pending_replacement_choice.is_some(),
            "preserve invalid continuation for diagnosis and recovery"
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), ids);
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
    }
    #[test]
    fn resolved_ownerless_event_is_rejected_without_consumption() {
        check_response(0, false);
    }
    #[test]
    fn ownerless_payload_cannot_execute_or_swallow_errors() {
        check_response(0, true);
    }
    #[test]
    fn invalid_option_retains_pending_event() {
        check_response(1, false);
    }
    #[test]
    fn priority_cannot_offer_an_unresumable_choice() {
        let (mut game, mut queue, alice, shield) = fixture(false);
        let result = advance_priority_with_dm(
            &mut game,
            &mut queue,
            &mut crate::decision::SelectFirstDecisionMaker,
        );
        assert!(
            matches!(result, Err(GameLoopError::InvalidState(_))),
            "a pending event without an operation owner must fail before offering a choice"
        );
        assert!(game.effect_store.pending_replacement_choice.is_some());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(queue.entries.is_empty());
    }
}
