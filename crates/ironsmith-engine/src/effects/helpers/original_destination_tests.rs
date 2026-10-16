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

#[test]
fn origin_queries_keep_past_controller_tokens_and_only_successful_original_destinations() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    let source = game.new_object_id();
    let mut origin = crate::snapshot::ObjectSnapshot::for_testing(game.new_object_id(), bob, "Returned token");
    origin.owner = alice;
    origin.zone = Zone::Battlefield;
    origin.card_types = vec![crate::types::CardType::Creature];
    origin.kind = crate::object::ObjectKind::Token;
    let mut arrival = origin.clone(); arrival.object_id = game.new_object_id(); arrival.zone = Zone::Hand;
    arrival.controller = alice; arrival.card_types = vec![crate::types::CardType::Artifact];
    let id = crate::effect::EffectId(15);
    let mut query = PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
        .with_filter(crate::target::ObjectFilter::creature().in_zone(Zone::Battlefield).controlled_by(PlayerFilter::IteratedPlayer));
    query.use_origin_memory = true;
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.iteration.iterated_player = Some(bob);
    assert!(matches!(resolve_prior_effect_metric(&game, &ctx, id, &query), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.store_outcome(id, EffectOutcome::count(0).with_execution_fact(ExecutionFact::OriginalZoneMoveObjects(vec![])));
    assert_eq!(resolve_prior_effect_metric(&game, &ctx, id, &query).unwrap(), 0);
    let mut redirected = arrival.clone(); redirected.zone = Zone::Exile;
    let original = EffectOutcome::count(1).with_execution_fact(ExecutionFact::OriginalZoneMoveObjects(vec![
        (origin.clone(), Zone::Hand, arrival.clone()),
        (origin.clone(), Zone::Hand, arrival.clone()), // Split arrivals count one source object.
        (origin.clone(), Zone::Hand, redirected),
    ]));
    let mut other = origin.clone(); other.object_id = game.new_object_id();
    let addition = EffectOutcome::count(1).with_execution_fact(ExecutionFact::OriginalZoneMoveObjects(vec![(other, Zone::Hand, arrival)]));
    ctx.store_outcome(id, EffectOutcome::aggregate_replacement_outcomes(original, [addition]));
    assert_eq!(resolve_prior_effect_metric(&game, &ctx, id, &query).unwrap(), 1);
    ctx.iteration.iterated_player = Some(alice);
    assert_eq!(resolve_prior_effect_metric(&game, &ctx, id, &query).unwrap(), 0);
}
