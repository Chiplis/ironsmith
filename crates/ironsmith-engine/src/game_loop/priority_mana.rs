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
    effect_outcomes: &mut std::collections::HashMap<
        crate::effect::EffectId,
        crate::effect::EffectOutcome,
    >,
    decision_maker: &mut impl DecisionMaker,
) -> Result<Option<Vec<ObjectSnapshot>>, GameLoopError> {
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
    cost_ctx.effect_outcomes = effect_outcomes.clone();
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
            if cost_ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
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
            if preserve_chosen_snapshot
                && let Some(tag) = effective_choice_tag.as_ref()
                && let Some(snapshot) = chosen_snapshot
            {
                let tagged = cost_ctx.tagged_objects.entry(tag.clone()).or_default();
                tagged.retain(|existing| existing.stable_id != snapshot.stable_id);
                tagged.push(snapshot);
            }
            let completed_sacrifice = cost_ctx.completed_sacrifice.take();
            if preserve_chosen_snapshot
                && let Some(binding) = effective_choice_tag
                    .as_ref()
                    .and_then(ironsmith_core::tag::SacrificeCostTag::parse)
            {
                let actual = completed_sacrifice.as_ref().ok_or_else(|| {
                    GameLoopError::InvalidState(
                        "paid sacrifice is missing its original-action result".into(),
                    )
                })?;
                cost_ctx
                    .tagged_objects
                    .insert(binding.original_result_key(), actual.clone());
            }
            *tagged_objects = cost_ctx.tagged_objects;
            *effect_outcomes = cost_ctx.effect_outcomes;
            Ok(completed_sacrifice)
        }
        Ok(crate::costs::CostPaymentResult::NeedsChoice(_)) => Err(GameLoopError::InvalidState(
            "Cost still needed a choice after preselection".to_string(),
        )),
        Err(err) => Err(activation_cost_error(err)),
    }
}

#[cfg(test)]
mod emerge_receipt_payment_boundary_tests {
    use super::*;

    #[test]
    fn invalid_sacrifice_characteristics_cannot_publish_a_cast_receipt_or_mutation() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let material = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Emerge material")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, i32::MAX)).build();
        let material = game.create_object_from_card(&material, player, Zone::Battlefield);
        let spell = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Emerge spell")
            .card_types(vec![crate::types::CardType::Creature]).build();
        let spell = game.create_object_from_card(&spell, player, Zone::Stack);
        // Corrupt imported/derived state after announcement. The legacy
        // receipt snapshot may see it, but checked payment must reject the
        // out-of-range final toughness before sacrificing or publishing tags.
        game.object_mut(material).unwrap().counters.insert(crate::object::CounterType::PlusOnePlusOne, 1);
        let cost = crate::costs::Cost::sacrifice(ObjectFilter::creature().you_control());
        let tag = crate::tag::TagKey::from("sacrifice_cost_0");
        let mut tagged = std::collections::HashMap::new();
        let mut outcomes = std::collections::HashMap::new();
        let before = game.battlefield.clone();
        let result = pay_selected_cost(&mut game, &cost, spell, player,
            crate::costs::PaymentReason::CastSpell, Default::default(), material,
            Some(&tag), &mut tagged, &mut outcomes, &mut crate::decision::SelectFirstDecisionMaker);
        assert!(result.is_err(), "required material evidence must fail closed");
        assert_eq!(game.battlefield, before);
        assert_eq!(game.object(material).unwrap().zone, Zone::Battlefield);
        assert!(tagged.is_empty() && outcomes.is_empty());
        assert!(game.object(spell).unwrap().cast_tagged_objects.is_empty());
        assert!(game.battlefield.iter().all(|id| game.object(*id).unwrap().kind != crate::object::ObjectKind::Token));
    }

    #[test]
    fn suspended_sacrifice_addition_replays_once_without_becoming_missing_receipt_failure() {
        struct Pause { pending: bool, pause: bool }
        impl crate::decision::DecisionMaker for Pause {
            fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Material")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 5)).build();
        let material = game.create_object_from_card(&card, player, Zone::Battlefield);
        let source = game.create_object_from_card(&card, player, Zone::Stack);
        game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            source, player,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(material), Some(Zone::Battlefield), Some(Zone::Graveyard)),
            crate::replacement::ReplacementAction::Additionally(vec![crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(3)])]),
        ));
        game.take_pending_trigger_events();
        let cost = crate::costs::Cost::sacrifice(ObjectFilter::creature().you_control());
        let tag = crate::tag::TagKey::from("sacrifice_cost_0");
        let mut tags = std::collections::HashMap::new(); let mut outcomes = std::collections::HashMap::new();
        let mut dm = Pause { pending: false, pause: true };
        let pending = pay_selected_cost(&mut game, &cost, source, player,
            crate::costs::PaymentReason::CastSpell, Default::default(), material,
            Some(&tag), &mut tags, &mut outcomes, &mut dm).unwrap();
        assert!(pending.is_none() && dm.pending);
        assert!(tags.is_empty() && outcomes.is_empty());
        assert_eq!(game.object(material).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.token_resource_failure().is_none());
        dm.pending = false; dm.pause = false;
        let completed = pay_selected_cost(&mut game, &cost, source, player,
            crate::costs::PaymentReason::CastSpell, Default::default(), material,
            Some(&tag), &mut tags, &mut outcomes, &mut dm).unwrap().unwrap();
        assert_eq!(completed.len(), 1);
        let actual_tag = ironsmith_core::tag::SacrificeCostTag::OriginalResult(0).key();
        assert_eq!(tags[&actual_tag].len(), 1);
        assert_eq!(tags[&actual_tag][0].object_id, material);
        assert_eq!(tags[&actual_tag][0].zone, Zone::Battlefield);
        assert_eq!(completed[0].object_id, material);
        assert_eq!(completed[0].toughness, Some(5));
        assert_eq!(game.player(player).unwrap().life, 23);
        assert!(game.object(material).is_none_or(|object| object.zone != Zone::Battlefield));
        assert_eq!(game.take_pending_trigger_events().iter().filter(|event|
            event.downcast::<crate::events::permanents::SacrificeEvent>().is_some()).count(), 1);
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

        if pip.len() == 1
            && let ManaSymbol::Generic(n) = pip[0]
        {
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
    execute_planned_mana_activations_with_outputs(
        game,
        trigger_queue,
        payer,
        payment,
        undo_locked_by_mana,
        decision_maker,
    )
    .map(|(pending, _)| pending)
}

fn execute_planned_mana_activations_with_outputs(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    payer: PlayerId,
    payment: &mut crate::mana_payment::PendingManaPayment,
    undo_locked_by_mana: &mut bool,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(bool, Vec<crate::effects::CompletedEffectOutputs>), GameLoopError> {
    let mut outputs = Vec::new();
    while let Some(step) = payment
        .plan
        .mana_ability_steps
        .get(payment.next_activation)
        .cloned()
    {
        let mut replay = crate::mana_payment::WitnessDecisionMaker::for_activation(
            step.replacement_witnesses.as_deref(),
            step.production_witnesses.as_deref(),
            decision_maker,
        );
        let request = payment
            .request
            .reserving_alternatives(&payment.plan.allocations);
        let mut exclusions = request.activation_excluded_sources.clone();
        exclusions.push(step.source);
        let completed = crate::special_actions::perform_mana_ability_with_payment_outputs(
            game,
            payer,
            step.source,
            step.ability_index,
            step.color_restriction.clone(),
            Some(exclusions),
            request.reserved_tap_sources,
            &mut replay,
        )
        .map_err(|error| match error {
            crate::special_actions::ActionError::ExecutionFailure { error, .. } => {
                GameLoopError::ExecutionFailed(error)
            }
            error => GameLoopError::InvalidState(format!(
                "planned mana ability is no longer legal: {error}"
            )),
        })?;
        if replay.awaiting_choice() {
            // Replay-based decision makers will rerun this same activation
            // from the enclosing action checkpoint with the captured answer.
            // Do not advance the plan cursor until that replay completes.
            return Ok((true, Vec::new()));
        }
        let activation = completed.activation_notification.ok_or_else(|| {
            GameLoopError::ExecutionFailed(crate::effects::ExecutionError::IncompleteEvidence(
                "completed planned mana activation has no prepared notification".into(),
            ))
        })?;
        outputs.extend(completed.outputs);
        queue_triggers_for_events(game, trigger_queue, completed.events)?;
        outputs.extend(queue_prepared_activation_notification_with_outputs(
            game,
            trigger_queue,
            &mut replay,
            activation,
        )?);
        if replay.awaiting_choice() {
            return Ok((true, Vec::new()));
        }
        if !replay.complete() {
            return Err(GameLoopError::InvalidState(
                "unused planned mana decision witness".into(),
            ));
        }
        *undo_locked_by_mana |= !step.undo_safe;
        payment.next_activation += 1;
        try_drain_pending_trigger_events(game, trigger_queue)?;
    }
    Ok((false, outputs))
}

/// Waterbend has the same payment owner for spells, activations and effects.
/// Emit completion only after mana/life payment succeeds below.
fn execute_planned_waterbend_taps(
    game: &mut GameState,
    payment: &crate::mana_payment::PendingManaPayment,
) -> Result<(), GameLoopError> {
    if !crate::mana_payment::validate_waterbend_taps(
        game,
        &payment.request,
        &payment.plan.allocations,
    ) {
        if let Some(error) = game.token_resource_failure() {
            return Err(GameLoopError::ExecutionFailed(error));
        }
        return Err(GameLoopError::InvalidState(
            "planned Waterbend resources are no longer eligible".into(),
        ));
    }
    let mut taps =
        crate::effects::permanents::TapAction::new(game, payment.request.payer, Default::default());
    for allocation in &payment.plan.allocations {
        if let crate::mana_payment::PlannedPipPayment::Waterbend(id) = allocation.payment {
            if !taps.tap(game, id) {
                return Err(GameLoopError::InvalidState(
                    "planned Waterbend permanent could not be tapped".into(),
                ));
            }
        }
    }
    for event in taps.finish(game).events {
        game.queue_trigger_event(event.provenance(), event);
    }
    Ok(())
}

fn execute_planned_keyword_payments(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &mut PendingCast,
    payment: &crate::mana_payment::PendingManaPayment,
    _decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    if !crate::mana_payment::validate_waterbend_taps(
        game,
        &payment.request,
        &payment.plan.allocations,
    ) {
        if let Some(error) = game.token_resource_failure() {
            return Err(GameLoopError::ExecutionFailed(error));
        }
        return Err(GameLoopError::InvalidState(
            "planned Waterbend resources are no longer eligible".into(),
        ));
    }
    // CR 702.66a / 603.2c: the cards exiled with delve leave the graveyard
    // together, as one event ("whenever one or more cards leave your
    // graveyard").
    let mut taps =
        crate::effects::permanents::TapAction::new(game, pending.caster, pending.provenance);
    let opened_batch = game.open_simultaneous_action();
    let result = (|| -> Result<(), GameLoopError> {
        for allocation in &payment.plan.allocations {
            if let crate::mana_payment::PlannedPipPayment::Waterbend(id) = allocation.payment {
                if !taps.tap(game, id) {
                    return Err(GameLoopError::InvalidState(
                        "planned Waterbend permanent could not be tapped".into(),
                    ));
                }
                continue;
            }
            if let crate::mana_payment::PlannedPipPayment::Delve(card_id) = allocation.payment {
                if !game
                    .player(pending.caster)
                    .is_some_and(|player| player.graveyard.contains(&card_id))
                {
                    return Err(GameLoopError::InvalidState(
                        "planned delve card is no longer available".to_string(),
                    ));
                }
                pay_selected_cost(
                    game,
                    &crate::costs::Cost::exile_from_graveyard(1, None),
                    pending.spell_id,
                    pending.caster,
                    crate::costs::PaymentReason::CastSpell,
                    pending.provenance,
                    card_id,
                    None,
                    &mut pending.tagged_objects,
                    &mut pending.effect_outcomes,
                    _decision_maker,
                )?;
                continue;
            }
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
            let tap_provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::PermanentTapped);
            if !taps.tap_with_event_provenance(game, permanent_id, tap_provenance) {
                return Err(GameLoopError::InvalidState(format!(
                    "planned {effect:?} permanent {permanent_id:?} could not be tapped"
                )));
            }
            let event_provenance = game
                .provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::KeywordAction);
            let completion = crate::effects::composition::observe_keyword_action_completion(
                game,
                TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        keyword_action_from_alternative_effect(effect),
                        pending.caster,
                        pending.spell_id,
                        1,
                    ),
                    event_provenance,
                ),
            )
            .map_err(GameLoopError::ExecutionFailed)?;
            for event in completion.events {
                queue_triggers_from_event(game, trigger_queue, event, true);
            }
            record_keyword_payment_contribution(
                &mut pending.keyword_payment_contributions,
                permanent_id,
                effect,
            );
        }
        Ok(())
    })();
    if result.is_ok() {
        for event in taps.finish(game).events {
            game.queue_trigger_event(event.provenance(), event);
        }
    }
    game.close_simultaneous_action(opened_batch);
    result?;
    try_drain_pending_trigger_events(game, trigger_queue)?;
    Ok(())
}

pub(super) fn prompt_pending_mana_ability_payment(
    game: &mut GameState,
    state: &mut PriorityLoopState,
    mut pending: PendingManaAbility,
    subject: String,
) -> Result<GameProgress, GameLoopError> {
    let refining_existing_plan = pending.pending_mana_payment.is_some();
    let spend_policy = game.mana_spend_policy(pending.activator, Some(pending.source));
    let mut request = crate::mana_payment::ManaPaymentRequest::new(
        pending.activator,
        pending.source,
        pending.payment_reason,
        pending.mana_cost.clone(),
    )
    .with_spend_policy(spend_policy);
    if let Some(existing) = pending.pending_mana_payment.as_ref() {
        request.preferences = existing.request.preferences.clone();
    }
    request.activation_excluded_sources.extend(
        state
            .pending_mana_parents
            .iter()
            .map(|parent| parent.source),
    );
    if pending.other_costs.iter().any(|cost| cost.requires_tap()) {
        request.reserved_tap_sources.push(pending.source);
    }
    request.preferences.normalize();
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(
            pending.activator,
            Some(pending.source),
            pending.payment_reason,
        );
    let plan_result =
        crate::mana_payment::plan_prompt_mana_payment(game, &request, !refining_existing_plan);
    let plan_result = plan_result.or_else(|failure| {
        if refining_existing_plan
            && matches!(
                failure,
                crate::mana_payment::ManaPaymentFailure::NoLegalPlan
                    | crate::mana_payment::ManaPaymentFailure::SearchLimitReached
                    | crate::mana_payment::ManaPaymentFailure::ConflictingPreferences
            )
        {
            Ok(crate::mana_payment::unfunded_mana_payment_plan(
                game, &request,
            ))
        } else {
            Err(failure)
        }
    });
    let plan = plan_result.map_err(|failure| {
        state.rollback_action(game);
        if let crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error) = failure {
            return GameLoopError::ExecutionFailed(error);
        }
        GameLoopError::ActionCancelled(format!(
            "the mana ability's activation cost has no legal payment plan: {failure:?}"
        ))
    })?;
    pending.pending_mana_payment = Some(if refining_existing_plan {
        crate::mana_payment::PendingManaPayment::new(request.clone(), plan.clone())
    } else {
        crate::mana_payment::PendingManaPayment::provisional(request.clone(), plan.clone())
    });
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
        if let Some(index) = request
            .preferences
            .required_activations
            .iter()
            .position(|selected| {
                selected.source == activated.source
                    && selected.ability_index == activated.ability_index
                    && selected.color_restriction == activated.color_restriction
            })
        {
            request.preferences.required_activations.remove(index);
        }
    }
    request.preferences.normalize();
    let plan = crate::mana_payment::plan_mana_payment(game, &request)
        .map_err(|failure| {
            if let crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error) = failure {
                return GameLoopError::ExecutionFailed(error);
            }
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
    let checkpoint = (game.clone(), trigger_queue.clone(), state.clone());
    let mut pending_checkpoint = pending.clone();
    pending_checkpoint.pending_mana_payment = Some(payment.clone());
    let result = (|| -> Result<GameProgress, GameLoopError> {
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
        if !game
            .try_pay_mana_cost_with_payment_options_and_dm(
                payment.request.payer,
                Some(payment.request.source),
                &payment.plan.mana_cost_after_alternatives,
                payment.request.x_value,
                payment.request.reason,
                &payment.request.spend_policy,
                payment.request.allow_life_payment,
                payment.request.allow_black_life,
                payment.request.preferences.prefer_life,
                decision_maker,
            )
            .map_err(|error| {
                state.rollback_action(game);
                GameLoopError::ExecutionFailed(error)
            })?
        {
            if decision_maker.awaiting_choice() {
                return Ok(GameProgress::Continue);
            }
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "spell payment failed validation and was rolled back".to_string(),
            ));
        }
        crate::mana_payment::record_waterbend_payment(game, &payment.request).map_err(|error| {
            state.rollback_action(game);
            GameLoopError::ExecutionFailed(error)
        })?;
        let pool_after = game
            .player(pending.caster)
            .map(|player| player.mana_pool.clone())
            .unwrap_or_default();
        add_spent_pool_delta(&mut pending.mana_spent_to_cast, &pool_before, &pool_after);
        pending.mana_cost_to_pay = None;
        pending.pending_mana_payment = None;
        pending.stage = spell_stage_after_targets(&pending);
        continue_spell_next_cost_or_finalize(game, trigger_queue, state, pending, decision_maker)
    })();
    if decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint.0, result.is_ok());
        *trigger_queue = checkpoint.1;
        *state = checkpoint.2;
        state.pending_cast = Some(pending_checkpoint);
        return Ok(GameProgress::Continue);
    }
    result
}

pub(super) fn commit_prepared_activation_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    mut pending: PendingActivation,
    mut payment: crate::mana_payment::PendingManaPayment,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let checkpoint = (game.clone(), trigger_queue.clone(), state.clone());
    let mut pending_checkpoint = pending.clone();
    pending_checkpoint.pending_mana_payment = Some(payment.clone());
    let result = (|| -> Result<GameProgress, GameLoopError> {
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
            if let Some(index) =
                request
                    .preferences
                    .required_activations
                    .iter()
                    .position(|selected| {
                        selected.source == activated.source
                            && selected.ability_index == activated.ability_index
                            && selected.color_restriction == activated.color_restriction
                    })
            {
                request.preferences.required_activations.remove(index);
            }
        }
        request.preferences.normalize();
        let plan = match crate::mana_payment::plan_mana_payment(game, &request) {
            Ok(plans) => plans.into_iter().next().ok_or_else(|| {
                GameLoopError::InvalidState(
                    "planner returned no prepared activation plan".to_string(),
                )
            })?,
            Err(failure) => {
                state.rollback_action(game);
                if let crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error) =
                    failure
                {
                    return Err(GameLoopError::ExecutionFailed(error));
                }
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
        execute_planned_waterbend_taps(game, &payment)?;
        if !game
            .try_pay_mana_cost_with_payment_options_and_dm(
                payment.request.payer,
                Some(payment.request.source),
                &payment.plan.mana_cost_after_alternatives,
                payment.request.x_value,
                payment.request.reason,
                &payment.request.spend_policy,
                payment.request.allow_life_payment,
                payment.request.allow_black_life,
                payment.request.preferences.prefer_life,
                decision_maker,
            )
            .map_err(|error| {
                state.rollback_action(game);
                GameLoopError::ExecutionFailed(error)
            })?
        {
            if decision_maker.awaiting_choice() {
                return Ok(GameProgress::Continue);
            }
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "activation payment failed validation and was rolled back".to_string(),
            ));
        }
        crate::mana_payment::record_waterbend_payment(game, &payment.request).map_err(|error| {
            state.rollback_action(game);
            GameLoopError::ExecutionFailed(error)
        })?;
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
    })();
    if decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint.0, result.is_ok());
        *trigger_queue = checkpoint.1;
        *state = checkpoint.2;
        state.pending_activation = Some(pending_checkpoint);
        return Ok(GameProgress::Continue);
    }
    result
}

fn revalidate_authoritative_payment_plan(
    game: &GameState,
    payment: &crate::mana_payment::PendingManaPayment,
    label: &str,
) -> Result<crate::mana_payment::ManaPaymentPlan, GameLoopError> {
    // A displayed first plan is already authoritative. Validate that same
    // proposal without running the optional ranking pass on confirmation.
    match crate::mana_payment::plan_first_mana_payment(game, &payment.request) {
        Ok(plan)
            if plan.id == payment.plan.id && plan.request_hash == payment.plan.request_hash =>
        {
            return Ok(plan);
        }
        Err(crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error)) => {
            return Err(GameLoopError::ExecutionFailed(error));
        }
        _ => {}
    }
    crate::mana_payment::plan_mana_payment(game, &payment.request)
        .map_err(|failure| {
            if let crate::mana_payment::ManaPaymentFailure::EffectExecutionFailed(error) = failure {
                return GameLoopError::ExecutionFailed(error);
            }
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
    // A captured nested decision replays this response against the same
    // authoritative payment. Cancellation retains its existing action rollback.
    let checkpoint = (!matches!(response, crate::mana_payment::ManaPaymentResponse::Cancel)
        && (state.pending_mana_ability.is_some()
            || state.pending_cast.is_some()
            || state.pending_activation.is_some()
            || matches!(
                response,
                crate::mana_payment::ManaPaymentResponse::Activate { .. }
            )))
    .then(|| (game.clone(), trigger_queue.clone(), state.clone()));
    let result = apply_mana_payment_plan_response_inner(
        game,
        trigger_queue,
        state,
        response,
        decision_maker,
    );
    if let Some((before_game, before_queue, before_state)) = checkpoint {
        if decision_maker.awaiting_choice()
            || matches!(&result, Err(GameLoopError::ExecutionFailed(_)))
        {
            game.restore_execution_checkpoint(
                before_game,
                result.is_ok() && decision_maker.awaiting_choice(),
            );
            *trigger_queue = before_queue;
            *state = before_state;
        }
        if decision_maker.awaiting_choice() {
            return Ok(GameProgress::Continue);
        }
    }
    result
}

fn lock_enclosing_mana_undo(state: &mut PriorityLoopState, locked: bool) {
    if !locked {
        return;
    }
    if let Some(pending) = state.pending_mana_ability.as_mut() {
        pending.undo_locked_by_mana = true;
    }
    for parent in &mut state.pending_mana_parents {
        parent.undo_locked_by_mana = true;
    }
    if let Some(pending) = state.pending_activation.as_mut() {
        pending.undo_locked_by_mana = true;
    }
    if let Some(pending) = state.pending_cast.as_mut() {
        pending.undo_locked_by_mana = true;
    }
}

/// Completed child activations can satisfy selections made in any enclosing
/// payment. Preserve the exact ability/color occurrence and payer: these
/// constraints are a multiset, not a set of source objects.
fn discharge_enclosing_mana_selections(
    state: &mut PriorityLoopState,
    payer: PlayerId,
    completed: &[crate::mana_payment::RequiredManaActivation],
) {
    let settle = |payment: &mut crate::mana_payment::PendingManaPayment| {
        if payment.request.payer != payer {
            return;
        }
        let preferences = &mut payment.request.preferences;
        for activation in completed {
            preferences
                .required_sources
                .retain(|source| *source != activation.source);
            if let Some(index) = preferences
                .required_activations
                .iter()
                .position(|selected| selected == activation)
            {
                preferences.required_activations.remove(index);
            }
        }
        preferences.normalize();
    };
    for parent in state
        .pending_mana_parents
        .iter_mut()
        .chain(state.pending_mana_ability.iter_mut())
    {
        if let Some(payment) = parent.pending_mana_payment.as_mut() {
            settle(payment);
        }
    }
    if let Some(payment) = state
        .pending_activation
        .as_mut()
        .and_then(|pending| pending.pending_mana_payment.as_mut())
    {
        settle(payment);
    }
    if let Some(payment) = state
        .pending_cast
        .as_mut()
        .and_then(|pending| pending.pending_mana_payment.as_mut())
    {
        settle(payment);
    }
}

fn resume_enclosing_mana_payment(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let payment = state
        .pending_mana_ability
        .as_ref()
        .and_then(|pending| pending.pending_mana_payment.as_ref())
        .or_else(|| {
            state
                .pending_activation
                .as_ref()
                .and_then(|pending| pending.pending_mana_payment.as_ref())
        })
        .or_else(|| {
            state
                .pending_cast
                .as_ref()
                .and_then(|pending| pending.pending_mana_payment.as_ref())
        });
    if let Some(payment) = payment {
        let response = crate::mana_payment::ManaPaymentResponse::Replan {
            preferences: payment.request.preferences.clone(),
        };
        return apply_mana_payment_plan_response(
            game,
            trigger_queue,
            state,
            &response,
            decision_maker,
        );
    }
    advance_priority_with_dm(game, trigger_queue, decision_maker)
}

fn apply_mana_payment_plan_response_inner(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &crate::mana_payment::ManaPaymentResponse,
    decision_maker: &mut impl DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    use crate::mana_payment::ManaPaymentResponse;
    game.refresh_continuous_state();

    if let ManaPaymentResponse::Activate {
        source,
        ability_index,
    } = response
    {
        let payment = state
            .pending_mana_ability
            .as_ref()
            .and_then(|pending| pending.pending_mana_payment.as_ref())
            .or_else(|| {
                state
                    .pending_activation
                    .as_ref()
                    .and_then(|pending| pending.pending_mana_payment.as_ref())
            })
            .or_else(|| {
                state
                    .pending_cast
                    .as_ref()
                    .and_then(|pending| pending.pending_mana_payment.as_ref())
            })
            .ok_or_else(|| GameLoopError::InvalidState("no mana payment is active".to_string()))?;
        let request = payment.request.clone();
        let undo_safe = mana_ability_is_undo_safe(game, *source, *ability_index);
        let manual = crate::mana_payment::manual_mana_abilities_checked(game, &request)
            .map_err(GameLoopError::ExecutionFailed)?;
        if !manual.contains(&(*source, *ability_index)) {
            return Err(GameLoopError::InvalidState(
                "illegal mana activation during payment".to_string(),
            ));
        }
        if let Some(parent) = state.pending_mana_ability.take() {
            state.pending_mana_parents.push(parent);
        }
        let progress = super::priority_apply::begin_mana_ability_activation(
            game,
            trigger_queue,
            state,
            source,
            ability_index,
            request.payer,
            decision_maker,
        )?;
        if decision_maker.awaiting_choice() {
            return Ok(GameProgress::Continue);
        }
        if state.pending_mana_ability.is_some() {
            return Ok(progress);
        }
        state.pending_mana_ability = state.pending_mana_parents.pop();
        lock_enclosing_mana_undo(state, !undo_safe);
        let completed = crate::mana_payment::RequiredManaActivation {
            source: *source,
            ability_index: *ability_index,
            color_restriction: None,
        };
        discharge_enclosing_mana_selections(state, request.payer, std::slice::from_ref(&completed));
        return resume_enclosing_mana_payment(game, trigger_queue, state, decision_maker);
    }

    if matches!(response, ManaPaymentResponse::Cancel) && state.opened_exile_play.is_some() {
        return Err(GameLoopError::InvalidState(
            "The exile play publicly opened a card; finish its announcement".into(),
        ));
    }
    if matches!(response, ManaPaymentResponse::Cancel) {
        let canceled = state.pending_mana_ability.take();
        let canceled_provenance = canceled.as_ref().map(|pending| pending.provenance);
        if let Some(mut child) = canceled {
            game.finish_library_top_announcement(
                crate::game_state::LibraryTopAnnouncement::Activation(child.provenance),
            );
            if let Some(announcement) = child.exhaust_announcement.take() {
                game.cancel_exhaust_announcement(announcement);
            }
            if !state.pending_mana_parents.is_empty()
                || state.pending_cast.is_some()
                || state.pending_activation.is_some()
            {
                state.pending_mana_ability = state.pending_mana_parents.pop();
                return resume_enclosing_mana_payment(game, trigger_queue, state, decision_maker);
            }
        }
        let blind_queue = if state.declared_exile_face_down.is_some() {
            Some(
                state
                    .exile_face_down_root_queue
                    .as_ref()
                    .ok_or_else(|| {
                        crate::effects::ExecutionError::IncompleteEvidence(
                            "face-down payment cancellation lost its original trigger queue".into(),
                        )
                    })?
                    .as_ref()
                    .clone(),
            )
        } else {
            None
        };
        if blind_queue.is_some() && state.checkpoint.is_none() {
            return Err(crate::effects::ExecutionError::IncompleteEvidence(
                "face-down payment cancellation lost its gameplay checkpoint".into(),
            )
            .into());
        }
        state.rollback_action(game);
        if let Some(original_queue) = blind_queue {
            *trigger_queue = original_queue;
        }
        // Also cover an older/incomplete root checkpoint that already held
        // this frame. Never remove an enclosing cast or a different mana owner.
        if let Some(provenance) = canceled_provenance {
            game.finish_library_top_announcement(
                crate::game_state::LibraryTopAnnouncement::Activation(provenance),
            );
        }
        state.pending_mana_ability = None;
        state.pending_mana_parents.clear();
        if state.pending_exile_face_down.is_some() {
            return super::exile_face_down::resume(game, state);
        }
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
            } if payment.plan.payable
                && *plan_id == payment.plan.id
                && *request_hash == payment.plan.request_hash => {}
            ManaPaymentResponse::Confirm { .. } => {
                pending.pending_mana_payment = Some(payment);
                state.pending_mana_ability = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "stale or client-authored mana-ability payment plan".to_string(),
                ));
            }
            ManaPaymentResponse::Cancel | ManaPaymentResponse::Activate { .. } => unreachable!(),
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
        execute_planned_waterbend_taps(game, &payment)?;
        if !game
            .try_pay_mana_cost_with_payment_options_and_dm(
                payment.request.payer,
                Some(payment.request.source),
                &payment.plan.mana_cost_after_alternatives,
                payment.request.x_value,
                payment.request.reason,
                &payment.request.spend_policy,
                payment.request.allow_life_payment,
                payment.request.allow_black_life,
                payment.request.preferences.prefer_life,
                decision_maker,
            )
            .map_err(|error| {
                state.rollback_action(game);
                GameLoopError::ExecutionFailed(error)
            })?
        {
            if decision_maker.awaiting_choice() {
                return Ok(GameProgress::Continue);
            }
            state.rollback_action(game);
            return Err(GameLoopError::ActionCancelled(
                "mana-ability payment failed validation and was rolled back".to_string(),
            ));
        }
        crate::mana_payment::record_waterbend_payment(game, &payment.request).map_err(|error| {
            state.rollback_action(game);
            GameLoopError::ExecutionFailed(error)
        })?;
        pending.mana_cost = crate::mana::ManaCost::new();
        pending.pending_mana_payment = None;
        if let Err(error) =
            execute_pending_mana_ability(game, trigger_queue, &pending, decision_maker)
        {
            state.rollback_action(game);
            return Err(error);
        }
        if decision_maker.awaiting_choice() {
            return Ok(GameProgress::Continue);
        }
        state.pending_mana_ability = state.pending_mana_parents.pop();
        lock_enclosing_mana_undo(state, pending.undo_locked_by_mana);
        let completed = payment
            .plan
            .mana_ability_steps
            .iter()
            .map(|step| crate::mana_payment::RequiredManaActivation {
                source: step.source,
                ability_index: step.ability_index,
                color_restriction: step.color_restriction.clone(),
            })
            .chain(std::iter::once(
                crate::mana_payment::RequiredManaActivation {
                    source: pending.source,
                    ability_index: pending.ability_index,
                    color_restriction: None,
                },
            ))
            .collect::<Vec<_>>();
        discharge_enclosing_mana_selections(state, pending.activator, &completed);
        // A completed deferred root must not lend its old pre-payment
        // checkpoint to the next, unrelated mana announcement.
        if !state.has_pending_action() {
            state.clear_checkpoint();
        }
        return resume_enclosing_mana_payment(game, trigger_queue, state, decision_maker);
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
            } if payment.plan.payable
                && *plan_id == payment.plan.id
                && *request_hash == payment.plan.request_hash => {}
            ManaPaymentResponse::Confirm { .. } => {
                pending.pending_mana_payment = Some(payment);
                state.pending_activation = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "stale or client-authored activation payment plan".to_string(),
                ));
            }
            ManaPaymentResponse::Cancel | ManaPaymentResponse::Activate { .. } => unreachable!(),
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
            } if payment.plan.payable
                && *plan_id == payment.plan.id
                && *request_hash == payment.plan.request_hash => {}
            ManaPaymentResponse::Confirm { .. } => {
                pending.pending_mana_payment = Some(payment);
                state.pending_cast = Some(pending);
                return Err(GameLoopError::InvalidState(
                    "stale or client-authored Assist payment plan".to_string(),
                ));
            }
            ManaPaymentResponse::Cancel | ManaPaymentResponse::Activate { .. } => unreachable!(),
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
        execute_planned_waterbend_taps(game, &payment)?;
        if !game
            .try_pay_mana_cost_with_payment_options_and_dm(
                payment.request.payer,
                Some(payment.request.source),
                &payment.plan.mana_cost_after_alternatives,
                payment.request.x_value,
                payment.request.reason,
                &payment.request.spend_policy,
                payment.request.allow_life_payment,
                payment.request.allow_black_life,
                payment.request.preferences.prefer_life,
                decision_maker,
            )
            .map_err(|error| {
                state.rollback_action(game);
                GameLoopError::ExecutionFailed(error)
            })?
        {
            if decision_maker.awaiting_choice() {
                return Ok(GameProgress::Continue);
            }
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
        } if payment.plan.payable
            && *plan_id == payment.plan.id
            && *request_hash == payment.plan.request_hash => {}
        ManaPaymentResponse::Confirm { .. } => {
            pending.pending_mana_payment = Some(payment);
            state.pending_cast = Some(pending);
            return Err(GameLoopError::InvalidState(
                "stale or client-authored spell payment plan".to_string(),
            ));
        }
        ManaPaymentResponse::Cancel | ManaPaymentResponse::Activate { .. } => unreachable!(),
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
        let restriction = crate::effects::composition::previously_chosen_mode_restriction(
            game,
            pending.source,
            pending.effects.all_effects(),
        );
        if modes.iter().any(|&mode| {
            crate::effects::composition::restricted_mode_was_chosen(
                game,
                pending.source,
                restriction,
                mode,
            )
        }) {
            return Err(GameLoopError::InvalidState(
                "Selected mode was already chosen".to_string(),
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

    // CR 601.2b: "Choose X." — the number of modes chosen is the announced X.
    if let Some(modal_spec) = extract_modal_spec_from_spell(game, pending.spell_id, pending.caster)
        && super::priority_cast::x_defined_mode_count_range(game, &pending, &modal_spec).is_some()
        && let Some(total) = super::priority_cast::mode_point_total(&modal_spec, modes)
    {
        let x = total as u32;
        pending.x_value = Some(x);
        if let Some(spell) = game.object_mut(pending.spell_id) {
            spell.x_value = Some(x);
        }
    }
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
    // Branch options of a one-of optional cost name the cost and the
    // announced payment branch.
    let mut announced_branches = Vec::new();
    let decoded_choices = choices
        .iter()
        .map(|&(option, times)| {
            let (index, branch) = super::priority_cast::decode_optional_cost_branch_option(option);
            if let Some(branch) = branch {
                announced_branches.push((index, branch));
            }
            (index, times)
        })
        .collect::<Vec<_>>();
    let choices = decoded_choices.as_slice();
    if announced_branches.iter().any(|&(index, branch)| {
        optional_costs
            .get(index)
            .and_then(|cost| cost.cost.as_one_of())
            .is_none_or(|branches| branch >= branches.len())
    }) {
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
            "optional-cost response names an invalid payment branch".to_string(),
        ));
    }
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
    for &(index, branch) in &announced_branches {
        pending.optional_costs_paid.set_branch_choice(index, branch);
    }

    if let Some(spell) = game.object_mut(pending.spell_id) {
        spell.optional_costs_paid = pending.optional_costs_paid.clone();
        // CR 718.3b: a spell cast prototyped has its prototype mana cost,
        // colors, and power/toughness, whatever cost is paid for it.
        if pending
            .optional_costs_paid
            .was_paid_label(super::priority_cast::PROTOTYPE_CHOICE_LABEL)
            && let Some((cost, power_toughness)) =
                super::priority_cast::spell_prototype_characteristics(spell)
        {
            spell.apply_prototype_cast_overlay(cost, power_toughness);
        }
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
        // Reject the announcement cleanly (e.g. entwine with a mode that has
        // no legal target) instead of leaving the spell half-cast.
        state.rollback_action(game);
        return Err(GameLoopError::ActionCancelled(
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
                if !crate::decision::with_complete_legality_query(game, |checked| {
                    Ok(spell_mana_payment_is_legal(checked, &pending))
                })
                .map_err(|error| {
                    state.pending_cast = Some(pending.clone());
                    GameLoopError::ExecutionFailed(error)
                })? {
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
            if crate::decision::with_complete_legality_query(game, |checked| {
                Ok(max_assist_generic_contribution(checked, &pending))
            })
            .map_err(|error| {
                state.pending_cast = Some(pending.clone());
                GameLoopError::ExecutionFailed(error)
            })? == 0
            {
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
            if !crate::decision::with_complete_legality_query(game, |checked| {
                Ok(assist_generic_contribution_is_legal(
                    checked,
                    &pending,
                    assistant,
                    contribution,
                ))
            })
            .map_err(|error| {
                state.pending_cast = Some(pending.clone());
                GameLoopError::ExecutionFailed(error)
            })? {
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

/// Retain executable failures from cost replacements so the owning response
/// restores its authoritative payment rather than cancelling it as invalid.
pub(super) fn activation_cost_error(error: crate::cost::CostPaymentError) -> GameLoopError {
    match error {
        crate::cost::CostPaymentError::ExecutionFailed(error) => {
            GameLoopError::ExecutionFailed(error)
        }
        error => GameLoopError::InvalidState(format!("Failed to pay cost: {error}")),
    }
}

pub(super) fn execute_pending_mana_ability(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    pending: &PendingManaAbility,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), GameLoopError> {
    let checkpoint = (game.clone(), trigger_queue.clone());
    let result = (|| -> Result<(), GameLoopError> {
        use crate::costs::CostContext;
        use crate::effects::ExecutionContext;

        crate::linked_exile::validate_program_owner(
            pending.effects.linked_exile_pair,
            pending.linked_exile_owner.as_ref(),
        )
        .map_err(GameLoopError::ExecutionFailed)?;
        game.begin_library_top_announcement(crate::game_state::LibraryTopAnnouncement::Activation(
            pending.provenance,
        ));
        // Snapshot with continuous effects applied so tap-for-mana triggers see the
        // source's real characteristics (an animated land is a creature only there).
        let source_snapshot = game
            .object(pending.source)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));

        // Pay the mana cost
        if !game
            .try_pay_mana_cost_with_reason_and_dm(
                pending.activator,
                Some(pending.source),
                &pending.mana_cost,
                0,
                pending.payment_reason,
                decision_maker,
            )
            .map_err(GameLoopError::ExecutionFailed)?
        {
            if decision_maker.awaiting_choice() {
                return Ok(());
            }
            return Err(GameLoopError::InvalidState(
                "Failed to pay mana cost".to_string(),
            ));
        }

        // Pay other costs from TotalCost
        let mut cost_ctx = CostContext::new(pending.source, pending.activator, decision_maker)
            .with_reason(pending.payment_reason)
            .with_provenance(pending.provenance);
        cost_ctx.x_value = pending.x_value;
        for c in &pending.other_costs {
            crate::special_actions::pay_cost_component_with_choice(game, c, &mut cost_ctx)
                .map_err(activation_cost_error)?;
            if cost_ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        // X is bound by the announced {X} or by a cost that fixes it as it is
        // paid ("Remove X storage counters"); the effect reads that value.
        let x_value_from_costs = cost_ctx.x_value;
        let cost_tagged_objects = cost_ctx.tagged_objects.clone();
        let cost_effect_outcomes = cost_ctx.effect_outcomes.clone();
        drop(cost_ctx);
        game.finish_library_top_announcement(
            crate::game_state::LibraryTopAnnouncement::Activation(pending.provenance),
        );
        try_drain_pending_trigger_events(game, trigger_queue)?;

        game.record_ability_activation_with_origin(
            pending.source,
            pending.ability_index,
            pending.activation_origin.clone(),
            pending.effects.activation_definition,
        );
        let mut mana_ctx =
            ExecutionContext::new(pending.source, pending.activator, &mut *decision_maker)
                .with_activation_origin(pending.activation_origin.clone())
                .with_activation_definition(pending.effects.activation_definition)
                .with_ability_index(pending.ability_index)
                .with_linked_exile_owner(pending.linked_exile_owner.clone())
                .with_source_number_owner(pending.source_number_owner.clone())
                .with_provenance(pending.provenance)
                .with_mana_usage_restrictions(pending.mana_usage_restrictions.clone())
                .with_mana_source_chosen_creature_type(pending.mana_source_chosen_creature_type)
                .with_mana_production_provenance(pending.mana_production_provenance)
                .with_tagged_objects(cost_tagged_objects.clone())
                .with_effect_outcomes(cost_effect_outcomes.clone());
        if let Some(snapshot) = source_snapshot.clone() {
            mana_ctx = mana_ctx.with_source_snapshot(snapshot);
        }
        if let Some(x) = x_value_from_costs {
            mana_ctx = mana_ctx.with_x(x);
        }
        let outcome = crate::effects::EffectExecutor::execute(
            &crate::effects::AddManaEffect::new(
                pending.mana_to_add.clone(),
                crate::target::PlayerFilter::Specific(pending.activator),
            ),
            game,
            &mut mana_ctx,
        )?;
        if mana_ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        drop(mana_ctx);
        super::targeting::queue_triggers_for_events_including_delayed(
            game,
            trigger_queue,
            outcome.events,
        )?;

        // Execute additional effects (for complex mana abilities)
        if !pending.effects.is_empty() {
            let mut ctx = ExecutionContext::new(pending.source, pending.activator, decision_maker)
                .with_activation_origin(pending.activation_origin.clone())
                .with_activation_definition(pending.effects.activation_definition)
                .with_ability_index(pending.ability_index)
                .with_linked_exile_owner(pending.linked_exile_owner.clone())
                .with_source_number_owner(pending.source_number_owner.clone())
                .with_provenance(pending.provenance)
                .with_mana_usage_restrictions(pending.mana_usage_restrictions.clone())
                .with_mana_source_chosen_creature_type(pending.mana_source_chosen_creature_type)
                .with_mana_production_provenance(pending.mana_production_provenance);
            if let Some(snapshot) = source_snapshot.clone() {
                ctx = ctx.with_source_snapshot(snapshot);
            }
            if let Some(x) = x_value_from_costs {
                ctx = ctx.with_x(x);
            }
            ctx = ctx
                .with_tagged_objects(cost_tagged_objects)
                .with_effect_outcomes(cost_effect_outcomes);
            let emitted_events = crate::game_loop::execute_resolution_program(
                game,
                &mut ctx,
                pending.activator,
                pending.source,
                &pending.effects,
                None,
                &[],
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            queue_triggers_for_events(game, trigger_queue, emitted_events)?;
            try_drain_pending_trigger_events(game, trigger_queue)?;
        }

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
        )?;

        Ok(())
    })();
    if result.is_err() || decision_maker.awaiting_choice() {
        *game = checkpoint.0;
        *trigger_queue = checkpoint.1;
    }
    result
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
    let raw = captured_activation_reference_branch(&pending, choice)?;
    let payable = match crate::cost::prospective_references::activation_branch_preflight_checked(
        game,
        pending.source,
        pending.ability_index,
        pending.activator,
        raw.as_ref(),
        &branch,
    ) {
        Ok(payable) => payable,
        Err(error) => {
            state.pending_activation = Some(pending);
            return Err(GameLoopError::ExecutionFailed(error));
        }
    };
    if !payable {
        state.pending_activation = Some(pending);
        return Err(GameLoopError::ActionCancelled(
            "the selected activation cost branch cannot be paid".to_string(),
        ));
    }

    assign_pending_activation_cost(game, &mut pending, &branch, decision_maker)?;
    if decision_maker.awaiting_choice() {
        state.pending_activation = Some(pending);
        return Ok(GameProgress::Continue);
    }
    pending.selected_alternative_cost = Some(choice);
    if let Some(base) = pending.cost_reference_base.as_ref() {
        let branch = base
            .as_one_of()
            .and_then(|branches| branches.get(choice))
            .ok_or_else(|| {
                GameLoopError::InvalidState("missing captured reference-cost branch".into())
            })?;
        pending.cost_reference_choices =
            crate::cost::prospective_references::activation_reference_choices(
                branch,
                pending.effects.flattened_default_effects(),
            )
            .map_err(|error| {
                GameLoopError::InvalidState(format!("reference-cost branch: {error:?}"))
            })?;
    }
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
        ActivationStage::ChoosingCostReferences => {
            let choice = pending.cost_reference_choices.first().ok_or_else(|| {
                GameLoopError::InvalidState("missing public cost reference choice".into())
            })?;
            let candidates = crate::cost::prospective_references::public_reference_candidates(
                game,
                pending.source,
                pending.activator,
                choice,
                &pending.tagged_objects,
                pending.x_value.map(|x| x as u32),
            );
            if !candidates.contains(&target_id) {
                return Err(GameLoopError::InvalidState(
                    "ineligible public cost reference".into(),
                ));
            }
            let object = game
                .object(target_id)
                .ok_or_else(|| GameLoopError::InvalidState("cost reference departed".into()))?;
            let snapshot =
                ObjectSnapshot::from_object_with_calculated_characteristics(object, game);
            pending
                .announced_cost_objects
                .insert(choice.tag.clone(), vec![snapshot.clone()]);
            pending
                .tagged_objects
                .insert(choice.tag.clone(), vec![snapshot]);
            pending.cost_reference_choices.remove(0);
        }

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
                &mut pending.effect_outcomes,
                decision_maker,
            )?;
            if decision_maker.awaiting_choice() {
                state.pending_activation = Some(pending);
                return Ok(GameProgress::Continue);
            }

            try_drain_pending_trigger_events(game, trigger_queue)?;

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
                ActivationCardCostChoice::Discard { cost, filter, .. } => {
                    let legal_cards =
                        get_legal_discard_cards(game, pending.activator, pending.source, &filter);
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_activation = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
            let (cost, filter, choice_tag, is_emerge_resource) =
                match pending.remaining_cost_steps.first() {
                    Some(ActivationCostStep::Sacrifice {
                        cost,
                        filter,
                        choice_tag,
                        is_emerge_resource,
                        ..
                    }) => (
                        cost.clone(),
                        filter.clone(),
                        choice_tag.clone(),
                        *is_emerge_resource,
                    ),
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
            let completed_sacrifice = pay_selected_cost(
                game,
                &cost,
                pending.spell_id,
                pending.caster,
                crate::costs::PaymentReason::CastSpell,
                pending.provenance,
                chosen_id,
                Some(&choice_tag),
                &mut pending.tagged_objects,
                &mut pending.effect_outcomes,
                decision_maker,
            )?;
            if decision_maker.awaiting_choice() {
                state.pending_cast = Some(pending);
                return Ok(GameProgress::Continue);
            }

            // The preannounced Emerge resource is the exact creature whose
            // mana value reduced the cost. Retain the paid snapshot, not a
            // later battlefield/graveyard lookup or an arbitrary sacrifice.
            if is_emerge_resource {
                let receipt = completed_sacrifice
                    .filter(|snapshots| {
                        snapshots.is_empty()
                            || (snapshots.len() == 1 && snapshots[0].object_id == chosen_id)
                    })
                    .ok_or_else(|| {
                        GameLoopError::InvalidState(
                            "paid Emerge cost is missing its original sacrifice receipt".into(),
                        )
                    })?;
                pending
                    .tagged_objects
                    .insert(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG.into(), receipt);
            }

            try_drain_pending_trigger_events(game, trigger_queue)?;

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
                ActivationCardCostChoice::Discard { cost, filter, .. } => {
                    let legal_cards =
                        get_legal_discard_cards(game, pending.caster, pending.spell_id, &filter);
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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

                    // CR 702.66a / 603.2c: the cards chosen one at a time for
                    // delve are all exiled as part of paying the cost, one
                    // event ("whenever one or more cards leave your
                    // graveyard"). They share the cast's identity and are
                    // matched together once the delve choices are over.
                    let delve = selected_delve_reduction > 0;
                    let opened_batch =
                        delve && game.open_simultaneous_action_with_batch(pending.provenance);
                    let paid = pay_selected_cost(
                        game,
                        &cost,
                        pending.spell_id,
                        pending.caster,
                        crate::costs::PaymentReason::CastSpell,
                        pending.provenance,
                        chosen_id,
                        None,
                        &mut pending.tagged_objects,
                        &mut pending.effect_outcomes,
                        decision_maker,
                    );
                    game.close_simultaneous_action(opened_batch);
                    paid?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    if !delve {
                        try_drain_pending_trigger_events(game, trigger_queue)?;
                    }
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
                        &mut pending.effect_outcomes,
                        decision_maker,
                    )?;
                    if decision_maker.awaiting_choice() {
                        state.pending_cast = Some(pending);
                        return Ok(GameProgress::Continue);
                    }

                    try_drain_pending_trigger_events(game, trigger_queue)?;
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
    from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
) -> Result<ObjectId, GameLoopError> {
    propose_spell_cast_with_origin(game, spell_id, from_zone, caster, casting_method, false)
}

/// Only the resolving-instruction owner supplies this authority. A UI action
/// cannot obtain a free-standing permission merely by naming a source.
pub(super) fn propose_spell_cast_from_effect(
    game: &mut GameState,
    spell_id: ObjectId,
    from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
) -> Result<ObjectId, GameLoopError> {
    propose_spell_cast_with_origin(game, spell_id, from_zone, caster, casting_method, true)
}

fn propose_spell_cast_with_origin(
    game: &mut GameState,
    spell_id: ObjectId,
    _from_zone: Zone,
    caster: PlayerId,
    casting_method: &CastingMethod,
    effect_authorized: bool,
) -> Result<ObjectId, GameLoopError> {
    if !effect_authorized {
        let checked = game
            .continuous_query_snapshot()
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
        if crate::alternative_cast::blind_play::requires_opening(&checked, spell_id, caster)
            && !crate::alternative_cast::blind_play::declared_method_is_authorized(
                &checked,
                spell_id,
                caster,
                casting_method,
            )?
        {
            return Err(GameLoopError::InvalidState(
                "Open this exiled card before announcing its spell face".into(),
            ));
        }
    }
    let has_exact_permission = matches!(casting_method, CastingMethod::ExactPermission { .. });
    let exact_grant = if has_exact_permission {
        let object = game.object(spell_id).ok_or_else(|| {
            GameLoopError::InvalidState("selected-permission spell does not exist".into())
        })?;
        crate::alternative_cast::play_permission::resolve_method(
            game,
            caster,
            object,
            casting_method,
        )?
    } else {
        None
    };
    let price_route = if matches!(
        casting_method.without_exact_permission(),
        CastingMethod::AlternativePrice { .. }
    ) {
        let object = game.object(spell_id).ok_or_else(|| {
            GameLoopError::InvalidState("Price-route spell does not exist".into())
        })?;
        if object.zone != _from_zone {
            return Err(GameLoopError::InvalidState(
                "Price route origin has changed".into(),
            ));
        }
        Some(
            crate::alternative_cast::price_routes::resolve_announcement_with_effect_authority(
                game,
                caster,
                object,
                casting_method,
                effect_authorized,
            )?
            .ok_or_else(|| {
                GameLoopError::InvalidState(
                    "Selected origin or alternative price does not authorize this exact spell face"
                        .into(),
                )
            })?,
        )
    } else {
        None
    };
    let exact_alternative = if has_exact_permission {
        crate::alternative_cast::play_permission::selected_alternative(
            game,
            caster,
            game.object(spell_id).expect("validated exact origin"),
            casting_method,
        )?
    } else {
        None
    };
    let casting_method = casting_method.origin_method();
    let visibility_boundary = game.capture_library_top_visibility_boundary();
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
    let selected_method = exact_alternative
        .or_else(|| {
            price_route
                .as_ref()
                .and_then(|route| route.origin_alternative.clone())
        })
        .or_else(|| {
            game.object(spell_id)
                .and_then(|obj| match casting_method.without_exact_permission() {
                    CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    }
                    | CastingMethod::SplitOtherHalfPlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    } => crate::decision::resolve_play_from_alternative_method(
                        game, caster, obj, *zone, *idx,
                    ),
                    CastingMethod::GrantedFlashback => Some(
                        crate::alternative_cast::AlternativeCastingMethod::Flashback {
                            total_cost: crate::cost::TotalCost::mana(
                                obj.mana_cost_owned().unwrap_or_default(),
                            ),
                        },
                    ),
                    _ => None,
                })
        });
    let selected_grant = if has_exact_permission {
        None
    } else {
        price_route
            .as_ref()
            .and_then(|route| route.origin.as_ref())
            .filter(|grant| !matches!(grant.grantable, crate::grant::Grantable::PlayFrom))
            .map(|grant| crate::grant_registry::GrantedAlternativeCast {
                permission_identity: grant.permission_identity.clone(),
                constraints: grant.play_from_constraints.clone(),
                method: selected_method
                    .clone()
                    .expect("validated priced origin has its additional-cost method"),
                source_id: grant.source.source_id(),
                zone: grant.zone,
                usage_limit: grant.usage_limit,
                cast_this_way_grants: grant.cast_this_way_grants.clone(),
                cast_this_way_filter: grant.cast_this_way_filter.clone(),
                permanent_this_way_grants: grant.permanent_this_way_grants.clone(),
                on_use_effects: grant.on_use_effects.clone(),
            })
            .or_else(|| {
                game.object(spell_id).and_then(|obj| {
                    match casting_method.without_exact_permission() {
                        CastingMethod::PlayFrom {
                            use_alternative: Some(idx),
                            zone,
                            ..
                        }
                        | CastingMethod::SplitOtherHalfPlayFrom {
                            use_alternative: Some(idx),
                            zone,
                            ..
                        } => crate::decision::resolve_play_from_alternative_grant(
                            game, caster, obj, *zone, *idx,
                        ),
                        _ => None,
                    }
                })
            })
    };
    if let Some(grant) = &selected_grant {
        let source = match casting_method.without_exact_permission() {
            CastingMethod::PlayFrom { source, .. }
            | CastingMethod::SplitOtherHalfPlayFrom { source, .. } => *source,
            _ => unreachable!(),
        };
        if grant.source_id != source {
            return Err(GameLoopError::InvalidState(
                "Selected alternative permission source does not match cast action".into(),
            ));
        }
    }
    let selected_method_for_overlay = selected_method.clone();
    // The public face-down cast kind, read before the card moves: in peer
    // matches it comes from the cast command, so peers holding only a
    // placeholder derive the same ward (see `decision::face_down_cast_kind`).
    let face_down_kind = match casting_method.without_exact_permission() {
        CastingMethod::FaceDown | CastingMethod::FaceDownPlayFrom { .. } => game
            .object(spell_id)
            .and_then(|obj| crate::decision::face_down_cast_kind(game, obj)),
        _ => None,
    };
    // A face-down cast through an effect's permission: the permission (read
    // before a single-use one is used up) supplies the claim peers check.
    let face_down_permission = face_down_kind
        .and_then(|kind| kind.permission_source())
        .and_then(|source| {
            let spell = game.object(spell_id)?;
            game.active_face_down_cast_permission(source, caster, spell.zone)
                .cloned()
        });
    // Capture before the precise exile identity and designation are retired.
    // Another permission may authorize a different price for the same card.
    let cast_was_foretold = game
        .object(spell_id)
        .is_some_and(|object| object.zone == Zone::Exile && game.is_foretold(spell_id));
    let cast_origin_snapshot = game.object(spell_id).map(|obj| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
    });
    let proposed_face = match (casting_method, game.object(spell_id)) {
        (CastingMethod::SplitOtherHalfPlayFrom { .. }, Some(object)) => {
            crate::decision::spell_view_for_split_other_half_cast(game, object)
        }
        (CastingMethod::FaceDownPlayFrom { .. }, Some(object)) => {
            if !crate::decision::spell_can_be_cast_face_down(game, object) {
                return Err(GameLoopError::InvalidState(
                    "The card has no face-down casting rule".into(),
                ));
            }
            Some(crate::decision::spell_view_for_face_down_cast(game, object))
        }
        _ => None,
    };
    let proposed_query = proposed_face
        .as_ref()
        .map(|face| crate::grant_registry::proposed_card_face_query(game, face))
        .transpose()?;
    let permission_game = proposed_query.as_ref().unwrap_or(game);
    let selected_plain_grant = if exact_grant.is_some() {
        exact_grant
    } else if let Some(origin) = price_route
        .as_ref()
        .and_then(|route| route.origin.as_ref())
        .filter(|grant| matches!(grant.grantable, crate::grant::Grantable::PlayFrom))
    {
        Some(origin.clone())
    } else if price_route.is_some() && effect_authorized {
        None
    } else if selected_grant.is_none() {
        match casting_method.without_exact_permission() {
            CastingMethod::PlayFrom { source, zone, .. }
            | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. }
            | CastingMethod::FaceDownPlayFrom { source, zone } => permission_game
                .effect_store
                .grant_registry
                .selected_play_from_grant_for_card(
                    permission_game,
                    spell_id,
                    *zone,
                    caster,
                    *source,
                ),
            _ => None,
        }
    } else {
        None
    };
    if matches!(casting_method.without_exact_permission(),
        CastingMethod::PlayFrom { .. }
            | CastingMethod::SplitOtherHalfPlayFrom { .. }
            | CastingMethod::FaceDownPlayFrom { .. })
        && selected_plain_grant.is_none() && selected_grant.is_none() && !effect_authorized
        // A priced additional-cost origin projects to PlayFrom(None), but
        // its exact derived permission has already been validated and frozen.
        && !price_route.as_ref().is_some_and(|route| route.origin.is_some())
    {
        let native_search_permission = matches!(casting_method.without_exact_permission(),
            CastingMethod::PlayFrom { source, zone: Zone::Library, use_alternative: None }
            if *source == spell_id)
            && game.current_has_static_ability_id(
                spell_id,
                crate::static_abilities::StaticAbilityId::CastThisCardFromLibraryWhileSearching,
            );
        if !native_search_permission {
            return Err(GameLoopError::InvalidState(
                "The selected play-from permission does not authorize this card face".into(),
            ));
        }
    }
    // Retain the selected permission for every granted cast, not just limited
    // ones. The receipt also pins top-only and timing scope during CR 601.2e.
    let usage_identity = selected_grant
        .as_ref()
        .and_then(|grant| grant.permission_identity.clone())
        .or_else(|| {
            selected_plain_grant
                .as_ref()
                .and_then(|grant| grant.permission_identity.clone())
        });
    let play_from_constraints = match casting_method.without_exact_permission() {
        CastingMethod::PlayFrom { source, zone, .. }
        | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. }
        | CastingMethod::FaceDownPlayFrom { source, zone } => {
            let constraints = selected_plain_grant
                .as_ref()
                .map(|grant| grant.play_from_constraints.clone())
                .or_else(|| {
                    selected_grant
                        .as_ref()
                        .map(|grant| grant.constraints.clone())
                })
                .unwrap_or_default();
            Some(Box::new((*source, *zone, constraints)))
        }
        _ => None,
    };
    // A cast through a granted alternative cost ("without paying its mana
    // cost") still uses the play permission from the same source; a
    // permission with a shared budget ("you may cast a creature spell from
    // among them", Idol of Endurance) spends it either way.
    let shared_usage_to_consume = price_route
        .as_ref()
        .and_then(|route| route.origin.as_ref())
        .and_then(|grant| grant.shared_usage_id)
        .or_else(|| match &selected_plain_grant {
            Some(grant) => grant.shared_usage_id,
            None if selected_grant.is_some() => selected_grant
                .as_ref()
                .and_then(|selected| selected.permission_identity.as_ref())
                .and_then(|identity| {
                    game.effect_store
                        .grant_registry
                        .grants
                        .iter()
                        .find(|grant| grant.permission_identity.as_ref() == Some(identity))
                })
                .and_then(|grant| grant.shared_usage_id)
                .or_else(|| match casting_method.without_exact_permission() {
                    CastingMethod::PlayFrom { source, zone, .. }
                    | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. }
                    | CastingMethod::FaceDownPlayFrom { source, zone } => game
                        .effect_store
                        .grant_registry
                        .selected_play_from_grant_for_card(game, spell_id, *zone, caster, *source)
                        .and_then(|grant| grant.shared_usage_id),
                    _ => None,
                }),
            None => None,
        });

    // Capture the exact origin and price occurrences before costs can remove
    // their providers. Recipient riders do not create a reflexive trigger.
    let permanent_riders = if let Some(route) = &price_route {
        route
            .origin
            .iter()
            .flat_map(|grant| grant.permanent_this_way_grants.iter())
            .chain(route.price.permanent_this_way_grants.iter())
            .cloned()
            .collect::<Vec<_>>()
    } else if let Some(grant) = &selected_grant {
        grant.permanent_this_way_grants.clone()
    } else {
        selected_plain_grant
            .as_ref()
            .map(|grant| grant.permanent_this_way_grants.clone())
            .unwrap_or_default()
    };

    let use_completion = if let Some(route) = &price_route {
        route.origin.as_ref().and_then(|grant| {
            crate::grant_registry::GrantUseCompletion::capture_with_snapshot(
                grant.source.source_id(),
                caster,
                grant.on_use_effects.clone(),
                route.origin_snapshot.clone(),
            )
        })
    } else {
        selected_grant
            .as_ref()
            .and_then(|grant| {
                crate::grant_registry::GrantUseCompletion::capture(
                    game,
                    grant.source_id,
                    caster,
                    grant.on_use_effects.clone(),
                )
            })
            .or_else(|| {
                selected_plain_grant.as_ref().and_then(|grant| {
                    crate::grant_registry::GrantUseCompletion::capture(
                        game,
                        grant.source.source_id(),
                        caster,
                        grant.on_use_effects.clone(),
                    )
                })
            })
    };

    let price_completion = price_route.as_ref().and_then(|route| {
        crate::grant_registry::GrantUseCompletion::capture_with_snapshot(
            route.price.source.source_id(),
            caster,
            route.price.on_use_effects.clone(),
            route.price_snapshot.clone(),
        )
    });
    let price_provider_snapshot = price_route
        .as_ref()
        .and_then(|route| route.price_snapshot.clone());
    let price_receipt = price_route
        .as_ref()
        .and_then(|route| crate::alternative_cast::price_routes::receipt_from_route(route));
    let exact_receipt = selected_plain_grant
        .as_ref()
        .filter(|grant| {
            has_exact_permission || !grant.play_from_constraints.cast_mana_spend_mode.is_normal()
        })
        .map(|grant| {
            crate::alternative_cast::play_permission::PlayPermissionReceipt::from_resolved_grant(
                grant,
                caster,
                spell_id,
                _from_zone,
                casting_method,
            )
        })
        .transpose()?;
    let new_id = game
        .move_object_by_effect(spell_id, Zone::Stack)
        .ok_or_else(|| {
            GameLoopError::InvalidState("Failed to move spell to stack during proposal".to_string())
        })?;
    game.register_library_top_announcement(
        crate::game_state::LibraryTopAnnouncement::Cast(new_id),
        visibility_boundary,
    );
    if let Some(completion) = use_completion {
        game.capture_cast_grant_completion(new_id, completion);
    }
    if let Some(completion) = price_completion {
        game.capture_cast_grant_completion(new_id, completion);
    }
    if let Some(spell) = game.object_mut(new_id) {
        spell.cast_play_from_constraints = play_from_constraints;
        spell.cast_grant_usage_identity = usage_identity.map(Box::new);
        spell.cast_price = price_receipt.map(Box::new);
        spell.cast_play_permission = exact_receipt.map(Box::new);
        if let Some(snapshot) = price_provider_snapshot {
            spell
                .cast_tagged_objects
                .insert("__cast_price_provider".into(), vec![snapshot]);
        }
    }
    if let Some(kind) = face_down_kind {
        // A peer that holds only a placeholder must later check that the
        // opened card really has this keyword (or is covered by the
        // permission it was cast through).
        game.record_hidden_face_down_cast_obligation(
            new_id,
            kind,
            cast_origin_snapshot
                .as_ref()
                .map_or(_from_zone, |snapshot| snapshot.zone),
            face_down_permission.as_ref(),
        );
    }
    if let Some(permission) = &face_down_permission {
        game.consume_face_down_cast_permission(
            permission.source,
            permission.player,
            permission.zone,
        );
    }
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
    if let Some(shared) = price_route
        .as_ref()
        .and_then(|route| route.price.shared_usage_id)
        && Some(shared) != shared_usage_to_consume
        && !game
            .effect_store
            .grant_registry
            .consume_shared_usage(shared)
    {
        return Err(GameLoopError::InvalidState(
            "Selected alternative price budget is unavailable".into(),
        ));
    }
    if let Some(snapshot) = cast_origin_snapshot {
        game.set_cast_origin_snapshot(new_id, snapshot);
    }
    let disturb_other_def = if selected_method
        .as_ref()
        .is_some_and(|method| method.casts_transformed())
    {
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
    let split_other_def = match casting_method.without_exact_permission() {
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
                        match casting_method.without_exact_permission() {
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
    let current_turn = game.turn.turn_number;
    game.stage_initial_controller_for_assembly(new_id, caster);
    if let Some(obj) = game.object_mut(new_id) {
        if let Some(method) = selected_method {
            obj.cast_alternative_method = Some(Box::new(method.clone()));
            crate::alternative_cast::ensure_alternative_battlefield_abilities(
                obj,
                &method,
                current_turn,
            );
            // CR 702.140a: Mutate is an alternative cost whose spell targets a
            // non-Human creature with the same owner.  Keep this requirement in
            // the ordinary resolution program so every casting path, legality
            // preview, retargeting effect, and resolution-time target check uses
            // the same target machinery as authored spell text.
            if method.is_mutate() {
                obj.begin_stack_program_overlay();
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

            if method.casts_transformed() {
                let other_def = disturb_other_def
                    .as_ref()
                    .expect("disturb linked face should be resolved before mutating the spell");
                // The spell has only its back face's characteristics, whose
                // color comes from its own color indicator (CR 712.8e, 204).
                obj.apply_definition_face(other_def);
                obj.cast_alternative_method = Some(Box::new(method.clone()));
            }

            if let crate::alternative_cast::AlternativeCastingMethod::Overload {
                ref effects, ..
            } = method
            {
                obj.begin_stack_program_overlay();
                obj.spell_effect = Some(
                    crate::resolution::ResolutionProgram::from_effects(effects.clone()).into(),
                );
            }
            if let crate::alternative_cast::AlternativeCastingMethod::Cleave {
                ref effects, ..
            } = method
            {
                obj.begin_stack_program_overlay();
                obj.spell_effect = Some(
                    crate::resolution::ResolutionProgram::from_effects(effects.clone()).into(),
                );
            }
            if let crate::alternative_cast::AlternativeCastingMethod::Awaken {
                ref effects, ..
            } = method
            {
                obj.begin_stack_program_overlay();
                obj.spell_effect = Some(
                    crate::resolution::ResolutionProgram::from_effects(effects.clone()).into(),
                );
            }
        }

        match casting_method.without_exact_permission() {
            CastingMethod::FaceDown | CastingMethod::FaceDownPlayFrom { .. } => {
                let disguise_ward =
                    face_down_kind == Some(crate::game_state::FaceDownCastKind::Disguise);
                obj.apply_face_down_cast_overlay_with_disguise_ward(disguise_ward);
                mark_face_down = true;
            }
            CastingMethod::SplitOtherHalf | CastingMethod::SplitOtherHalfPlayFrom { .. } => {
                let other_def = split_other_def
                    .as_ref()
                    .expect("split linked face should be resolved before mutating the spell");
                obj.apply_definition_face(other_def);
                if let CastingMethod::SplitOtherHalfPlayFrom { .. } = casting_method
                    && let Some(method) = selected_method_for_overlay.clone()
                {
                    if has_exact_permission {
                        if method.is_bestow() {
                            obj.apply_bestow_cast_overlay();
                        }
                        if let Some(power_toughness) = method.prototype_power_toughness()
                            && let Some(cost) = method.mana_cost()
                        {
                            obj.apply_prototype_cast_overlay(cost.clone(), power_toughness);
                        }
                    }
                    obj.cast_alternative_method = Some(Box::new(method));
                }
            }
            CastingMethod::Fuse => {
                let other_def = split_other_def
                    .as_ref()
                    .expect("fuse linked face should be resolved before mutating the spell");
                obj.apply_fused_split_spell_overlay(other_def);
            }
            _ => {}
        }

        if let Some(index) = price_route.as_ref().and_then(|route| route.prototype) {
            let method = obj.alternative_casts.get(index).ok_or_else(|| {
                GameLoopError::InvalidState("Selected prototype disappeared from this face".into())
            })?;
            let cost = method.mana_cost().cloned().ok_or_else(|| {
                GameLoopError::InvalidState("Selected prototype has no mana characteristic".into())
            })?;
            let power_toughness = method.prototype_power_toughness().ok_or_else(|| {
                GameLoopError::InvalidState("Selected price characteristic is not prototype".into())
            })?;
            obj.apply_prototype_cast_overlay(cost, power_toughness);
        }

        obj.ensure_aura_cast_spell_effect();

        // Initialize announcement metadata while the proposal object is
        // already mutably borrowed. Keeping this before the proposal's single
        // continuous-state refresh avoids immediately dirtying the freshly
        // rebuilt state in each caller, and keeps method-selection casts in
        // sync with direct casts.
        let mut optional_costs_paid = OptionalCostsPaid::from_costs(&obj.optional_costs);
        optional_costs_paid.cast_was_foretold = Some(cast_was_foretold);
        if price_route
            .as_ref()
            .is_some_and(|route| route.prototype.is_some())
        {
            optional_costs_paid.mark_label_paid(super::priority_cast::PROTOTYPE_CHOICE_LABEL);
        }
        if cast_during_main_phase {
            optional_costs_paid.record_main_phase_cast(caster);
        }
        if cast_at_sorcery_timing {
            optional_costs_paid.mark_cast_at_sorcery_timing();
        }
        obj.optional_costs_paid = optional_costs_paid;
    }

    if mark_face_down {
        game.set_face_down(new_id);
    }

    apply_play_from_cast_this_way_grants(
        game,
        new_id,
        caster,
        casting_method,
        selected_grant,
        selected_plain_grant,
    );
    for ability in permanent_riders {
        game.grant_incarnation_static_ability(new_id, ability);
    }

    if let Some(route) = price_route {
        for ability in route.price_riders {
            game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
                new_id,
                ability.id(),
                Some(ability),
            );
        }
    }

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
    game.refresh_continuous_state().map_err(|error| {
        GameLoopError::ExecutionFailed(crate::effects::ExecutionError::ContinuousDiscovery(error))
    })?;

    Ok(new_id)
}

fn apply_play_from_cast_this_way_grants(
    game: &mut GameState,
    stack_id: ObjectId,
    caster: PlayerId,
    casting_method: &CastingMethod,
    selected_grant: Option<crate::grant_registry::GrantedAlternativeCast>,
    selected_plain_grant: Option<crate::grant_registry::Grant>,
) {
    let (source_id, zone) = match casting_method.without_exact_permission() {
        CastingMethod::PlayFrom { source, zone, .. }
        | CastingMethod::SplitOtherHalfPlayFrom { source, zone, .. }
        | CastingMethod::FaceDownPlayFrom { source, zone } => (*source, *zone),
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
    // Both indexed and source-only proposals capture one concrete permission
    // before movement. Its constraints, usage and riders share this identity.
    let (permission_source, permission_zone, expected_zone, rider_filter, riders) =
        if let Some(selected) = selected_grant {
            (
                selected.source_id,
                selected.zone,
                crate::grant_registry::alternative_cast_grant_zone(zone),
                selected.cast_this_way_filter,
                selected.cast_this_way_grants,
            )
        } else if let Some(selected) = selected_plain_grant {
            (
                selected.source.source_id(),
                selected.zone,
                zone,
                selected.cast_this_way_filter,
                selected.cast_this_way_grants,
            )
        } else {
            return;
        };
    if permission_source != source_id
        || permission_zone != expected_zone
        || !rider_filter
            .as_ref()
            .is_none_or(|filter| filter.matches(&spell_as_cast, &ctx, game))
    {
        return;
    }
    for ability in riders {
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
///
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
    let method = match casting_method.without_exact_permission() {
        CastingMethod::AlternativePrice { .. } => {
            crate::alternative_cast::price_routes::origin_alternative(
                game,
                caster,
                obj,
                casting_method,
            )
        }
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
    let method = match casting_method.without_exact_permission() {
        CastingMethod::AlternativePrice { .. } => {
            crate::alternative_cast::price_routes::origin_alternative(
                game,
                caster,
                obj,
                casting_method,
            )
        }
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: Some(idx),
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
    let method = match casting_method.without_exact_permission() {
        CastingMethod::AlternativePrice { .. } => {
            crate::alternative_cast::price_routes::origin_alternative(
                game,
                caster,
                obj,
                casting_method,
            )
        }
        CastingMethod::Alternative(idx) => obj.alternative_casts.get(*idx).cloned(),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(game, caster, obj, *zone, *idx)
            .or_else(|| obj.cast_alternative_method_owned()),
        _ => None,
    }?;
    let reference =
        ironsmith_core::AlternativeCostReference::paid_marker(method.name(), method.mana_cost());
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
    let mut base_mana_cost = if mana_already_paid {
        None
    } else {
        game.object(spell_id).and_then(|obj| {
            crate::decision::spell_mana_cost_for_cast(game, caster, obj, &casting_method, from_zone)
        })
    };
    if base_mana_cost_waived && !mana_already_paid {
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
        if !game
            .try_pay_mana_cost_with_reason_and_dm(
                caster,
                Some(spell_id),
                &cost,
                x,
                crate::costs::PaymentReason::CastSpell,
                _decision_maker,
            )
            .map_err(GameLoopError::ExecutionFailed)?
        {
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
        spell_obj.caster_mana_spent_to_cast = Some(
            mana_spent_total.saturating_sub(
                assist_mana_spent_to_cast
                    .as_ref()
                    .map(|(_, spent)| spent.total())
                    .unwrap_or(0),
            ),
        );
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
    if game.object(new_id).is_some_and(|object| {
        object
            .cast_price
            .as_ref()
            .is_some_and(|price| price.prototype.is_some())
    }) {
        optional_costs_paid.mark_label_paid(super::priority_cast::PROTOTYPE_CHOICE_LABEL);
        if let Some(object) = game.object_mut(new_id) {
            object
                .optional_costs_paid
                .mark_label_paid(super::priority_cast::PROTOTYPE_CHOICE_LABEL);
        }
    }
    // Only the completed payment transaction authors a date; proposal and
    // unknown recovered receipts never acquire inferred payment evidence.
    let payment_turn = game.turn.turn_number;
    optional_costs_paid.record_completed_cast_payment(payment_turn);
    if let Some(spell_obj) = game.object_mut(new_id) {
        spell_obj
            .optional_costs_paid
            .record_completed_cast_payment(payment_turn);
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

    if let Some(identity) = game
        .object(new_id)
        .and_then(|spell| spell.cast_grant_usage_identity.as_deref())
        .cloned()
    {
        game.turn_store
            .grant_cast_uses_this_turn
            .insert((caster, identity));
    }

    if let Some(identity) = game
        .object(new_id)
        .and_then(|spell| spell.cast_price.as_deref())
        .map(|price| price.identity.clone())
    {
        game.turn_store
            .grant_cast_uses_this_turn
            .insert((caster, identity));
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
    game.refresh_continuous_state();
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

    // Cast-time trigger filters inspect the spell object itself. Preserve the
    // same tagged LKI there that the stack entry carries through resolution.
    if let Some(spell) = game.object_mut(new_id) {
        spell.cast_tagged_objects = stack_entry_tagged_objects.clone();
    }

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
    game.refresh_continuous_state();
    if let Some(spell_obj) = game.object(new_id).cloned() {
        game.turn_store
            .cast_spell_lki
            .insert(new_id, std::sync::Arc::new((spell_obj, entry.clone())));
    }
    game.push_to_stack(entry);
    game.complete_cast_grant(new_id);
    game.consume_next_cast_timing(caster, new_id);
    game.finish_library_top_announcement(crate::game_state::LibraryTopAnnouncement::Cast(new_id));

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
        crate::grant_registry::GrantRegistry::exhaust_next_matching_cast_grants(
            game, new_id, caster,
        );
    }
    queue_targeting_crime(game, trigger_queue, &targets, new_id, caster, provenance);

    if from_zone == Zone::Command {
        game.record_commander_cast_from_command_zone(new_id);
    }

    // CR: the creature stops being prepared as its prepare spell copy is cast.
    // The copy is already on the stack, so it survives losing the designation.
    if from_zone == Zone::Exile {
        game.unprepare_for_cast(spell_id);
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

    game.record_completed_cast_origin(new_id, from_zone);

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
                        Err(GameLoopError::ExecutionFailed(error))
                            if error.is_incomplete_execution() =>
                        {
                            // An unfinished engine calculation is not a
                            // rejected player action. The typed response owner
                            // has restored its attempt; retain it for recovery.
                            return Err(GameLoopError::ExecutionFailed(error));
                        }
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
                                state.pending_mana_parents.clear();
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
    let checkpoint = (game.clone(), trigger_queue.clone(), state.clone());
    let result =
        apply_decision_context_with_dm_inner(game, trigger_queue, state, ctx, decision_maker);
    if matches!(&result, Err(GameLoopError::ExecutionFailed(error)) if error.is_incomplete_execution())
    {
        game.restore_execution_checkpoint(checkpoint.0, false);
        *trigger_queue = checkpoint.1;
        *state = checkpoint.2;
    }
    result
}

fn apply_decision_context_with_dm_inner<D: DecisionMaker>(
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
            let response = decision_maker.decide_mana_payment(game, payment_ctx);
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
                    ActivationStage::ChoosingSacrifice
                        | ActivationStage::ChoosingCardCost
                        | ActivationStage::ChoosingCostReferences
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
            if options_ctx.exile_face_down_choice {
                let pending = state.pending_exile_face_down.as_ref().ok_or_else(|| {
                    GameLoopError::InvalidState("No face-down declaration owns this prompt".into())
                })?;
                if options_ctx.player != pending.player
                    || options_ctx.source != Some(pending.card_id)
                {
                    return Err(GameLoopError::InvalidState(
                        "Face-down declaration has a different source or player".into(),
                    ));
                }
                if decision_maker.awaiting_choice() {
                    return Ok(GameProgress::Continue);
                }
                if result.len() != 1 {
                    return Err(ResponseError::IllegalChoice(
                        "Choose one face-down declaration".into(),
                    )
                    .into());
                }
                return super::exile_face_down::apply_choice(
                    game,
                    trigger_queue,
                    state,
                    result[0],
                    decision_maker,
                );
            }
            if options_ctx.exile_play_choice {
                let pending = state.pending_exile_play.as_ref().ok_or_else(|| {
                    GameLoopError::InvalidState("No opened-card choice owns this prompt".into())
                })?;
                if options_ctx.player != pending.player
                    || options_ctx.source != Some(pending.card_id)
                {
                    return Err(GameLoopError::InvalidState(
                        "Opened-card prompt has a different source or player".into(),
                    ));
                }
                if decision_maker.awaiting_choice() {
                    return Ok(GameProgress::Continue);
                }
                if result.len() != 1 {
                    return Err(
                        ResponseError::IllegalChoice("Choose one opened-card play".into()).into(),
                    );
                }
                return super::exile_play::apply_exile_play_choice(
                    game,
                    trigger_queue,
                    state,
                    result[0],
                    decision_maker,
                );
            }
            if state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| pending.stage == CastStage::ChoosingCostResource)
            {
                let choice = result.first().copied().ok_or_else(|| {
                    GameLoopError::InvalidState("cost resource choice required".into())
                })?;
                return apply_cost_resource_response(
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
                .is_some_and(|pending| pending.stage == CastStage::ChoosingCreatureType)
            {
                let choice = result.first().copied().ok_or_else(|| {
                    GameLoopError::InvalidState("Creature type selection requires one type".into())
                })?;
                return apply_creature_type_announcement_response(
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
                .is_some_and(|pending| pending.stage == CastStage::ChoosingModeChooser)
            {
                let Some(choice) = result.first().copied() else {
                    return Err(GameLoopError::InvalidState(
                        "mode chooser selection requires one player".to_string(),
                    ));
                };
                return apply_mode_chooser_response(
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
            state.mandatory_loop.observe_priority_snapshot(game);
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
                        .observe_resolution(resolved_signature, queued_signatures)?
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
        pctx.analysis_complete
            && pctx.actions.len() == 1
            && matches!(pctx.actions[0], LegalAction::PassPriority)
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
mod exact_play_permission_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod replacement_owner_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    struct PausePayload {
        pause: bool,
        pending: bool,
        questions: usize,
    }
    impl DecisionMaker for PausePayload {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.questions += 1;
            if self.pause {
                self.pending = true;
                false
            } else {
                true
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn check_planned_activation_error_transport(illegal: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;
        let definition =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Planned source")
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                    vec![crate::mana::ManaSymbol::Green],
                ))
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let request = crate::mana_payment::ManaPaymentRequest::new(
            alice,
            source,
            crate::costs::PaymentReason::Effect,
            crate::mana::ManaCost::new().add_generic(1),
        );
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        assert_eq!(plan.mana_ability_steps.len(), 1);
        let mut payment = crate::mana_payment::PendingManaPayment::new(request, plan);
        // Exercise the activation adapter's own response to a changed live
        // board; authoritative plan revalidation has separate owning tests.
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                    ObjectFilter::default(),
                ),
                ReplacementAction::Instead(vec![Effect::gain_life(2), Effect::lose_life(Value::X)]),
            ),
        );
        if illegal {
            game.tap(source);
        }
        game.take_pending_trigger_events();
        game.queue_trigger_event(
            ProvNodeId::default(),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 1),
                ProvNodeId::default(),
            ),
        );
        let mut queue = TriggerQueue::new();
        let before_queue = format!("{queue:?}");
        let before_payment = format!("{payment:?}");
        let before_objects = game.object_ids_in_deterministic_order();
        let next_id = game.next_object_id_counter();
        let mut undo_locked = false;
        let mut dm = PausePayload {
            pause: false,
            pending: false,
            questions: 0,
        };
        let result = execute_planned_mana_activations(
            &mut game,
            &mut queue,
            alice,
            &mut payment,
            &mut undo_locked,
            &mut dm,
        );
        if illegal {
            assert!(
                matches!(result, Err(GameLoopError::InvalidState(_))),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(
                    result,
                    Err(GameLoopError::ExecutionFailed(
                        crate::effects::ExecutionError::UnresolvableValue(_)
                    ))
                ),
                "planned adapter must preserve executable replacement failure: {result:?}"
            );
        }
        assert_eq!(format!("{payment:?}"), before_payment);
        assert_eq!(format!("{queue:?}"), before_queue);
        assert!(!undo_locked);
        assert_eq!(game.is_tapped(source), illegal);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object_ids_in_deterministic_order(), before_objects);
        assert_eq!(game.next_object_id_counter(), next_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap()
                .amount,
            1
        );
    }
    #[test]
    fn planned_activation_preserves_replacement_execution_error() {
        check_planned_activation_error_transport(false);
    }
    #[test]
    fn planned_activation_retains_illegal_action_error() {
        check_planned_activation_error_transport(true);
    }

    fn perform_probe(
        game: &mut GameState,
        queue: &mut TriggerQueue,
        state: &mut PriorityLoopState,
        source: ObjectId,
        paid: bool,
        dm: &mut PausePayload,
    ) -> Result<(), GameLoopError> {
        let alice = PlayerId::from_index(0);
        let activation_origin = game.current_characteristics(source).and_then(|chars| chars.abilities.origin(0).cloned());
        if paid {
            execute_pending_mana_ability(
                game,
                queue,
                &PendingManaAbility {
                    activation_origin,
                    linked_exile_owner: None,
                    source_number_owner: None,
                    payment_reason: crate::costs::PaymentReason::ActivateManaAbility,
                    source,
                    ability_index: 0,
                    activator: alice,
                    provenance: ProvNodeId::default(),
                    mana_cost: crate::mana::ManaCost::new().add_generic(1),
                    other_costs: vec![crate::costs::Cost::tap()],
                    mana_to_add: vec![crate::mana::ManaSymbol::Green],
                    effects: Default::default(),
                    mana_usage_restrictions: vec![],
                    mana_source_chosen_creature_type: None,
                    mana_production_provenance:
                        crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
                    undo_locked_by_mana: false,
                    pending_mana_payment: None,
                    exhaust_announcement: None,
                    x_value: None,
                },
                dm,
            )
        } else {
            apply_priority_response_with_dm(
                game,
                queue,
                state,
                &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
                    source,
                    ability_index: 0,
                }),
                dm,
            )
            .map(|_| ())
        }
    }
    fn check_priority_mana_owner(paid: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Priority mana probe",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(crate::ability::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![crate::mana::ManaSymbol::Green],
        ))
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if paid {
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(crate::mana::ManaSymbol::Colorless, 1);
        }
        let mut effects = vec![Effect::gain_life(2)];
        if mode == 1 {
            effects.push(Effect::lose_life(Value::X));
        }
        if mode == 2 {
            effects.push(Effect::may(vec![Effect::gain_life(4)]));
        }
        effects.push(Effect::gain_life(8));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                    ObjectFilter::default(),
                ),
                ReplacementAction::Instead(effects),
            ),
        );
        game.take_pending_trigger_events();
        // Existing unpublished observations must survive a failed/suspended owner.
        game.queue_trigger_event(
            ProvNodeId::default(),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 1),
                ProvNodeId::default(),
            ),
        );
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let before_state = format!("{state:?}");
        let before_queue = format!("{queue:?}");
        let before_id = game.next_object_id_counter();
        let before_objects = game.objects_in_deterministic_order().len();
        let mut dm = PausePayload {
            pause: mode == 2,
            pending: false,
            questions: 0,
        };
        let result = perform_probe(&mut game, &mut queue, &mut state, source, paid, &mut dm);
        if mode == 1 {
            assert!(
                matches!(
                    result,
                    Err(GameLoopError::ExecutionFailed(
                        crate::effects::ExecutionError::UnresolvableValue(_)
                    ))
                ),
                "mana replacement error remains typed"
            );
        } else {
            result.expect("valid replacement succeeds or suspends");
            if mode == 2 {
                assert!(dm.awaiting_choice(), "pending payload exposes its decision");
            } else {
                assert_eq!(game.player(bob).unwrap().life, 30);
                assert!(game.is_tapped(source));
            }
        }
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.green, 0);
        assert_eq!(game.objects_in_deterministic_order().len(), before_objects);
        assert_eq!(game.next_object_id_counter(), before_id);
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some(),
            mode != 0
        );
        if mode != 0 {
            assert!(!game.is_tapped(source));
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(
                game.player(alice).unwrap().mana_pool.colorless,
                if paid { 1 } else { 0 }
            );
            assert_eq!(format!("{state:?}"), before_state);
            assert_eq!(format!("{queue:?}"), before_queue);
            let events = game.take_pending_trigger_events();
            assert_eq!(events.len(), 1);
            let sentinel = events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap();
            assert_eq!(sentinel.player, alice);
            assert_eq!(sentinel.amount, 1);
        } else {
            assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
        }
        if mode == 2 {
            assert_eq!(dm.questions, 1);
            let mut replay = PausePayload {
                pause: false,
                pending: false,
                questions: 0,
            };
            perform_probe(&mut game, &mut queue, &mut state, source, paid, &mut replay).unwrap();
            assert_eq!(replay.questions, 1);
            assert!(!replay.awaiting_choice());
            assert!(game.is_tapped(source));
            assert_eq!(game.player(bob).unwrap().life, 34);
            assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
            assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
    }
    #[test]
    fn immediate_mana_replacement_uses_payload_controller() {
        check_priority_mana_owner(false, 0);
    }
    #[test]
    fn immediate_mana_replacement_error_restores_owner() {
        check_priority_mana_owner(false, 1);
    }
    #[test]
    fn immediate_mana_replacement_pending_restores_then_replays() {
        check_priority_mana_owner(false, 2);
    }
    #[test]
    fn paid_mana_replacement_uses_payload_controller() {
        check_priority_mana_owner(true, 0);
    }
    #[test]
    fn paid_mana_replacement_error_restores_costs_and_queue() {
        check_priority_mana_owner(true, 1);
    }
    #[test]
    fn paid_mana_replacement_pending_restores_then_replays() {
        check_priority_mana_owner(true, 2);
    }

    fn check_public_payment_confirmation(pending: bool, cost_replacement: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Paid priority owner probe",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(crate::ability::Ability::mana(
            crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::mana(crate::mana::ManaCost::new().add_generic(1)),
                crate::costs::Cost::tap(),
                crate::costs::Cost::life(if cost_replacement { 2 } else { 0 }),
            ]),
            vec![crate::mana::ManaSymbol::Green],
        ))
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(crate::mana::ManaSymbol::Colorless, 1);
        let mut effects = vec![Effect::gain_life(2)];
        if pending {
            effects.push(Effect::may(vec![Effect::gain_life(4)]));
        } else {
            effects.push(Effect::lose_life(Value::X));
        }
        effects.push(Effect::gain_life(8));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                    ObjectFilter::default(),
                ),
                ReplacementAction::Instead(effects.clone()),
            ),
        );
        let shield = if cost_replacement {
            game.effect_store.replacement_effects.remove_effect(shield);
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::life::matchers::WouldLoseLifeMatcher::you(),
                    ReplacementAction::Instead(effects),
                ),
            )
        } else {
            shield
        };
        game.take_pending_trigger_events();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut queue = TriggerQueue::new();
        let mut initial = PausePayload {
            pause: false,
            pending: false,
            questions: 0,
        };
        let prompt = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
                source,
                ability_index: 0,
            }),
            &mut initial,
        )
        .unwrap();
        assert!(matches!(
            prompt,
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::ManaPayment(_)
            )
        ));
        let payment = state
            .pending_mana_ability
            .as_ref()
            .unwrap()
            .pending_mana_payment
            .as_ref()
            .unwrap();
        assert!(payment.plan.payable);
        let response =
            PriorityResponse::ManaPaymentPlan(crate::mana_payment::ManaPaymentResponse::Confirm {
                plan_id: payment.plan.id,
                request_hash: payment.plan.request_hash,
            });
        let before_state = format!("{state:?}");
        let before_queue = format!("{queue:?}");
        let before_id = game.next_object_id_counter();
        game.queue_trigger_event(
            ProvNodeId::default(),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 1),
                ProvNodeId::default(),
            ),
        );
        let mut dm = PausePayload {
            pause: pending,
            pending: false,
            questions: 0,
        };
        let result =
            apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &response, &mut dm);
        if pending {
            result.unwrap();
            assert!(dm.awaiting_choice());
            assert_eq!(dm.questions, 1);
        } else {
            assert!(matches!(
                result,
                Err(GameLoopError::ExecutionFailed(
                    crate::effects::ExecutionError::UnresolvableValue(_)
                ))
            ));
        }
        assert_eq!(
            format!("{state:?}"),
            before_state,
            "confirmation must retain authoritative payment for replay"
        );
        assert_eq!(format!("{queue:?}"), before_queue);
        assert!(!game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert_eq!(game.next_object_id_counter(), before_id);
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap()
                .amount,
            1
        );
        if pending {
            let mut replay = PausePayload {
                pause: false,
                pending: false,
                questions: 0,
            };
            apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &response,
                &mut replay,
            )
            .unwrap();
            assert_eq!(replay.questions, 1);
            assert!(!replay.awaiting_choice());
            assert!(state.pending_mana_ability.is_none());
            assert!(game.is_tapped(source));
            assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
            assert_eq!(
                game.player(alice).unwrap().mana_pool.green,
                if cost_replacement { 1 } else { 0 }
            );
            assert_eq!(
                game.player(if cost_replacement { alice } else { bob })
                    .unwrap()
                    .life,
                34
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
    }
    #[test]
    fn public_mana_confirmation_error_retains_payment_and_restores_costs() {
        check_public_payment_confirmation(false, false);
    }
    #[test]
    fn public_mana_confirmation_pending_retains_payment_and_replays_once() {
        check_public_payment_confirmation(true, false);
    }
    fn check_cost_adapter_failure(adapter: u8, pause: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Cost adapter probe",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if adapter == 0 {
            game.tap(source);
        }
        let snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut payload = vec![Effect::gain_life(2)];
        if pause {
            payload.push(Effect::may(vec![Effect::gain_life(4)]));
        } else {
            payload.push(Effect::lose_life(Value::X));
        }
        payload.push(Effect::gain_life(8));
        let shield = if adapter == 0 {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    bob,
                    crate::events::life::matchers::WouldLoseLifeMatcher::any_player(),
                    ReplacementAction::Instead(payload),
                ),
            )
        } else {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    bob,
                    crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                        ObjectFilter::specific(source),
                    ),
                    ReplacementAction::Instead(payload),
                ),
            )
        };
        game.take_pending_trigger_events();
        game.queue_trigger_event(
            ProvNodeId::default(),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 1),
                ProvNodeId::default(),
            ),
        );
        let mut queue = TriggerQueue::new();
        let before_queue = format!("{queue:?}");
        let mut tags = std::collections::HashMap::new();
        tags.insert(
            crate::tag::TagKey::from("prior_cost"),
            vec![snapshot.clone()],
        );
        let mut cast = PendingCast::new(
            source,
            Zone::Hand,
            alice,
            ProvNodeId::default(),
            CastStage::ChoosingNextCost,
            None,
            vec![],
            CastingMethod::Normal,
            Default::default(),
            None,
            source,
        );
        cast.remaining_cost_steps = vec![
            ActivationCostStep::Cost(crate::costs::Cost::tap()),
            ActivationCostStep::Cost(crate::costs::Cost::untap()),
        ];
        let mut activation = PendingActivation::new(
            source,
            0,
            None,
            alice,
            ProvNodeId::default(),
            ActivationStage::ChoosingNextCost,
            Default::default(),
            vec![],
            None,
            vec![],
            crate::costs::PaymentReason::ActivateAbility,
            vec![],
            vec![
                ActivationCostStep::Cost(crate::costs::Cost::tap()),
                ActivationCostStep::Cost(crate::costs::Cost::untap()),
            ],
            Default::default(),
            0,
            false,
            false,
            snapshot.stable_id,
            snapshot,
            "Cost adapter probe".into(),
            None,
            false,
            false,
            vec![],
            None,
            vec![],
        );
        let before_cast = format!("{cast:?}");
        let before_activation = format!("{activation:?}");
        let before_tags = format!("{tags:?}");
        let mut dm = PausePayload {
            pause,
            pending: false,
            questions: 0,
        };
        let run =
            |game: &mut GameState,
             queue: &mut TriggerQueue,
             cast: &mut PendingCast,
             activation: &mut PendingActivation,
             tags: &mut std::collections::HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
             dm: &mut PausePayload|
             -> Result<(), GameLoopError> {
                match adapter {
                    0 => pay_selected_cost(
                        game,
                        &crate::costs::Cost::life(2),
                        source,
                        alice,
                        crate::costs::PaymentReason::ActivateAbility,
                        ProvNodeId::default(),
                        source,
                        None,
                        tags,
                        &mut activation.effect_outcomes,
                        dm,
                    ).map(|_| ()),
                    1 => super::super::priority_cast::auto_pay_spell_tap_cost_steps(
                        game, queue, cast, dm,
                    ),
                    _ => super::super::priority_cast::auto_pay_activation_tap_cost_steps(
                        game, queue, activation, dm,
                    ),
                }
            };
        let result = run(
            &mut game,
            &mut queue,
            &mut cast,
            &mut activation,
            &mut tags,
            &mut dm,
        );
        if pause {
            result.unwrap();
            assert!(dm.awaiting_choice());
            assert_eq!(dm.questions, 1);
        } else {
            assert!(
                matches!(
                    result,
                    Err(GameLoopError::ExecutionFailed(
                        crate::effects::ExecutionError::UnresolvableValue(_)
                    ))
                ),
                "adapter {adapter} must preserve executable replacement failure: {result:?}"
            );
        }
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(
            game.is_tapped(source),
            adapter == 0,
            "automatic payment must undo its completed tap prefix on error or suspension"
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert_eq!(format!("{queue:?}"), before_queue);
        assert_eq!(format!("{tags:?}"), before_tags);
        assert_eq!(
            format!("{cast:?}"),
            before_cast,
            "failed/suspended spell payment must retain cost steps"
        );
        assert_eq!(
            format!("{activation:?}"),
            before_activation,
            "failed/suspended activation must retain cost steps"
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap()
                .amount,
            1
        );
        if pause {
            let mut replay = PausePayload {
                pause: false,
                pending: false,
                questions: 0,
            };
            run(
                &mut game,
                &mut queue,
                &mut cast,
                &mut activation,
                &mut tags,
                &mut replay,
            )
            .unwrap();
            assert_eq!(replay.questions, 1);
            assert_eq!(game.player(bob).unwrap().life, 34);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
            if adapter == 1 {
                assert!(cast.remaining_cost_steps.is_empty());
            }
            if adapter == 2 {
                assert!(activation.remaining_cost_steps.is_empty());
            }
        }
    }
    fn check_public_untap_cost_owner(pause: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Untap cost owner probe",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(crate::ability::Ability::activated(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::untap()),
            vec![Effect::gain_life(1)],
        ))
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.tap(source);
        let mut payload = vec![Effect::gain_life(2)];
        if pause {
            payload.push(Effect::may(vec![Effect::gain_life(4)]));
        } else {
            payload.push(Effect::lose_life(Value::X));
        }
        payload.push(Effect::gain_life(8));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::permanents::matchers::WouldBecomeUntappedMatcher::new(
                    ObjectFilter::specific(source),
                ),
                ReplacementAction::Instead(payload),
            ),
        );
        game.take_pending_trigger_events();
        game.queue_trigger_event(
            ProvNodeId::default(),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 1),
                ProvNodeId::default(),
            ),
        );
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let before_state = format!("{state:?}");
        let before_queue = format!("{queue:?}");
        let response = PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
            source,
            ability_index: 0,
        });
        let mut dm = PausePayload {
            pause,
            pending: false,
            questions: 0,
        };
        let result =
            apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &response, &mut dm);
        if pause {
            result.unwrap();
            assert!(dm.awaiting_choice());
            assert_eq!(dm.questions, 1);
        } else {
            assert!(
                matches!(
                    result,
                    Err(GameLoopError::ExecutionFailed(
                        crate::effects::ExecutionError::UnresolvableValue(_)
                    ))
                ),
                "activation must preserve replacement failure: {result:?}"
            );
        }
        assert!(
            game.stack_is_empty(),
            "an unanswered or failed cost cannot announce an ability"
        );
        assert_eq!(game.ability_activation_count_this_turn(source, 0), 0);
        assert_eq!(
            format!("{state:?}"),
            before_state,
            "root activation bookkeeping must roll back"
        );
        assert_eq!(format!("{queue:?}"), before_queue);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap()
                .amount,
            1
        );
        if pause {
            let mut replay = PausePayload {
                pause: false,
                pending: false,
                questions: 0,
            };
            apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &response,
                &mut replay,
            )
            .unwrap();
            assert_eq!(replay.questions, 1);
            assert_eq!(game.player(bob).unwrap().life, 34);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.ability_activation_count_this_turn(source, 0), 1);
            assert_eq!(game.stack.len(), 1);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
    }
    #[test]
    fn public_untap_cost_error_restores_activation_owner() {
        check_public_untap_cost_owner(false);
    }
    #[test]
    fn public_untap_cost_pause_does_not_announce_and_replays_once() {
        check_public_untap_cost_owner(true);
    }

    #[test]
    fn selected_cost_replacement_error_preserves_type_and_resources() {
        check_cost_adapter_failure(0, false);
    }
    #[test]
    fn selected_cost_replacement_pause_replays_once() {
        check_cost_adapter_failure(0, true);
    }
    #[test]
    fn auto_spell_cost_replacement_error_preserves_type_and_steps() {
        check_cost_adapter_failure(1, false);
    }
    #[test]
    fn auto_spell_cost_replacement_pause_retains_steps_and_replays_once() {
        check_cost_adapter_failure(1, true);
    }
    #[test]
    fn auto_activation_cost_replacement_error_preserves_type_and_steps() {
        check_cost_adapter_failure(2, false);
    }
    #[test]
    fn auto_activation_cost_replacement_pause_retains_steps_and_replays_once() {
        check_cost_adapter_failure(2, true);
    }

    fn check_public_cost_menu_invalid_response(alternative: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let fodder = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Menu cost resource",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .build();
        let chosen = game.create_object_from_definition(&fodder, alice, Zone::Battlefield);
        let hand_resource =
            (!alternative).then(|| game.create_object_from_definition(&fodder, alice, Zone::Hand));
        let cost = if alternative {
            crate::cost::TotalCost::one_of(vec![
                crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
                crate::cost::TotalCost::from_cost(crate::costs::Cost::life(2)),
            ])
        } else {
            crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::sacrifice(crate::filter::ObjectFilter::specific(chosen)),
                crate::costs::Cost::discard(1, None),
            ])
        };
        let definition =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Menu cost owner")
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(crate::ability::Ability::activated(
                    cost,
                    vec![Effect::gain_life(1)],
                ))
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateAbility {
                source,
                ability_index: 0,
            }),
            &mut dm,
        )
        .unwrap();
        let GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(menu),
        ) = progress
        else {
            panic!("expected cost menu, got {progress:?}");
        };
        let before_state = format!("{state:?}");
        let before_queue = format!("{queue:?}");
        let result = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::NextCostChoice(usize::MAX),
            &mut dm,
        );
        assert!(
            matches!(result, Err(GameLoopError::InvalidState(_))),
            "{result:?}"
        );
        assert_eq!(
            format!("{state:?}"),
            before_state,
            "invalid menu choice cannot orphan payment"
        );
        assert_eq!(format!("{queue:?}"), before_queue);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.stack_is_empty());
        assert_eq!(game.object(chosen).unwrap().zone, Zone::Battlefield);
        let choice = if alternative {
            0
        } else {
            menu.options
                .iter()
                .find(|option| {
                    option
                        .description
                        .to_ascii_lowercase()
                        .contains("sacrifice")
                })
                .expect("menu offers sacrifice")
                .index
        };
        apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::NextCostChoice(choice),
            &mut dm,
        )
        .unwrap();
        if !alternative {
            apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::SacrificeTarget(chosen),
                &mut dm,
            )
            .unwrap();
            apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::CardCostChoice(hand_resource.unwrap()),
                &mut dm,
            )
            .unwrap();
            assert!(!game.battlefield.contains(&chosen));
            assert!(
                !game
                    .player(alice)
                    .unwrap()
                    .hand
                    .contains(&hand_resource.unwrap())
            );
        }
        assert!(state.pending_activation.is_none());
        assert_eq!(
            game.stack.len(),
            1,
            "valid response after rejection announces once"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if alternative { 19 } else { 20 }
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    }
    #[test]
    fn public_cost_menu_invalid_next_step_retains_continuation() {
        check_public_cost_menu_invalid_response(false);
    }
    #[test]
    fn public_cost_menu_invalid_alternative_retains_continuation() {
        check_public_cost_menu_invalid_response(true);
    }

    fn check_public_selected_cost_owner(owner: u8, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;
        let card_choice = owner % 2 == 1;
        let spell = owner >= 2;
        let selected_zone = if card_choice {
            Zone::Hand
        } else {
            Zone::Battlefield
        };
        let fodder = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Selected cost resource",
        )
        .card_types(vec![crate::types::CardType::Creature])
        .build();
        let chosen = game.create_object_from_definition(&fodder, alice, selected_zone);
        let selected_cost = if card_choice {
            crate::costs::Cost::discard(1, None)
        } else {
            crate::costs::Cost::sacrifice(crate::filter::ObjectFilter::specific(chosen))
        };
        let costs =
            crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::life(1), selected_cost]);
        let mut builder = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Selected cost owner",
        )
        .card_types(vec![if spell {
            crate::types::CardType::Sorcery
        } else {
            crate::types::CardType::Artifact
        }]);
        if spell {
            builder = builder.mana_cost(crate::mana::ManaCost::new());
        } else {
            builder = builder.with_ability(crate::ability::Ability::activated(
                costs.clone(),
                vec![Effect::gain_life(1)],
            ));
        }
        let mut definition = builder.build();
        if spell {
            definition.additional_cost = costs;
            definition.spell_effect = Some(vec![Effect::gain_life(1)].into());
        }
        let source = game.create_object_from_definition(
            &definition,
            alice,
            if spell { Zone::Hand } else { Zone::Battlefield },
        );
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut initial = crate::decision::SelectFirstDecisionMaker;
        let action = if spell {
            compute_legal_actions(&game, alice).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).expect("cost spell is legal")
        } else {
            LegalAction::ActivateAbility {
                source,
                ability_index: 0,
            }
        };
        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut initial,
        )
        .unwrap();
        assert!(
            matches!(
                progress,
                GameProgress::NeedsDecisionCtx(
                    crate::decisions::context::DecisionContext::SelectObjects(_)
                )
            ),
            "must reach actual selected-cost prompt: {progress:?}"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            19,
            "the earlier automatic life cost has committed"
        );
        let mut payload = vec![Effect::gain_life(2)];
        if mode == 1 {
            payload.push(Effect::may(vec![Effect::gain_life(4)]));
        } else {
            payload.push(Effect::lose_life(Value::X));
        }
        payload.push(Effect::gain_life(8));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                bob,
                crate::events::zones::matchers::WouldGoToGraveyardMatcher::new(
                    crate::filter::ObjectFilter::specific(chosen),
                ),
                ReplacementAction::Instead(payload),
            ),
        );
        game.take_pending_trigger_events();
        game.queue_trigger_event(
            ProvNodeId::default(),
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::LifeGainEvent::new(alice, 1),
                ProvNodeId::default(),
            ),
        );
        let before_state = format!("{state:?}");
        let before_queue = format!("{queue:?}");
        let before_stack = game.stack.len();
        let before_id = game.next_object_id_counter();
        let target = if mode == 2 {
            ObjectId::from_raw(u64::MAX - 100)
        } else {
            chosen
        };
        let response = if card_choice || spell {
            PriorityResponse::CardCostChoice(target)
        } else {
            PriorityResponse::SacrificeTarget(target)
        };
        let mut dm = PausePayload {
            pause: mode == 1,
            pending: false,
            questions: 0,
        };
        let result =
            apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &response, &mut dm);
        if mode == 1 {
            result.unwrap();
            assert!(dm.awaiting_choice());
            assert_eq!(dm.questions, 1);
        } else if mode == 2 {
            assert!(
                matches!(result, Err(GameLoopError::InvalidState(_))),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(
                    result,
                    Err(GameLoopError::ExecutionFailed(
                        crate::effects::ExecutionError::UnresolvableValue(_)
                    ))
                ),
                "{result:?}"
            );
        }
        assert_eq!(
            format!("{state:?}"),
            before_state,
            "selected-cost response must retain its authoritative continuation"
        );
        assert_eq!(format!("{queue:?}"), before_queue);
        assert_eq!(
            game.player(alice).unwrap().life,
            19,
            "earlier costs survive rollback of this response"
        );
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.object(chosen).unwrap().zone, selected_zone);
        assert_eq!(
            game.stack.len(),
            before_stack,
            "no announcement while selected replacement is incomplete"
        );
        assert_eq!(game.next_object_id_counter(), before_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .unwrap()
                .amount,
            1
        );
        if mode == 1 {
            let mut replay = PausePayload {
                pause: false,
                pending: false,
                questions: 0,
            };
            apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &response,
                &mut replay,
            )
            .unwrap();
            assert_eq!(replay.questions, 1);
            assert_eq!(game.player(alice).unwrap().life, 19);
            assert_eq!(game.player(bob).unwrap().life, 34);
            assert_eq!(game.stack.len(), before_stack + 1);
            assert!(state.pending_activation.is_none());
            assert!(state.pending_cast.is_none());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
    }
    #[test]
    fn public_selected_cost_error_retains_all_four_owners() {
        for owner in 0..4 {
            check_public_selected_cost_owner(owner, 0);
        }
    }
    #[test]
    fn public_selected_cost_pause_replays_all_four_owners_once() {
        for owner in 0..4 {
            check_public_selected_cost_owner(owner, 1);
        }
    }
    #[test]
    fn public_selected_cost_invalid_choice_retains_all_four_owners() {
        for owner in 0..4 {
            check_public_selected_cost_owner(owner, 2);
        }
    }

    fn check_nested_preselected_obligations(mode: u8) {
        use crate::mana::{ManaCost, ManaSymbol};
        use crate::mana_payment::{ManaPaymentResponse, RequiredManaActivation};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.turn.priority_player = Some(alice);
        game.turn.active_player = alice;
        let mut builder = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Selected producer",
        )
        .card_types(vec![crate::types::CardType::Artifact]);
        if mode == 2 {
            builder = builder.with_ability(crate::ability::Ability::activated(
                crate::cost::TotalCost::free(),
                vec![Effect::add_mana_of_any_color(1)],
            ));
        } else {
            let free_mana = |symbol| crate::ability::Ability {
                kind: crate::ability::AbilityKind::Activated(
                    crate::ability::ActivatedAbility::mana_with_costs(
                        crate::cost::TotalCost::free(),
                        vec![],
                        vec![symbol],
                    ),
                ),
                functional_zones: vec![Zone::Battlefield],
            };
            builder = builder
                .with_ability(free_mana(ManaSymbol::Blue))
                .with_ability(free_mana(ManaSymbol::Red));
        }
        let producer =
            game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
        let filter = |name: &str, price, output| {
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(crate::ability::Ability::mana(
                    crate::cost::TotalCost::from_costs(vec![
                        crate::costs::Cost::mana(ManaCost::new().add_generic(price)),
                        crate::costs::Cost::tap(),
                    ]),
                    output,
                ))
                .build()
        };
        let parent = game.create_object_from_definition(
            &filter("Parent filter", 2, vec![ManaSymbol::Green]),
            alice,
            Zone::Battlefield,
        );
        let child = game.create_object_from_definition(
            &filter(
                "Child filter",
                1,
                vec![ManaSymbol::Colorless, ManaSymbol::Colorless],
            ),
            alice,
            Zone::Battlefield,
        );
        let first = RequiredManaActivation {
            source: producer,
            ability_index: 0,
            color_restriction: (mode == 2).then_some(vec![crate::color::Color::Blue]),
        };
        let second = RequiredManaActivation {
            source: producer,
            ability_index: usize::from(mode == 1),
            color_restriction: (mode == 2).then_some(vec![crate::color::Color::Red]),
        };
        let child_selection = if mode == 2 {
            second.clone()
        } else {
            first.clone()
        };
        let expected_remaining = if mode == 2 {
            first.clone()
        } else {
            second.clone()
        };
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
                source: parent,
                ability_index: 0,
            }),
            &mut dm,
        )
        .unwrap();
        assert!(matches!(
            progress,
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::ManaPayment(_)
            )
        ));
        let mut preferences = state
            .pending_mana_ability
            .as_ref()
            .unwrap()
            .pending_mana_payment
            .as_ref()
            .unwrap()
            .request
            .preferences
            .clone();
        preferences.required_sources.push(producer);
        preferences.required_activations = vec![first, second];
        if mode == 3 {
            preferences.required_sources.push(child);
            preferences
                .required_activations
                .push(RequiredManaActivation {
                    source: child,
                    ability_index: 0,
                    color_restriction: None,
                });
        }
        apply_mana_payment_plan_response(
            &mut game,
            &mut queue,
            &mut state,
            &ManaPaymentResponse::Replan { preferences },
            &mut dm,
        )
        .unwrap();
        apply_mana_payment_plan_response(
            &mut game,
            &mut queue,
            &mut state,
            &ManaPaymentResponse::Activate {
                source: child,
                ability_index: 0,
            },
            &mut dm,
        )
        .unwrap();
        let mut preferences = state
            .pending_mana_ability
            .as_ref()
            .unwrap()
            .pending_mana_payment
            .as_ref()
            .unwrap()
            .request
            .preferences
            .clone();
        preferences.required_activations = vec![child_selection.clone()];
        apply_mana_payment_plan_response(
            &mut game,
            &mut queue,
            &mut state,
            &ManaPaymentResponse::Replan { preferences },
            &mut dm,
        )
        .unwrap();
        let payment = state
            .pending_mana_ability
            .as_ref()
            .unwrap()
            .pending_mana_payment
            .as_ref()
            .unwrap();
        assert!(
            payment.plan.payable,
            "mode={mode}, actual payment={payment:?}"
        );
        assert_eq!(payment.plan.mana_ability_steps.len(), 1);
        assert_eq!(payment.plan.mana_ability_steps[0].source, producer);
        assert_eq!(
            payment.plan.mana_ability_steps[0].ability_index,
            child_selection.ability_index
        );
        assert_eq!(
            payment.plan.mana_ability_steps[0].color_restriction,
            child_selection.color_restriction
        );
        let confirm = ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        };
        apply_mana_payment_plan_response(&mut game, &mut queue, &mut state, &confirm, &mut dm)
            .unwrap();
        assert!(game.is_tapped(child));
        assert!(!game.is_tapped(parent));
        assert_eq!(
            game.ability_activation_count_this_turn(producer, child_selection.ability_index),
            1,
            "child must actually execute exactly one selected producer activation"
        );
        let resumed = state.pending_mana_ability.as_ref().unwrap();
        assert_eq!(resumed.source, parent);
        let payment = resumed.pending_mana_payment.as_ref().unwrap();
        assert!(
            payment.request.preferences.required_sources.is_empty(),
            "completed child discharged the source-use obligation"
        );
        assert_eq!(
            payment.request.preferences.required_activations,
            vec![expected_remaining.clone()],
            "only the committed exact occurrence is discharged; retain repetition, other ability or other color"
        );
        assert!(
            payment.plan.payable,
            "mode={mode}, actual payment={payment:?}, inventory={:?}, producer legal={:?}, tapped={}",
            crate::mana_payment::mana_payment_activation_inventory(&game, &payment.request),
            crate::special_actions::can_activate_mana_ability_check(
                &game,
                alice,
                producer,
                expected_remaining.ability_index
            ),
            game.is_tapped(producer)
        );
        assert_eq!(payment.plan.mana_ability_steps.len(), 1);
        assert_eq!(
            payment.plan.mana_ability_steps[0].ability_index,
            expected_remaining.ability_index
        );
        assert_eq!(
            payment.plan.mana_ability_steps[0].color_restriction,
            expected_remaining.color_restriction
        );
        let confirm = ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        };
        apply_mana_payment_plan_response(&mut game, &mut queue, &mut state, &confirm, &mut dm)
            .unwrap();
        assert!(state.pending_mana_ability.is_none());
        assert!(state.pending_mana_parents.is_empty());
        assert!(game.is_tapped(parent));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 2);
        assert_eq!(
            game.ability_activation_count_this_turn(producer, 0),
            if mode == 1 { 1 } else { 2 }
        );
        if mode == 1 {
            assert_eq!(game.ability_activation_count_this_turn(producer, 1), 1);
        }
        assert_eq!(game.ability_activation_count_this_turn(child, 0), 1);
        assert_eq!(game.ability_activation_count_this_turn(parent, 0), 1);
    }
    #[test]
    fn nested_planned_activation_also_discharges_completed_child() {
        check_nested_preselected_obligations(3);
    }
    #[test]
    fn nested_planned_activation_discharges_only_one_required_occurrence() {
        check_nested_preselected_obligations(0);
    }
    #[test]
    fn nested_planned_activation_preserves_other_ability_on_same_source() {
        check_nested_preselected_obligations(1);
    }
    #[test]
    fn nested_planned_activation_preserves_other_color_on_same_ability() {
        check_nested_preselected_obligations(2);
    }

    #[test]
    fn public_mana_cost_replacement_error_retains_payment_and_restores_costs() {
        check_public_payment_confirmation(false, true);
    }
    #[test]
    fn public_mana_cost_replacement_pending_retains_payment_and_replays_once() {
        check_public_payment_confirmation(true, true);
    }
}

#[cfg(test)]
mod retained_effect_model_stack_program_proposal_tests {
    use super::*;
    #[test]
    fn alternative_programs_and_later_splice_restore_original_after_real_proposal() {
        use crate::alternative_cast::AlternativeCastingMethod;
        for method_index in 0..3 {
            for splice in [false, true] {
                for printed_program in [false, true] {
                    let alice = PlayerId::from_index(0);
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let mut definition = crate::cards::CardDefinitionBuilder::new(
                        crate::CardId::new(),
                        "Original stack program",
                    )
                    .card_types(vec![crate::types::CardType::Sorcery])
                    .build();
                    if printed_program {
                        definition.spell_effect =
                            Some(crate::resolution::ResolutionProgram::from_effects(vec![
                                crate::effect::Effect::gain_life(2),
                            ]));
                    }
                    let effects = vec![crate::effect::Effect::gain_life(7)];
                    definition.alternative_casts.push(match method_index {
                        0 => AlternativeCastingMethod::Overload {
                            cost: crate::mana::ManaCost::new(),
                            effects,
                        },
                        1 => AlternativeCastingMethod::Cleave {
                            cost: crate::mana::ManaCost::new(),
                            effects,
                        },
                        _ => AlternativeCastingMethod::Awaken {
                            amount: 3,
                            cost: crate::mana::ManaCost::new(),
                            effects,
                        },
                    });
                    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
                    let spell = propose_spell_cast(
                        &mut game,
                        source,
                        Zone::Hand,
                        alice,
                        &CastingMethod::Alternative(0),
                    )
                    .unwrap();
                    let object = game.object(spell).unwrap();
                    assert!(
                        object.splice_cast_state.is_some(),
                        "real cast must capture pre-overlay program"
                    );
                    let mut probe = game.clone();
                    let mut context = crate::effects::EffectContext::new_default(spell, alice);
                    for effect in object.spell_effect.as_ref().unwrap().all_effects() {
                        crate::effects::execute_effect(&mut probe, effect, &mut context).unwrap();
                    }
                    assert_eq!(
                        probe.player(alice).unwrap().life,
                        27,
                        "alternate program must remain active on stack"
                    );
                    if splice {
                        let object = game.object_mut(spell).unwrap();
                        assert!(
                            !object.begin_splice_cast_overlay(),
                            "additional overlay must retain earliest snapshot"
                        );
                        let mut program = object.spell_effect_owned().unwrap();
                        program.extend(vec![crate::effect::Effect::gain_life(9)].into());
                        object.spell_effect = Some(program.into());
                        assert_eq!(object.spell_effect.as_ref().unwrap().all_effects().len(), 2);
                    }
                    let mut competing = definition.clone();
                    competing.spell_effect =
                        Some(crate::resolution::ResolutionProgram::from_effects(vec![
                            crate::effect::Effect::gain_life(3),
                        ]));
                    game.create_object_from_definition(&competing, alice, Zone::Hand);
                    let destination = game
                        .move_object(
                            spell,
                            Zone::Graveyard,
                            crate::events::cause::EventCause::effect(),
                        )
                        .unwrap();
                    let object = game.object(destination).unwrap();
                    assert!(object.splice_cast_state.is_none());
                    assert!(object.cast_alternative_method.is_none());
                    assert_eq!(
                        object.spell_effect.is_some(),
                        printed_program,
                        "absence is an original value too"
                    );
                    if let Some(program) = object.spell_effect_owned() {
                        assert_eq!(program.all_effects().len(), 1);
                        let mut context =
                            crate::effects::EffectContext::new_default(destination, alice);
                        for effect in program.all_effects() {
                            crate::effects::execute_effect(&mut game, effect, &mut context)
                                .unwrap();
                        }
                    }
                    assert_eq!(
                        game.player(alice).unwrap().life,
                        if printed_program { 22 } else { 20 }
                    );
                }
            }
        }
    }
    #[test]
    fn mutate_target_program_is_stack_only_and_restores_absence() {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut definition =
            crate::cards::CardDefinitionBuilder::new(crate::CardId::new(), "Mutating original")
                .card_types(vec![crate::types::CardType::Creature])
                .subtypes(vec![crate::types::Subtype::Beast])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
        definition.alternative_casts.push(
            crate::alternative_cast::AlternativeCastingMethod::Mutate {
                cost: crate::mana::ManaCost::new(),
            },
        );
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let spell = propose_spell_cast(
            &mut game,
            source,
            Zone::Hand,
            alice,
            &CastingMethod::Alternative(0),
        )
        .unwrap();
        let object = game.object(spell).unwrap();
        assert!(
            object
                .splice_cast_state
                .as_ref()
                .unwrap()
                .spell_effect
                .is_none()
        );
        let program = object.spell_effect.as_ref().unwrap();
        assert_eq!(program.all_effects().len(), 1);
        assert!(
            program.all_effects()[0]
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
        );
        let mut competing = definition.clone();
        competing.spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
            crate::effect::Effect::gain_life(3),
        ]));
        game.create_object_from_definition(&competing, alice, Zone::Hand);
        let destination = game
            .move_object(
                spell,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
            )
            .unwrap();
        let object = game.object(destination).unwrap();
        assert!(object.spell_effect.is_none());
        assert!(object.splice_cast_state.is_none());
    }
}

#[cfg(test)]
mod planned_mana_witness_tests {
    use super::*;
    use crate::mana::{ManaCost, ManaSymbol};
    struct WhiteChooser {
        prompts: usize,
    }
    impl DecisionMaker for WhiteChooser {
        fn decide_colors(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::ColorsContext,
        ) -> Vec<crate::color::Color> {
            self.prompts += 1;
            vec![crate::color::Color::White; ctx.count as usize]
        }
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            panic!("confirmed mana witnesses must not request replacement ordering");
        }
    }

    #[test]
    fn prepared_production_plan_replays_green_and_manual_override_stays_white() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition =
            crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Choice producer")
                .card_types(vec![crate::types::CardType::Artifact])
                .with_ability(crate::Ability::mana_with_effects(
                    crate::cost::TotalCost::free(),
                    vec![crate::effect::Effect::add_mana_of_any_color_restricted(
                        1,
                        vec![crate::color::Color::Green, crate::color::Color::White],
                    )],
                ))
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let request = crate::mana_payment::ManaPaymentRequest::new(
            alice,
            source,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        );
        let mut manual_game = game.clone();
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        assert!(
            plan.mana_ability_steps[0]
                .production_witnesses
                .as_ref()
                .is_some_and(|records| !records.is_empty())
        );
        let mut payment = crate::mana_payment::PendingManaPayment::new(request, plan);
        let mut dm = WhiteChooser { prompts: 0 };
        assert!(
            !execute_planned_mana_activations(
                &mut game,
                &mut TriggerQueue::new(),
                alice,
                &mut payment,
                &mut false,
                &mut dm
            )
            .unwrap()
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 1);
        assert_eq!(dm.prompts, 0);
        crate::special_actions::perform_activate_mana_ability(
            &mut manual_game,
            alice,
            source,
            0,
            &mut dm,
        )
        .unwrap();
        assert_eq!(dm.prompts, 1);
        assert_eq!(manual_game.player(alice).unwrap().mana_pool.white, 1);
        assert_eq!(manual_game.player(alice).unwrap().mana_pool.green, 0);
    }

    #[test]
    fn fallback_source_keeps_nonfirst_color_under_opponents_search_control() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.add_scoped_player_control(bob, alice, None);
        let definition = crate::cards::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Costly W/U producer",
        )
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(crate::Ability::mana_with_effects(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
            vec![crate::effect::Effect::add_mana_of_any_color_restricted(
                1,
                vec![crate::color::Color::White, crate::color::Color::Blue],
            )],
        ))
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let request = crate::mana_payment::ManaPaymentRequest::new(
            alice,
            source,
            crate::costs::PaymentReason::Effect,
            ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]),
        );
        let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
        let step = &plan.mana_ability_steps[0];
        assert!(
            step.replacement_witnesses.is_none(),
            "life-cost source must retain simulation fallback"
        );
        assert_eq!(
            step.color_restriction,
            Some(vec![crate::color::Color::Blue])
        );
        let records = step.production_witnesses.as_ref().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].output, vec![ManaSymbol::Blue]);
        assert_eq!(records[0].chooser, bob);
        let mut dm = WhiteChooser { prompts: 0 };
        assert_eq!(
            crate::mana_payment::execute_mana_payment_plan(&mut game, &request, &plan, &mut dm)
                .unwrap(),
            crate::mana_payment::ManaPaymentExecution::Paid
        );
        assert_eq!(dm.prompts, 0);
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert!(game.is_tapped(source));
    }

    #[test]
    fn staged_payment_replays_noncommuting_replacement_order() {
        use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
        for amount in [1, 3] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let definition = crate::cards::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Ordered mana source",
            )
            .card_types(vec![crate::types::CardType::Artifact])
            .with_ability(crate::Ability::mana(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
                vec![ManaSymbol::Green; 2],
            ))
            .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            for action in [
                ReplacementAction::ReplaceManaExact(vec![ManaSymbol::Blue]),
                ReplacementAction::Modify(EventModification::Multiply(3)),
            ] {
                game.effect_store
                    .replacement_effects
                    .add_effect(ReplacementEffect::with_matcher(
                        source,
                        alice,
                        crate::events::mana::matchers::ManaProducedBySourceMatcher::new(
                            ObjectFilter::default(),
                        ),
                        action,
                    ));
            }
            game.refresh_continuous_state().unwrap();
            let request = crate::mana_payment::ManaPaymentRequest::new(
                alice,
                source,
                crate::costs::PaymentReason::Effect,
                ManaCost::from_pips(vec![vec![ManaSymbol::Blue]; amount]),
            );
            let plan = crate::mana_payment::plan_first_mana_payment(&game, &request).unwrap();
            assert_eq!(plan.expected_pool_after_activations.blue, amount as u32);
            assert!(plan.mana_ability_steps[0].replacement_witnesses.is_some());
            let mut payment = crate::mana_payment::PendingManaPayment::new(request, plan);
            let mut dm = WhiteChooser { prompts: 0 };
            assert!(
                !execute_planned_mana_activations(
                    &mut game,
                    &mut TriggerQueue::new(),
                    alice,
                    &mut payment,
                    &mut false,
                    &mut dm
                )
                .unwrap()
            );
            assert_eq!(game.player(alice).unwrap().mana_pool.blue, amount as u32);
            assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
            assert!(game.is_tapped(source));
            assert_eq!(dm.prompts, 0);
        }
    }
}
