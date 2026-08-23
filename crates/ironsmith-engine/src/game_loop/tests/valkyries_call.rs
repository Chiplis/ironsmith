#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

const VALKYRIES_CALL_TEXT: &str = "Whenever a nontoken, non-Angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types.";

fn valkyries_call_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Valkyrie's Call")
        .card_types(vec![CardType::Enchantment])
        .parse_text(VALKYRIES_CALL_TEXT)
        .expect("Valkyrie's Call should parse")
}

fn creature_definition(name: &str, subtype: Subtype, token: bool) -> crate::cards::CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 2));
    if token {
        builder.token().build()
    } else {
        builder.build()
    }
}

fn assert_no_call_trigger(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    object: ObjectId,
    label: &str,
) {
    game.move_object_by_effect(object, Zone::Graveyard)
        .unwrap_or_else(|| panic!("{label} should move to the graveyard"));
    drain_pending_trigger_events(game, queue);
    assert!(
        queue.entries.is_empty(),
        "Valkyrie's Call must not trigger for {label}: {:?}",
        queue.entries
    );
}

#[test]
fn valkyries_call_follows_the_returned_incarnation_for_all_modifications() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&valkyries_call_definition(), alice, Zone::Battlefield);

    let human = creature_definition("Returned Human", Subtype::Human, false);
    let original_id = game.create_object_from_definition(&human, alice, Zone::Battlefield);
    let stable_id = game
        .object(original_id)
        .expect("test creature should exist")
        .stable_id;

    let graveyard_id = game
        .move_object_by_effect(original_id, Zone::Graveyard)
        .expect("test creature should die");
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue.entries.len(),
        1,
        "exactly one Valkyrie's Call trigger should be pending"
    );

    put_triggers_on_stack(&mut game, &mut queue)
        .expect("Valkyrie's Call trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Valkyrie's Call trigger should resolve");
    game.refresh_continuous_state();

    let returned_id = game
        .find_object_by_stable_id(stable_id)
        .expect("the returned card should preserve its stable identity");
    let returned = game
        .object(returned_id)
        .expect("the returned incarnation should exist");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(returned.owner, alice);
    assert_eq!(game.current_controller(returned_id), Some(alice));
    assert_ne!(
        returned_id, graveyard_id,
        "returning the card must create a new battlefield incarnation"
    );
    assert_eq!(
        game.counter_count(returned_id, crate::object::CounterType::PlusOnePlusOne),
        1,
        "the battlefield incarnation should enter with the counter"
    );

    let subtypes = game
        .current_subtypes(returned_id)
        .expect("returned creature should have current characteristics");
    assert!(
        subtypes.contains(&Subtype::Human),
        "the returned object should retain its original subtype: {subtypes:?}"
    );
    assert!(
        subtypes.contains(&Subtype::Angel),
        "the returned object should gain Angel: {subtypes:?}"
    );
    assert!(
        game.object_has_static_ability_id(
            returned_id,
            crate::static_abilities::StaticAbilityId::Flying
        ),
        "the returned object should gain flying"
    );
}

#[test]
fn valkyries_call_rejects_each_nonmatching_filter_branch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&valkyries_call_definition(), alice, Zone::Battlefield);
    let mut queue = TriggerQueue::new();

    let artifact = CardDefinitionBuilder::new(CardId::new(), "Nontoken Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    assert_no_call_trigger(
        &mut game,
        &mut queue,
        artifact_id,
        "a nontoken noncreature permanent",
    );

    let angel = creature_definition("Existing Angel", Subtype::Angel, false);
    let angel_id = game.create_object_from_definition(&angel, alice, Zone::Battlefield);
    assert_no_call_trigger(
        &mut game,
        &mut queue,
        angel_id,
        "an Angel creature you control",
    );

    let token = creature_definition("Human Token", Subtype::Human, true);
    let token_id = game.create_object_from_definition(&token, alice, Zone::Battlefield);
    assert_no_call_trigger(
        &mut game,
        &mut queue,
        token_id,
        "a non-Angel creature token",
    );

    let opponent_creature = creature_definition("Opponent Human", Subtype::Human, false);
    let opponent_id =
        game.create_object_from_definition(&opponent_creature, bob, Zone::Battlefield);
    assert_no_call_trigger(
        &mut game,
        &mut queue,
        opponent_id,
        "a non-Angel nontoken creature an opponent controls",
    );
}
