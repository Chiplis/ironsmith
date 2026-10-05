use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::{Effect, Until};
use crate::effects::execute_effect;
use crate::events::cause::EventCause;
use crate::triggers::TriggerEvent;
use crate::{CardId, CardType, Zone};

fn creature(game: &mut GameState) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "LKI fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        PlayerId::from_index(0),
        Zone::Battlefield,
    )
}
fn snapshot(game: &GameState, id: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(id).unwrap(), game)
}
fn pump(game: &mut GameState, source: ObjectId, target: ObjectId, amount: i32) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm);
    execute_effect(
        game,
        &Effect::pump(
            amount,
            amount,
            ChooseSpec::SpecificObject(target),
            Until::EndOfTurn,
        ),
        &mut ctx,
    )
    .unwrap();
}

#[test]
fn tagged_number_reads_exact_departure_not_a_later_incarnation_of_same_card() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = creature(&mut game);
    let object = creature(&mut game);
    let original = snapshot(&game, object);
    pump(&mut game, source, object, 3);
    {
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm);
        ctx.tag_object("still_here", original.clone());
        assert_eq!(
            resolve_value(
                &game,
                &Value::PowerOf(Box::new(ChooseSpec::Tagged("still_here".into()))),
                &ctx
            )
            .unwrap(),
            5,
            "while the exact object remains, read live calculated power rather than the old 2-power event snapshot"
        );
    }
    let grave = game
        .move_object_by_game_rule(object, Zone::Graveyard)
        .unwrap();
    let returned = game
        .move_object_by_game_rule(grave, Zone::Battlefield)
        .unwrap();
    pump(&mut game, source, returned, 9);
    let _ = game
        .move_object_by_game_rule(returned, Zone::Graveyard)
        .unwrap();
    let lki = latest_tagged_lki_snapshot(&game, &original).unwrap();
    assert_eq!(lki.object_id, object);
    assert_eq!(lki.power, Some(5), "the later incarnation had 11 power");
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm);
    ctx.resolution_object_id_floor = Some(ObjectId(game.next_object_id_counter()));
    ctx.tag_object("watched", original);
    let value = Value::PowerOf(Box::new(ChooseSpec::Tagged("watched".into())));
    assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 5);
}

#[test]
fn emerge_receipt_reads_immutable_characteristics_rather_than_a_live_object() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = creature(&mut game);
    let material = creature(&mut game);
    let paid = snapshot(&game, material);
    pump(&mut game, source, material, 7);
    let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
    ctx.tag_object(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG, paid);
    let value = Value::ToughnessOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG.into())));
    assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 2);
    assert_eq!(game.current_toughness(material), Some(9));
}

#[test]
fn malformed_emerge_receipts_are_incomplete_through_positive_negated_and_outer_rollback_paths() {
    for kind in ["absent", "multiple", "wrong-zone", "noncreature", "missing-toughness"] {
        for negated in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = creature(&mut game);
            let material = creature(&mut game);
            let mut receipt = snapshot(&game, material);
            match kind {
                "wrong-zone" => receipt.zone = Zone::Graveyard,
                "noncreature" => receipt.card_types = vec![CardType::Artifact],
                "missing-toughness" => receipt.toughness = None,
                _ => {}
            }
            let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
            if kind != "absent" {
                ctx.set_tagged_objects(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG, match kind {
                    "multiple" => vec![receipt.clone(), receipt], _ => vec![receipt],
                });
            }
            let value = Value::ToughnessOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG.into())));
            let (root, meter) = game.begin_token_resource_scope();
            assert!(matches!(resolve_value(&game, &value, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
            assert!(matches!(game.token_resource_failure(), Some(ExecutionError::IncompleteEvidence(_))));
            game.end_token_resource_scope(root, &meter);
            let mut condition = crate::effect::Condition::ValueComparison {
                left: value, operator: crate::effect::ValueComparisonOperator::GreaterThan,
                right: Value::Fixed(0),
            };
            if negated { condition = crate::effect::Condition::Not(Box::new(condition)); }
            let effect = Effect::new(crate::effects::SequenceEffect::new(vec![
                Effect::gain_life(4), Effect::conditional(condition,
                    vec![Effect::gain_life(8)], vec![Effect::gain_life(16)]),
            ]));
            assert!(matches!(execute_effect(&mut game, &effect, &mut ctx),
                Err(ExecutionError::IncompleteEvidence(_))), "{kind}, negated={negated}");
            assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20,
                "outer mutation rolls back rather than selecting either branch");
        }
    }
}

#[test]
fn known_empty_emerge_sacrifice_receipt_is_zero_and_does_not_latch_missing_evidence() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = creature(&mut game);
    let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
    ctx.set_tagged_objects(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG, vec![]);
    let (root, meter) = game.begin_token_resource_scope();
    let value = Value::ToughnessOf(Box::new(ChooseSpec::Tagged(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG.into())));
    assert_eq!(resolve_value(&game, &value, &ctx).unwrap(), 0);
    assert!(game.token_resource_failure().is_none());
    game.end_token_resource_scope(root, &meter);
}

#[test]
fn tagged_lki_searches_every_member_of_a_simultaneous_departure() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = creature(&mut game);
    let first = creature(&mut game);
    let second = creature(&mut game);
    let original = snapshot(&game, second);
    pump(&mut game, source, second, 4);
    // Isolate the batch representation contract: the primary snapshot is not
    // the tagged object, and no individual departure record can hide the bug.
    let batch = ZoneChangeEvent::batch_with_snapshots(
        vec![first, second],
        Zone::Battlefield,
        Zone::Graveyard,
        EventCause::from_game_rule(),
        vec![snapshot(&game, first), snapshot(&game, second)],
    );
    game.record_turn_history_event(&TriggerEvent::new_with_provenance(
        batch,
        Default::default(),
    ));
    let lki = latest_tagged_lki_snapshot(&game, &original).unwrap();
    assert_eq!(lki.object_id, second);
    assert_eq!(lki.power, Some(6));
}

#[test]
fn explicit_same_resolution_move_links_and_updated_tags_still_follow_the_new_object() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = creature(&mut game);
    let object = creature(&mut game);
    let old = snapshot(&game, object);
    let stable = old.stable_id;
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut dm);
    ctx.resolution_object_id_floor = Some(ObjectId(game.next_object_id_counter()));
    ctx.tag_object("moved", old);
    let reference = ChooseSpec::Tagged("moved".into());
    execute_effect(&mut game, &Effect::exile(reference.clone()), &mut ctx).unwrap();
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    // CR 400.7j permits this resolution's own movement. It does not rely on
    // latest_tagged_lki_snapshot accepting an unrelated stable-id incarnation.
    execute_effect(
        &mut game,
        &Effect::move_to_zone(reference, Zone::Battlefield, false),
        &mut ctx,
    )
    .unwrap();
    let returned = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
    ctx.set_tagged_objects("moved", vec![snapshot(&game, returned)]);
    execute_effect(
        &mut game,
        &Effect::pump(2, 2, ChooseSpec::SpecificObject(returned), Until::EndOfTurn),
        &mut ctx,
    )
    .unwrap();
    assert_eq!(
        resolve_value(
            &game,
            &Value::PowerOf(Box::new(ChooseSpec::Tagged("moved".into()))),
            &ctx
        )
        .unwrap(),
        4
    );
}
