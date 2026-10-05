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
