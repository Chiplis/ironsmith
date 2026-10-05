//! UNVALIDATED pending-stack LKI contracts. Public movement/payment links retain
//! their own scope; matching a stable physical card is not a link by itself.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Until, Value};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::game_state::StackEntry;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
const A: PlayerId = PlayerId::from_index(0);
fn definition() -> CardDefinition {
    compile_to_runtime_definition(
        "Exact LKI object",
        "Type: Creature — Human\nPower/Toughness: 2/2",
        false,
    )
    .unwrap()
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn snapshot(game: &GameState, id: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(id).unwrap(), game)
}
fn pump(game: &mut GameState, id: ObjectId, amount: i32) {
    let mut dm = SelectFirstDecisionMaker;
    execute_effect(
        game,
        &Effect::pump(
            amount,
            amount,
            ChooseSpec::SpecificObject(id),
            Until::EndOfTurn,
        ),
        &mut EffectContext::new(id, A, &mut dm),
    )
    .unwrap();
}
fn resolve(game: &mut GameState) {
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
}

#[test]
fn pending_source_lki_refreshes_at_its_departure_only() {
    let mut game = game();
    let source = game.create_object_from_definition(&definition(), A, Zone::Battlefield);
    game.push_to_stack(StackEntry::ability(
        source,
        A,
        vec![Effect::gain_life(Value::SourcePower)],
    ));
    pump(&mut game, source, 3);
    let grave = game
        .move_object_by_game_rule(source, Zone::Graveyard)
        .unwrap();
    assert_eq!(
        game.stack[0].source_snapshot.as_ref().unwrap().power,
        Some(5)
    );
    let returned = game
        .move_object_by_game_rule(grave, Zone::Battlefield)
        .unwrap();
    pump(&mut game, returned, 40);
    game.move_object_by_game_rule(returned, Zone::Graveyard)
        .unwrap();
    let lki = game.stack[0].source_snapshot.as_ref().unwrap();
    assert_eq!(lki.object_id, source);
    assert_eq!(lki.power, Some(5));
    resolve(&mut game);
    assert_eq!(game.player(A).unwrap().life, 25);
}

#[test]
fn pending_tagged_lki_keeps_the_watched_incarnation_after_two_later_departures() {
    let mut game = game();
    let source = game.create_object_from_definition(&definition(), A, Zone::Battlefield);
    let watched = game.create_object_from_definition(&definition(), A, Zone::Battlefield);
    let mut entry = StackEntry::ability(
        source,
        A,
        vec![Effect::gain_life(Value::PowerOf(Box::new(
            ChooseSpec::Tagged("watched".into()),
        )))],
    );
    entry
        .tagged_objects
        .insert("watched".into(), vec![snapshot(&game, watched)]);
    game.push_to_stack(entry);
    pump(&mut game, watched, 3);
    let grave = game
        .move_object_by_game_rule(watched, Zone::Graveyard)
        .unwrap();
    let returned = game
        .move_object_by_game_rule(grave, Zone::Battlefield)
        .unwrap();
    pump(&mut game, returned, 50);
    game.move_object_by_game_rule(returned, Zone::Graveyard)
        .unwrap();
    let lki = &game.stack[0].tagged_objects["watched"][0];
    assert_eq!(lki.object_id, watched);
    assert_eq!(lki.power, Some(5));
    resolve(&mut game);
    assert_eq!(game.player(A).unwrap().life, 25);
}

#[test]
fn public_cost_move_is_linked_once_when_the_ability_enters_the_stack() {
    for moves_again in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition(), A, Zone::Battlefield);
        let paid = game.create_object_from_definition(&definition(), A, Zone::Battlefield);
        let before_cost = snapshot(&game, paid);
        let stable = before_cost.stable_id;
        let grave = game
            .move_object_by_game_rule(paid, Zone::Graveyard)
            .unwrap();
        let mut entry = StackEntry::ability(
            source,
            A,
            vec![Effect::return_from_graveyard_to_battlefield(
                ChooseSpec::Tagged("paid".into()),
                false,
            )],
        );
        entry
            .tagged_objects
            .insert("paid".into(), vec![before_cost]);
        game.push_to_stack(entry);
        let linked = &game.stack[0].tagged_objects["paid"][0];
        assert_eq!(linked.object_id, grave);
        assert_eq!(linked.zone, Zone::Graveyard);
        if moves_again {
            let exile = game.move_object_by_game_rule(grave, Zone::Exile).unwrap();
            game.move_object_by_game_rule(exile, Zone::Graveyard)
                .unwrap();
        }
        resolve(&mut game);
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(current).unwrap().zone,
            if moves_again {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
    }
}
