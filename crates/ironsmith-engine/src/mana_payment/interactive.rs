//! Manual source activation shares the normal costs and replay decision machinery.
use super::*;
use crate::cost::CostPaymentError;
use crate::decision::DecisionMaker;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Unlike planner simulations, this includes sources whose costs need player choices
/// or mana from other sources. Every entry is checked again when activated.
pub fn manual_mana_abilities(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Vec<(ObjectId, usize)> {
    if !request.allow_mana_abilities {
        return Vec::new();
    }
    super::planner::useful_manual_mana_abilities(game, request)
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
    if !manual_mana_abilities(game, request).contains(&(source, ability_index)) {
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
    if result.is_err() {
        *game = checkpoint;
        return Ok(false);
    }
    for event in result.unwrap() {
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

pub(crate) fn pay_activation_mana_interactively(
    game: &mut GameState,
    payer: PlayerId,
    source: ObjectId,
    cost: crate::mana::ManaCost,
    exclusions: Vec<ObjectId>,
    dm: &mut dyn DecisionMaker,
) -> Result<(), CostPaymentError> {
    let mut request = ManaPaymentRequest::new(
        payer,
        source,
        crate::costs::PaymentReason::ActivateManaAbility,
        cost,
    )
    .with_spend_policy(game.mana_spend_policy(payer, Some(source)));
    request.allow_black_life = crate::decision::mana_cost_has_black_symbol(&request.cost)
        && game.player_can_pay_black_with_life_for_reason(payer, Some(source), request.reason);
    request.preferences.excluded_sources = exclusions.clone();
    loop {
        let plan = plan_mana_payment(game, &request)
            .ok()
            .and_then(|plans| plans.into_iter().next())
            .unwrap_or_else(|| unfunded_mana_payment_plan(game, &request));
        let subject = game
            .object(source)
            .map(|object| format!("{}'s mana ability", object.name))
            .unwrap_or_else(|| "mana ability".to_string());
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
            ManaPaymentResponse::Cancel => return Err(CostPaymentError::InsufficientMana),
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
                    .map_err(|_| CostPaymentError::InsufficientMana)?;
                if dm.awaiting_choice() {
                    return Err(CostPaymentError::InsufficientMana);
                }
            }
            ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            } if plan.payable && plan_id == plan.id && request_hash == plan.request_hash => {
                return match execute_mana_payment_plan(game, &request, &plan, dm) {
                    Ok(ManaPaymentExecution::Paid) => Ok(()),
                    _ => Err(CostPaymentError::InsufficientMana),
                };
            }
            ManaPaymentResponse::Confirm { .. } => return Err(CostPaymentError::InsufficientMana),
        }
    }
}
