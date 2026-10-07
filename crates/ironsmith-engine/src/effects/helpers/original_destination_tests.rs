use super::*;
use crate::effect::ExecutionFact;

#[test]
fn actual_destination_queries_require_the_exact_completed_original_receipt() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let controller = PlayerId::from_index(0);
    let source = game.new_object_id();
    let id = crate::effect::EffectId(701);
    let mut query = PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
        .with_action(ironsmith_core::PriorEffectAction::PutIntoGraveyard);
    query.original_destination = Some(Zone::Graveyard);
    let mut ctx = ExecutionContext::new_default(source, controller);
    assert!(matches!(resolve_prior_effect_metric(&game, &ctx, id, &query), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.store_outcome(id, EffectOutcome::count(8));
    assert!(matches!(resolve_prior_effect_metric(&game, &ctx, id, &query), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.store_outcome(id, EffectOutcome::count(0).with_execution_fact(ExecutionFact::OriginalZoneMoveCards(vec![])));
    assert_eq!(resolve_prior_effect_metric(&game, &ctx, id, &query).unwrap(), 0);
    let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(120_060), "Arrival")
        .card_types(vec![CardType::Artifact]).build();
    let object = game.create_object_from_card(&card, controller, Zone::Graveyard);
    let memory = crate::effect::OutcomeObjectMemory::from_snapshot(&ObjectSnapshot::from_object(game.object(object).unwrap(), &game));
    let original = EffectOutcome::count(1).with_execution_fact(ExecutionFact::OriginalZoneMoveCards(vec![memory.clone()]));
    let additions = EffectOutcome::count(1).with_execution_fact(ExecutionFact::OriginalZoneMoveCards(vec![memory]));
    ctx.store_outcome(id, EffectOutcome::aggregate_replacement_outcomes(original, [additions]));
    game.move_object_by_effect(object, Zone::Exile).unwrap();
    assert_eq!(resolve_prior_effect_metric(&game, &ctx, id, &query).unwrap(), 1,
        "the original arrival counts after leaving, and the added instruction is excluded");
    query.original_destination = Some(Zone::Hand);
    assert_eq!(resolve_prior_effect_metric(&game, &ctx, id, &query).unwrap(), 0);
}
