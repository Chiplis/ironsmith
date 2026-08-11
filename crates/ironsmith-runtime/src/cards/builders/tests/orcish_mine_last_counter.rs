#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: [&str; 4] = [
    "Enchant land",
    "This Aura enters with three ore counters on it.",
    "At the beginning of your upkeep and whenever enchanted land becomes tapped, remove an ore counter from this Aura.",
    "When the last ore counter is removed from this Aura, destroy enchanted land and this Aura deals 2 damage to that land's controller.",
];

fn land() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Mined Land")
        .card_types(vec![CardType::Land])
        .build()
}

#[test]
fn orcish_mine_keeps_both_removal_triggers_and_the_last_counter_payload() {
    let definition = parse_oracle_card_definition("Orcish Mine");
    assert_eq!(canonical_compiled_lines(&definition), ORACLE);
    let debug = format!("{definition:#?}");
    assert!(debug.contains("OrTrigger"), "{debug}");
    assert!(debug.contains("CounterRemovedFromTrigger"), "{debug}");
    assert!(debug.contains("last: true"), "{debug}");
    assert!(
        debug.contains("Named(\n") && debug.contains("\"ore\""),
        "{debug}"
    );
}

#[test]
fn removing_the_last_ore_counter_destroys_the_land_and_damages_its_controller() {
    let definition = parse_oracle_card_definition("Orcish Mine");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mined_land = game.create_object_from_definition(&land(), bob, Zone::Battlefield);
    let mined_land_stable = game.object(mined_land).expect("land exists").stable_id;
    assert!(
        game.attach_object_to_target(source, crate::object::AttachmentTarget::Object(mined_land),)
    );
    game.object_mut(source)
        .expect("Mine exists")
        .counters
        .clear();
    game.add_counters(source, CounterType::Named("ore"), 1);

    let (_, event) = game
        .remove_counters(
            source,
            CounterType::Named("ore"),
            1,
            Some(source),
            Some(alice),
        )
        .expect("the last ore counter should be removed");
    let triggers = crate::triggers::check_triggers(&game, &event);
    assert_eq!(triggers.len(), 1, "{triggers:#?}");
    let mut queue = crate::triggers::TriggerQueue::new();
    queue.add(triggers.into_iter().next().expect("one trigger"));
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("last-counter trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("last-counter trigger should resolve");

    let current_land = game
        .find_object_by_stable_id(mined_land_stable)
        .expect("destroyed land remains represented in its graveyard");
    assert_eq!(
        game.object(current_land).map(|object| object.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(game.player(bob).expect("Bob").life, 18);
}
