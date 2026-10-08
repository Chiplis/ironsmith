//! Shared eligibility and completion receipts for scoped Waterbend payments.
use super::{ManaPaymentRequest, PlannedPipAllocation, PlannedPipPayment};
use crate::{game_state::GameState, ids::ObjectId};

pub(crate) fn validate_waterbend_scope(
    request: &ManaPaymentRequest,
) -> Result<(), crate::effects::ExecutionError> {
    request
        .cost
        .waterbend_capacity_checked(request.x_value)
        .map(|_| ())
        .ok_or_else(|| {
            crate::effects::ExecutionError::IncompleteEvidence(
                "Waterbend payment scope is empty or its quantity overflows".into(),
            )
        })
}

pub(crate) fn waterbend_sources(game: &GameState, request: &ManaPaymentRequest) -> Vec<ObjectId> {
    if let Err(error) = validate_waterbend_scope(request) {
        game.record_token_resource_failure(&error);
        return Vec::new();
    }
    if request.cost.waterbend_capacity(request.x_value) == 0 || request.assist_completion.is_some()
    {
        return Vec::new();
    }
    let mut sources = crate::decision::get_improvise_artifacts(game, request.payer);
    sources.extend(
        crate::decision::get_convoke_creatures(game, request.payer)
            .into_iter()
            .map(|(id, _)| id),
    );
    sources.retain(|id| {
        !game.is_phased_out(*id)
            && !request.reserved_tap_sources.contains(id)
            && !request.preferences.excluded_sources.contains(id)
    });
    sources.sort_unstable();
    sources.dedup();
    sources
}

/// Recheck live ownership, types and tap state after all mana activations. A
/// selected resource cannot also pay {T}, another alternative, or mana.
pub(crate) fn validate_waterbend_taps(
    game: &GameState,
    request: &ManaPaymentRequest,
    allocations: &[PlannedPipAllocation],
) -> bool {
    let selected = allocations
        .iter()
        .filter_map(|allocation| match allocation.payment {
            PlannedPipPayment::Waterbend(id) => Some(id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let legal = waterbend_sources(game, request);
    selected.len() <= request.cost.waterbend_capacity(request.x_value) as usize
        && selected
            .iter()
            .enumerate()
            .all(|(index, id)| legal.contains(id) && !selected[..index].contains(id))
}

/// Accepted zero is deliberately represented by a real completion event.
/// Call once, only after the entire typed payment has succeeded.
pub(crate) fn record_waterbend_payment(
    game: &mut GameState,
    request: &ManaPaymentRequest,
) -> Result<(), crate::effects::ExecutionError> {
    record_waterbend_payment_with_outputs(game, request).map(|_| ())
}

pub(crate) fn record_waterbend_payment_with_outputs(
    game: &mut GameState,
    request: &ManaPaymentRequest,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, crate::effects::ExecutionError> {
    validate_waterbend_scope(request)?;
    let Some(scope) = request.cost.waterbend_payment_scope() else {
        return Ok(Vec::new());
    };
    let mut outputs = Vec::new();
    for obligation in &scope.obligations {
        let amount = obligation.amount(request.x_value).ok_or_else(|| {
            crate::effects::ExecutionError::IncompleteEvidence(
                "Waterbend receipt quantity overflows".into(),
            )
        })?;
        let provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::KeywordAction);
        let completion =
            crate::effects::composition::observe_keyword_action_completion_with_outputs(
                game,
                crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::KeywordActionEvent::new(
                        crate::events::KeywordActionKind::Waterbend,
                        request.payer,
                        request.source,
                        amount,
                    ),
                    provenance,
                ),
            )?;
        for event in &completion.outcome.events {
            game.queue_trigger_event(event.provenance(), event.clone());
        }
        outputs.push(completion);
    }
    Ok(outputs)
}

/// A conservative declaration bound followed by exact planner checks. This
/// handles activation and effect costs as well as casting prices.
pub(crate) fn maximum_waterbend_x(
    game: &GameState,
    request: &ManaPaymentRequest,
) -> Result<u32, super::ManaPaymentFailure> {
    let mut inventory = request.clone();
    inventory.x_value = 1;
    let upper = crate::derived_view::DerivedGameView::new(game)
        .potential_mana(request.payer)
        .total()
        .saturating_add(waterbend_sources(game, &inventory).len() as u32);
    let mut lower = 0;
    let mut upper = upper;
    while lower < upper {
        let middle = lower + (upper - lower) / 2 + (upper - lower) % 2;
        let mut candidate = request.clone();
        candidate.x_value = middle;
        match super::check_mana_payment(game, &candidate) {
            Ok(()) => lower = middle,
            Err(error @ super::ManaPaymentFailure::EffectExecutionFailed(_)) => return Err(error),
            Err(_) => upper = middle - 1,
        }
    }
    Ok(lower)
}

#[cfg(test)]
mod contracts {
    use super::*;
    #[test]
    fn overflowing_obligation_fails_before_payment_or_receipt() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let payer = crate::PlayerId::from_index(0);
        let source = game.new_object_id();
        let cost = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X; 2]).with_waterbend();
        let request = ManaPaymentRequest::new(payer, source, crate::costs::PaymentReason::Effect, cost).with_x(u32::MAX);
        assert!(matches!(super::super::plan_first_mana_payment(&game, &request),
            Err(super::super::ManaPaymentFailure::EffectExecutionFailed(crate::effects::ExecutionError::IncompleteEvidence(_)))));
        assert!(matches!(record_waterbend_payment(&mut game, &request), Err(crate::effects::ExecutionError::IncompleteEvidence(_))));
        assert!(game.take_pending_trigger_events().is_empty());
        assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
    }
}
