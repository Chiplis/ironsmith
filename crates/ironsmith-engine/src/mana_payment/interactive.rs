//! Manual source activation shares the normal costs and replay decision machinery.
use super::*;
use crate::cost::CostPaymentError;
use crate::decision::DecisionMaker;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Unlike planner simulations, this includes sources whose costs need player choices
/// or mana from other sources. Every entry is checked again when activated.
pub fn manual_mana_abilities(game: &GameState, request: &ManaPaymentRequest) -> Vec<(ObjectId, usize)> {
    manual_mana_abilities_checked(game, request).unwrap_or_default()
}

pub fn manual_mana_abilities_checked(game: &GameState, request: &ManaPaymentRequest) -> Result<Vec<(ObjectId, usize)>, crate::effects::ExecutionError> {
    if !request.allow_mana_abilities { return Ok(Vec::new()); }
    super::planner::with_inventory_query(game, |query| super::planner::useful_manual_mana_abilities(query, request))
}

/// False means the activation was cancelled or needs replay with another answer.
/// The enclosing payment is retained in either case.
pub(crate) fn activate_mana_during_payment(
    game: &mut GameState,
    request: &ManaPaymentRequest,
    source: ObjectId,
    ability_index: usize,
    dm: &mut dyn DecisionMaker,
) -> Result<bool, crate::special_actions::ActionError> {
    let (root, meter) = game.begin_token_resource_scope();
    let checkpoint = game.clone();
    let mut result = activate_mana_during_payment_inner(game, request, source, ability_index, dm);
    if let Err(crate::special_actions::ActionError::ExecutionFailure { error, .. }) = &result {
        game.record_token_resource_failure(error);
    }
    if let Some(error) = game.token_resource_failure() {
        result = Err(crate::special_actions::ActionError::ExecutionFailure { source, error });
        game.restore_execution_checkpoint(checkpoint, false);
    }
    game.end_token_resource_scope(root, &meter);
    result
}

fn activate_mana_during_payment_inner(
    game: &mut GameState, request: &ManaPaymentRequest, source: ObjectId,
    ability_index: usize, dm: &mut dyn DecisionMaker,
) -> Result<bool, crate::special_actions::ActionError> {
    let manual = manual_mana_abilities_checked(game, request)
        .map_err(|error| crate::special_actions::ActionError::ExecutionFailure { source, error })?;
    if !manual.contains(&(source, ability_index)) {
        return Err(crate::special_actions::ActionError::CantPayCost);
    }
    let checkpoint = game.clone();
    let snapshot = game
        .object(source)
        .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game));
    let has_tap = game.current_ability(source, ability_index).is_some_and(|ability| {
        matches!(&ability.kind, crate::ability::AbilityKind::Activated(a) if a.has_tap_cost())
    });
    let mut exclusions = if request.reason == crate::costs::PaymentReason::ActivateManaAbility {
        request.preferences.excluded_sources.clone()
    } else {
        Vec::new()
    };
    exclusions.push(source);
    let result = crate::special_actions::perform_mana_ability_with_payment_mode(
        game,
        request.payer,
        source,
        ability_index,
        None,
        Some(exclusions),
        dm,
    );
    if dm.awaiting_choice() {
        return Ok(false);
    }
    let events = match result {
        Ok(events) => events,
        Err(error @ crate::special_actions::ActionError::ExecutionFailure { .. }) => {
            *game = checkpoint; return Err(error);
        }
        Err(_) => { *game = checkpoint; return Ok(false); }
    };
    for event in events {
        game.queue_trigger_event(event.provenance(), event);
    }
    let provenance = game
        .provenance_graph_mut()
        .alloc_root_event(crate::events::EventKind::AbilityActivated);
    game.queue_trigger_event(
        provenance,
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::AbilityActivatedEvent::new(source, request.payer, true)
                .with_activation_cost_has_tap(has_tap)
                .with_snapshot(snapshot),
            provenance,
        ),
    );
    Ok(true)
}

pub(crate) fn pay_mana_interactively(
    game: &mut GameState,
    payer: PlayerId,
    source: ObjectId,
    cost: crate::mana::ManaCost,
    reason: crate::costs::PaymentReason,
    exclusions: Vec<ObjectId>,
    dm: &mut dyn DecisionMaker,
) -> Result<(), CostPaymentError> {
    pay_mana_interactively_in_context(game, payer, source, cost, reason, exclusions, dm, None)
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn pay_mana_interactively_in_context(
    game: &mut GameState, payer: PlayerId, source: ObjectId, cost: crate::mana::ManaCost,
    reason: crate::costs::PaymentReason, exclusions: Vec<ObjectId>, dm: &mut dyn DecisionMaker,
    execution: Option<&crate::effects::ExecutionContextCheckpoint>,
) -> Result<(), CostPaymentError> {
    if cost.is_empty() {
        return Ok(());
    }
    let mut request = ManaPaymentRequest::new(payer, source, reason, cost)
        .with_spend_policy(game.mana_spend_policy(payer, Some(source)));
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(payer, Some(source), request.reason);
    request.preferences.excluded_sources = exclusions.clone();
    loop {
        // Foreground prompts need one executable proposal. Source selection
        // remains available through constrained replanning below.
        let plan = match plan_first_mana_payment(game, &request) {
            Ok(plan) => plan,
            Err(ManaPaymentFailure::EffectExecutionFailed(error)) => return Err(CostPaymentError::ExecutionFailed(error)),
            Err(_) => unfunded_mana_payment_plan(game, &request),
        };
        let subject = game
            .object(source)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| "action".to_string());
        let decision = crate::decisions::context::ManaPaymentContext::new(
            payer,
            source,
            subject,
            request.clone(),
            plan.clone(),
        );
        let response = dm.decide_mana_payment(game, &decision);
        if dm.awaiting_choice() {
            return Err(CostPaymentError::InsufficientMana);
        }
        match response {
            ManaPaymentResponse::Cancel => return Err(CostPaymentError::Cancelled),
            ManaPaymentResponse::Replan { mut preferences } => {
                preferences
                    .excluded_sources
                    .extend(exclusions.iter().copied());
                preferences.normalize();
                request.preferences = preferences;
            }
            ManaPaymentResponse::Activate {
                source,
                ability_index,
            } => {
                activate_mana_during_payment(game, &request, source, ability_index, dm)
                    .map_err(|error| match error {
                        crate::special_actions::ActionError::ExecutionFailure { error, .. } => CostPaymentError::ExecutionFailed(error),
                        _ => CostPaymentError::InsufficientMana,
                    })?;
                if dm.awaiting_choice() {
                    return Err(CostPaymentError::InsufficientMana);
                }
            }
            ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if plan.payable && plan_id == plan.id && request_hash == plan.request_hash => {
                return match super::execute_mana_payment_plan_in_context(game, &request, &plan, dm, execution) {
                    Ok(ManaPaymentExecution::Paid) => Ok(()),
                    Err(ManaPaymentFailure::EffectExecutionFailed(error)) => Err(CostPaymentError::ExecutionFailed(error)),
                    _ => Err(CostPaymentError::InsufficientMana),
                };
            }
            ManaPaymentResponse::Confirm { .. } => return Err(CostPaymentError::InsufficientMana),
        }
    }
}

#[cfg(test)]
mod foreground_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    struct ConfirmOrReplan {
        required: Option<ObjectId>,
        prompts: usize,
    }
    impl DecisionMaker for ConfirmOrReplan {
        fn decide_mana_payment(&mut self, _game: &GameState,
            context: &crate::decisions::context::ManaPaymentContext) -> ManaPaymentResponse {
            self.prompts += 1;
            assert!(last_mana_payment_perf().visited_nodes < 64,
                "foreground prompt must not rank every equivalent source permutation");
            if let Some(source) = self.required.take() {
                let mut preferences = context.request.preferences.clone();
                preferences.required_sources.push(source);
                return ManaPaymentResponse::Replan { preferences };
            }
            ManaPaymentResponse::Confirm {
                plan_id: context.plan.id, request_hash: context.plan.request_hash,
            }
        }
    }

    #[test]
    fn foreground_payment_preserves_life_costs_and_constrained_source_alternatives() {
        for replan in [false, true] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut sources = Vec::new();
            for index in 0..8 {
                let card = CardBuilder::new(CardId::new(), format!("Mana source {index}"))
                    .card_types(vec![CardType::Land]).build();
                let id = game.create_object_from_card(&card, alice, Zone::Battlefield);
                let mut costs = vec![crate::costs::Cost::tap()];
                if index >= 3 { costs.push(crate::costs::Cost::life(1)); }
                game.object_mut(id).unwrap().abilities_mut().push(crate::Ability::mana(
                    crate::cost::TotalCost::from_costs(costs), vec![ManaSymbol::Red]));
                sources.push(id);
            }
            let mut dm = ConfirmOrReplan { required: replan.then_some(sources[7]), prompts: 0 };
            pay_mana_interactively(&mut game, alice, sources[0],
                ManaCost::new().add_generic(4), crate::costs::PaymentReason::Effect,
                vec![], &mut dm).unwrap();
            assert_eq!(dm.prompts, if replan { 2 } else { 1 });
            assert_eq!(sources.iter().filter(|source| game.is_tapped(**source)).count(), 4);
            assert_eq!(game.player(alice).unwrap().life, 19);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            if replan { assert!(game.is_tapped(sources[7])); }
        }
    }
}
