use super::*;

pub(super) fn stage_after_activation_announcements(pending: &PendingActivation) -> ActivationStage {
    if !pending.remaining_requirements.is_empty() {
        ActivationStage::ChoosingTargets
    } else if !pending.remaining_cost_steps.is_empty()
        || pending.mana_cost_to_pay.is_some()
        || !pending.remaining_mana_pips.is_empty()
    {
        ActivationStage::ChoosingNextCost
    } else {
        ActivationStage::ReadyToFinalize
    }
}

#[cfg_attr(not(all(feature = "wasm", target_arch = "wasm32")), allow(dead_code))]
fn restore_priority_step_checkpoint(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    checkpoint_game: GameState,
    checkpoint_trigger_queue: TriggerQueue,
    checkpoint_state: PriorityLoopState,
) {
    *game = checkpoint_game;
    *trigger_queue = checkpoint_trigger_queue;
    *state = checkpoint_state;
}

#[cfg_attr(not(all(feature = "wasm", target_arch = "wasm32")), allow(dead_code))]
pub(crate) fn apply_priority_response_with_suspension<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    let checkpoint_game = game.clone();
    let checkpoint_trigger_queue = trigger_queue.clone();
    let checkpoint_state = state.clone();
    let result =
        apply_priority_response_with_dm(game, trigger_queue, state, response, decision_maker);

    if decision_maker.awaiting_choice() {
        restore_priority_step_checkpoint(
            game,
            trigger_queue,
            state,
            checkpoint_game,
            checkpoint_trigger_queue,
            checkpoint_state,
        );
        state.pending_continuation =
            Some(PendingPriorityContinuation::ApplyResponse(response.clone()));
        return Ok(GameProgress::Continue);
    }

    match result {
        Ok(progress) => Ok(progress),
        Err(err) => {
            restore_priority_step_checkpoint(
                game,
                trigger_queue,
                state,
                checkpoint_game,
                checkpoint_trigger_queue,
                checkpoint_state,
            );
            Err(err)
        }
    }
}

#[cfg_attr(not(all(feature = "wasm", target_arch = "wasm32")), allow(dead_code))]
pub(crate) fn resume_pending_priority_continuation_with_dm<D: DecisionMaker>(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    decision_maker: &mut D,
) -> Result<GameProgress, GameLoopError> {
    let continuation = state.pending_continuation.take().ok_or_else(|| {
        GameLoopError::InvalidState("No suspended priority continuation".to_string())
    })?;

    let checkpoint_game = game.clone();
    let checkpoint_trigger_queue = trigger_queue.clone();
    let checkpoint_state = state.clone();
    let result = match &continuation {
        PendingPriorityContinuation::ApplyResponse(response) => {
            apply_priority_response_with_dm(game, trigger_queue, state, response, decision_maker)
        }
    };

    if decision_maker.awaiting_choice() {
        restore_priority_step_checkpoint(
            game,
            trigger_queue,
            state,
            checkpoint_game,
            checkpoint_trigger_queue,
            checkpoint_state,
        );
        state.pending_continuation = Some(continuation);
        return Ok(GameProgress::Continue);
    }

    match result {
        Ok(progress) => Ok(progress),
        Err(err) => {
            restore_priority_step_checkpoint(
                game,
                trigger_queue,
                state,
                checkpoint_game,
                checkpoint_trigger_queue,
                checkpoint_state,
            );
            state.pending_continuation = Some(continuation);
            Err(err)
        }
    }
}

pub fn apply_priority_response_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if let PriorityResponse::Attackers(declarations) = response {
        if game.turn.step != Some(Step::DeclareAttackers) {
            return Err(GameLoopError::InvalidState(
                "Attackers response outside Declare Attackers step".to_string(),
            ));
        }
        let mut combat = game.combat.take().unwrap_or_default();
        let result = apply_attacker_declarations(game, &mut combat, trigger_queue, declarations);
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

    // Handle X value selection for a pending cast
    if let PriorityResponse::XValue(x) | PriorityResponse::NumberChoice(x) = response {
        return apply_x_value_response(game, trigger_queue, state, *x, &mut *decision_maker);
    }

    // Handle mode selection for a pending cast (per MTG rule 601.2b, modes before optional costs)
    if let PriorityResponse::Modes(modes) = response
        && state.pending_cast.is_some()
    {
        return apply_modes_response(game, trigger_queue, state, modes, &mut *decision_maker);
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

    // Handle mana payment selection for a pending cast, activation, or mana ability
    if let PriorityResponse::ManaPayment(choice) = response {
        // Check for pending mana ability first (most specific)
        if state.pending_mana_ability.is_some() {
            return apply_mana_payment_response_mana_ability(
                game,
                trigger_queue,
                state,
                *choice,
                decision_maker,
            );
        }
        // Check for pending activation
        if state.pending_activation.is_some() {
            return apply_mana_payment_response_activation(
                game,
                trigger_queue,
                state,
                *choice,
                &mut *decision_maker,
            );
        }
        return apply_mana_payment_response(
            game,
            trigger_queue,
            state,
            *choice,
            &mut *decision_maker,
        );
    }

    // Handle pip-by-pip mana payment for a pending activation or cast
    if let PriorityResponse::ManaPipPayment(choice) = response {
        if state.pending_activation.is_some() {
            return apply_pip_payment_response_activation(
                game,
                trigger_queue,
                state,
                *choice,
                &mut *decision_maker,
            );
        }
        if state.pending_cast.is_some() {
            return apply_pip_payment_response_cast(
                game,
                trigger_queue,
                state,
                *choice,
                &mut *decision_maker,
            );
        }
        return Err(GameLoopError::InvalidState(
            "ManaPipPayment response but no pending activation or cast".to_string(),
        ));
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

    match action {
        LegalAction::PassPriority => {
            let result = pass_priority(game, &mut state.tracker);

            match result {
                PriorityResult::Continue => {
                    // Next player gets priority, advance again
                    // Use decision maker for triggered ability targeting if available
                    advance_priority_with_dm(game, trigger_queue, decision_maker)
                }
                PriorityResult::StackResolves => {
                    // Resolve top of stack, passing decision maker for ETB replacements, choices, etc.
                    resolve_stack_entry_with_dm_and_triggers(game, decision_maker, trigger_queue)?;
                    // Reset priority to active player
                    reset_priority(game, &mut state.tracker);
                    // Signal that stack resolved - outer loop will call advance_priority_with_dm
                    // with the proper decision maker for trigger target selection
                    Ok(GameProgress::StackResolved)
                }
                PriorityResult::PhaseEnds => Ok(GameProgress::Continue),
            }
        }
        LegalAction::PlayLand { land_id } => {
            // Play the land with ETB replacement handling
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            let action = crate::special_actions::SpecialAction::PlayLand { card_id: *land_id };

            // Validate that the player can play the land
            crate::special_actions::can_perform(&action, game, player, &mut *decision_maker)
                .map_err(|e| GameLoopError::InvalidState(format!("Cannot play land: {e}")))?;

            let old_zone = game.object(*land_id).map(|o| o.zone).unwrap_or(Zone::Hand);
            let result = game
                .move_object_with_etb_processing_with_dm(
                    *land_id,
                    Zone::Battlefield,
                    decision_maker,
                )
                .ok_or_else(|| GameLoopError::InvalidState("Failed to move land".to_string()))?;
            let new_id = result.new_id;

            // Set controller
            if let Some(obj) = game.object_mut(new_id) {
                obj.controller = player;
            }

            // Check for ETB triggers only if the land entered the battlefield.
            if game
                .object(new_id)
                .map(|o| o.zone == Zone::Battlefield)
                .unwrap_or(false)
            {
                // Drain pending ZoneChangeEvent emitted by ETB move processing.
                drain_pending_trigger_events(game, trigger_queue);

                let etb_event_provenance = game
                    .provenance_graph
                    .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                let etb_event = if result.enters_tapped {
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
                let etb_event = game.ensure_trigger_event_provenance(etb_event);
                let etb_triggers = check_triggers(game, &etb_event);
                for trigger in etb_triggers {
                    trigger_queue.add(trigger);
                }

                let land_play_event_provenance = game
                    .provenance_graph
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

                if game
                    .object(new_id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Saga))
                {
                    add_lore_counter_and_check_chapters(game, new_id, trigger_queue);
                }
            }

            // Mark that the player has played a land this turn
            if let Some(player_data) = game.player_mut(player) {
                player_data.record_land_play();
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
            if matches!(casting_method, CastingMethod::Normal) {
                let available_methods =
                    collect_available_casting_methods(game, player, *spell_id, *from_zone);
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
                        .map(|o| o.name.clone())
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
                game.provenance_graph
                    .alloc_root(ProvenanceNodeKind::EffectExecution {
                        source: stack_id,
                        controller: player,
                    });

            // Get the spell's mana cost and effects, considering casting method
            // Note: We use stack_id now since the spell has been moved to stack
            let (mana_cost, effects) = if let Some(obj) = game.object(stack_id) {
                let cost = crate::decision::spell_mana_cost_for_cast(
                    game,
                    player,
                    obj,
                    casting_method,
                    *from_zone,
                );
                (cost, obj.spell_effect.clone().unwrap_or_default())
            } else {
                (None, Vec::new())
            };

            let (needs_x, max_x) = compute_spell_cast_x_bounds(
                game,
                player,
                stack_id,
                casting_method,
                mana_cost.as_ref(),
            );

            if needs_x {
                // Extract target requirements for later (use stack_id since spell is on stack)
                let requirements =
                    extract_target_requirements(game, &effects, player, Some(stack_id));

                // Initialize optional costs tracker from the spell's optional costs
                let optional_costs_paid = game
                    .object(stack_id)
                    .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
                    .unwrap_or_default();

                state.pending_cast = Some(PendingCast::new(
                    stack_id,
                    *from_zone,
                    player,
                    cast_provenance,
                    CastStage::ChoosingX,
                    None,
                    requirements,
                    casting_method.clone(),
                    optional_costs_paid,
                    None,
                    stack_id,
                ));

                let ctx = crate::decisions::context::NumberContext::x_value(
                    player, stack_id, // Use stack_id
                    max_x,
                );
                Ok(GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::Number(ctx),
                ))
            } else {
                // No X cost, check for optional costs then targets
                let requirements =
                    extract_target_requirements(game, &effects, player, Some(stack_id));

                // Initialize optional costs tracker from the spell's optional costs
                let optional_costs_paid = game
                    .object(stack_id)
                    .map(|obj| OptionalCostsPaid::from_costs(&obj.optional_costs))
                    .unwrap_or_default();

                let pending = PendingCast::new(
                    stack_id,
                    *from_zone,
                    player,
                    cast_provenance,
                    CastStage::ChoosingModes, // Will be updated by helper
                    None,
                    requirements,
                    casting_method.clone(),
                    optional_costs_paid,
                    None,
                    stack_id,
                );

                check_modes_or_continue(game, trigger_queue, state, pending, &mut *decision_maker)
            }
        }
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => {
            // Re-check activation legality at execution time so stale actions can’t
            // bypass constraints discovered after action discovery.
            if let Some(obj) = game.object(*source) {
                if let Some(ability) = obj.abilities.get(*ability_index) {
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

            // Get the ability cost, effects, tracking info, and source info for the stack entry
            let (
                base_cost,
                effects,
                is_turn_capped,
                source_stable_id,
                source_name,
                source_snapshot,
            ) = if let Some(obj) = game.object(*source) {
                let stable_id = obj.stable_id;
                let name = obj.name.clone();
                let snapshot =
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game);
                if let Some(ability) = obj.abilities.get(*ability_index) {
                    if let AbilityKind::Activated(activated) = &ability.kind {
                        let is_turn_capped = activated.max_activations_per_turn().is_some();
                        (
                            activated.mana_cost.clone(),
                            activated.effects.clone(),
                            is_turn_capped,
                            stable_id,
                            name,
                            snapshot,
                        )
                    } else {
                        (
                            crate::cost::TotalCost::free(),
                            Vec::new(),
                            false,
                            stable_id,
                            name,
                            snapshot,
                        )
                    }
                } else {
                    (
                        crate::cost::TotalCost::free(),
                        Vec::new(),
                        false,
                        stable_id,
                        name,
                        snapshot,
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
            let cost = crate::decision::calculate_effective_activation_total_cost(
                game, player, *source, &base_cost,
            );
            let activation_provenance =
                game.provenance_graph
                    .alloc_root(ProvenanceNodeKind::EffectExecution {
                        source: *source,
                        controller: player,
                    });

            // Defer non-mana activation costs until after target selection.
            let mut mana_cost_to_pay: Option<crate::mana::ManaCost> = None;
            let mut remaining_cost_steps = Vec::new();
            let payment_trace: Vec<CostStep> = Vec::new();

            append_activation_cost_steps_from_components(cost.costs(), &mut remaining_cost_steps);
            for cost_component in cost.costs() {
                match cost_component.processing_mode() {
                    crate::costs::CostProcessingMode::ManaPayment { cost } => {
                        mana_cost_to_pay = Some(cost);
                    }
                    _ => {}
                }
            }

            // Extract target requirements from the ability effects
            let target_requirements =
                extract_target_requirements(game, &effects, player, Some(*source));

            // Check if mana cost has X
            let has_x = mana_cost_to_pay
                .as_ref()
                .map(|c| c.has_x())
                .unwrap_or(false);

            // Check for hybrid/Phyrexian pips requiring announcement (per MTG rule 601.2b via 602.2b)
            let pips_to_announce = mana_cost_to_pay
                .as_ref()
                .map(get_pips_requiring_announcement)
                .unwrap_or_default();
            let has_hybrid_pips = !pips_to_announce.is_empty();

            // Create pending activation if there are choices to make
            if has_x
                || !remaining_cost_steps.is_empty()
                || has_hybrid_pips
                || !target_requirements.is_empty()
                || mana_cost_to_pay.is_some()
            {
                // Determine starting stage (per MTG rule 602.2b, follows 601.2b-h order)
                // Order: X value → Hybrid/Phyrexian announcement → Targets → non-mana costs
                // → Mana payment.
                let stage = if has_x {
                    ActivationStage::ChoosingX
                } else if has_hybrid_pips {
                    ActivationStage::AnnouncingCost
                } else if !target_requirements.is_empty() {
                    ActivationStage::ChoosingTargets
                } else if !remaining_cost_steps.is_empty() || mana_cost_to_pay.is_some() {
                    ActivationStage::ChoosingNextCost
                } else {
                    ActivationStage::ReadyToFinalize
                };

                let pending = PendingActivation::new(
                    *source,
                    *ability_index,
                    player,
                    activation_provenance,
                    stage,
                    effects.to_vec(),
                    target_requirements,
                    mana_cost_to_pay,
                    payment_trace,
                    remaining_cost_steps,
                    std::collections::HashMap::new(),
                    0,
                    is_turn_capped,
                    source_stable_id,
                    source_snapshot,
                    source_name,
                    None,
                    pips_to_announce,
                );

                continue_activation(game, trigger_queue, state, pending, &mut *decision_maker)
            } else {
                // No choices needed - put ability on stack directly
                if is_turn_capped {
                    game.record_ability_activation(*source, *ability_index);
                }

                let entry = StackEntry::ability(*source, player, effects.to_vec())
                    .with_source_info(source_stable_id, source_name)
                    .with_source_snapshot(source_snapshot)
                    .with_tagged_objects(std::collections::HashMap::new());
                game.push_to_stack(entry);
                queue_ability_activated_event(
                    game,
                    trigger_queue,
                    &mut *decision_maker,
                    *source,
                    player,
                    false,
                    Some(source_stable_id),
                );

                reset_priority(game, &mut state.tracker);
                advance_priority_with_dm(game, trigger_queue, decision_maker)
            }
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            // Mana abilities don't use the stack
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            if let Some(obj) = game.object(*source)
                && let Some(ability) = obj.abilities.get(*ability_index)
                && let AbilityKind::Activated(mana_ability) = &ability.kind
                && mana_ability.is_mana_ability()
            {
                let mana_to_add = mana_ability.mana_output.clone().unwrap_or_default();
                let effects_to_run = mana_ability.effects.clone();
                let base_cost = mana_ability.mana_cost.clone();
                let cost = crate::decision::calculate_effective_activation_total_cost(
                    game, player, *source, &base_cost,
                );

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

                // Check if we can pay the mana cost from current pool
                let can_pay_mana = if let Some(ref mc) = mana_cost {
                    game.can_pay_mana_cost(player, Some(*source), mc, 0)
                } else {
                    true // No mana cost
                };
                let mana_ability_provenance =
                    game.provenance_graph
                        .alloc_root(ProvenanceNodeKind::EffectExecution {
                            source: *source,
                            controller: player,
                        });

                if can_pay_mana {
                    // Pay all costs immediately
                    let mut cost_ctx = CostContext::new(*source, player, &mut *decision_maker)
                        .with_provenance(mana_ability_provenance);

                    // Pay mana cost first
                    if let Some(ref mc) = mana_cost
                        && !game.try_pay_mana_cost(player, Some(*source), mc, 0)
                    {
                        return Err(GameLoopError::InvalidState(
                            "Failed to pay mana cost".to_string(),
                        ));
                    }

                    // Pay other costs from TotalCost
                    for c in &other_costs {
                        crate::special_actions::pay_cost_component_with_choice(
                            game,
                            c,
                            &mut cost_ctx,
                        )
                        .map_err(|e| {
                            GameLoopError::InvalidState(format!("Failed to pay cost: {e}"))
                        })?;
                    }
                    drain_pending_trigger_events(game, trigger_queue);

                    // Add fixed mana to player's pool
                    if !mana_to_add.is_empty() {
                        if let Some(player_obj) = game.player_mut(player) {
                            for symbol in &mana_to_add {
                                player_obj.mana_pool.add(*symbol, 1);
                            }
                        }
                    }

                    // Execute additional effects (for complex mana abilities)
                    if !effects_to_run.is_empty() {
                        let mut ctx = ExecutionContext::new(*source, player, &mut *decision_maker)
                            .with_provenance(mana_ability_provenance);
                        let mut emitted_events = Vec::new();

                        for effect in &effects_to_run {
                            if let Ok(outcome) = execute_effect(game, effect, &mut ctx) {
                                emitted_events.extend(outcome.events);
                            }
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
                    );

                    // Player retains priority after activating mana ability
                    return advance_priority_with_dm(game, trigger_queue, decision_maker);
                } else {
                    // Need to tap lands / activate mana abilities to pay the mana cost
                    // Create a pending mana ability and show PayMana decision
                    let source_name = game
                        .object(*source)
                        .map(|o| o.name.clone())
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
                        undo_locked_by_mana: !mana_ability_is_undo_safe(
                            game,
                            *source,
                            *ability_index,
                        ),
                    };

                    let options = compute_mana_ability_payment_options(
                        game,
                        player,
                        &pending,
                        &mut *decision_maker,
                    );
                    state.pending_mana_ability = Some(pending);

                    // Convert ManaPaymentOption to SelectableOption
                    let selectable_options: Vec<crate::decisions::context::SelectableOption> =
                        options
                            .iter()
                            .map(|opt| {
                                crate::decisions::context::SelectableOption::new(
                                    opt.index,
                                    &opt.description,
                                )
                            })
                            .collect();

                    let ctx = crate::decisions::context::SelectOptionsContext::mana_payment(
                        player,
                        *source,
                        context,
                        selectable_options,
                    );
                    return Ok(GameProgress::NeedsDecisionCtx(
                        crate::decisions::context::DecisionContext::SelectOptions(ctx),
                    ));
                }
            }

            // Player retains priority after activating mana ability
            advance_priority_with_dm(game, trigger_queue, decision_maker)
        }
        LegalAction::TurnFaceUp { creature_id } => {
            let player = game
                .turn
                .priority_player
                .ok_or_else(|| GameLoopError::InvalidState("No priority player".to_string()))?;

            let action = crate::special_actions::SpecialAction::TurnFaceUp {
                permanent_id: *creature_id,
            };
            crate::special_actions::can_perform(&action, game, player, &mut *decision_maker)
                .map_err(|e| GameLoopError::InvalidState(format!("Cannot turn face up: {e}")))?;
            crate::special_actions::perform(action, game, player, &mut *decision_maker)
                .map_err(|e| GameLoopError::InvalidState(format!("Failed to turn face up: {e}")))?;
            drain_pending_trigger_events(game, trigger_queue);

            // Player retains priority
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
                crate::special_actions::perform(
                    special.clone(),
                    game,
                    player,
                    &mut *decision_maker,
                )
                .map_err(|e| GameLoopError::InvalidState(format!("Failed special action: {e}")))?;
                if let crate::special_actions::SpecialAction::ActivateManaAbility {
                    permanent_id,
                    ..
                } = special
                {
                    queue_ability_activated_event(
                        game,
                        trigger_queue,
                        &mut *decision_maker,
                        *permanent_id,
                        player,
                        true,
                        None,
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
pub(super) fn apply_replacement_choice_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    chosen_index: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::event_processor::{TraitEventResult, process_event_with_chosen_replacement_trait};

    // Take the pending choice
    let pending = game
        .pending_replacement_choice
        .take()
        .ok_or_else(|| GameLoopError::InvalidState("No pending replacement choice".to_string()))?;
    let pending_event_provenance = pending.event.provenance();

    // Get the chosen effect ID
    let chosen_id = pending
        .applicable_effects
        .get(chosen_index)
        .copied()
        .unwrap_or_else(|| {
            // Default to first if index is invalid
            pending.applicable_effects[0]
        });

    // Process the event with the chosen replacement effect
    let result = process_event_with_chosen_replacement_trait(game, pending.event, chosen_id);

    // Handle the result
    match result {
        TraitEventResult::Prevented => {
            // Event was prevented - nothing more to do
        }
        TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {
            // Event can proceed - the actual event application happens
            // at the point where the event was originally generated
            // (e.g., damage application, zone change, etc.)
            // The result is now stored and will be picked up by the caller
        }
        TraitEventResult::Replaced { effects, effect_id } => {
            // Event was replaced with different effects - execute them
            // Consume one-shot effects
            game.replacement_effects.mark_effect_used(effect_id);

            // Get the source/controller from the chosen replacement effect
            let (source, controller) = game
                .replacement_effects
                .get_effect(chosen_id)
                .map(|e| (e.source, e.controller))
                .unwrap_or((ObjectId::from_raw(0), PlayerId::from_index(0)));

            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, controller, &mut dm)
                .with_provenance(pending_event_provenance);

            for effect in effects {
                // Execute each replacement effect
                let _ = execute_effect(game, &effect, &mut ctx);
            }
        }
        TraitEventResult::NeedsChoice {
            player,
            applicable_effects,
            event,
        } => {
            // Build options first (before moving applicable_effects)
            let options: Vec<_> = applicable_effects
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    game.replacement_effects.get_effect(*id).map(|e| {
                        crate::decision::ReplacementOption {
                            index: i,
                            source: e.source,
                            description: game
                                .object(e.source)
                                .map(|obj| format!("Apply replacement effect from {}", obj.name))
                                .unwrap_or_else(|| "Apply replacement effect".to_string()),
                        }
                    })
                })
                .collect();

            // Still more choices needed - store and prompt again
            game.pending_replacement_choice = Some(crate::game_state::PendingReplacementChoice {
                event: *event,
                applicable_effects,
                player,
            });

            // Return to prompt for the next choice - convert to SelectOptionsContext
            let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
                .iter()
                .map(|opt| {
                    crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
                })
                .collect();
            let ctx = crate::decisions::context::SelectOptionsContext::new(
                player,
                None,
                "Choose replacement effect to apply",
                selectable_options,
                1,
                1,
            );
            return Ok(GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ));
        }
        TraitEventResult::NeedsInteraction { .. } => {
            // Interactive replacements are handled in resolve_stack_entry_full,
            // not in the replacement choice flow
            // This shouldn't happen here, but just proceed if it does
        }
    }

    // Continue with normal game flow
    advance_priority_with_dm(game, trigger_queue, decision_maker)
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
        // Combine previously chosen targets with new ones
        pending.chosen_targets.extend(targets.iter().cloned());
        pending.remaining_requirements.clear();

        pending.stage = stage_after_activation_announcements(&pending);

        return continue_activation(game, trigger_queue, state, pending, decision_maker);
    }

    let pending = state.pending_cast.take().ok_or_else(|| {
        GameLoopError::InvalidState("No pending cast for targets response".to_string())
    })?;

    // Combine previously chosen targets with new ones
    let mut all_targets = pending.chosen_targets.clone();
    all_targets.extend(targets.iter().cloned());

    // Continue to mana payment stage
    continue_to_mana_payment(
        game,
        trigger_queue,
        state,
        pending,
        all_targets,
        decision_maker,
    )
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
        // Store the X value
        pending.x_value = Some(x_value as usize);

        // Move to next stage (per MTG rule 602.2b, follows 601.2b-h order)
        // After X: Hybrid/Phyrexian announcement → Targets → non-mana costs → mana payment
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

    // Check for optional costs, then continue to targeting or finalization
    check_optional_costs_or_continue(game, trigger_queue, state, pending, decision_maker)
}
