#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn record_cast(game: &mut crate::GameState, caster: PlayerId, name: &str, card_type: CardType) {
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build();
    let spell = game.create_object_from_definition(&spell_definition, caster, Zone::Stack);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

fn arclight_combat_triggers(
    game: &crate::GameState,
    arclight: ObjectId,
    player: PlayerId,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfCombatEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    );
    crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == arclight)
        .collect()
}

#[test]
fn arclight_phoenix_requires_three_matching_spells_cast_by_its_controller() {
    let definition = parse_oracle_card_definition("Arclight Phoenix");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    let arclight = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    let stable_id = game
        .object(arclight)
        .expect("Arclight should exist")
        .stable_id;

    record_cast(&mut game, alice, "Alice Instant", CardType::Instant);
    record_cast(&mut game, alice, "Alice Sorcery", CardType::Sorcery);
    record_cast(&mut game, alice, "Alice Creature", CardType::Creature);
    record_cast(&mut game, bob, "Bob Instant", CardType::Instant);
    assert!(
        arclight_combat_triggers(&game, arclight, alice).is_empty(),
        "two matching spells plus an off-type spell and an opponent's spell must not satisfy three"
    );

    record_cast(&mut game, alice, "Alice Second Instant", CardType::Instant);
    let matching = arclight_combat_triggers(&game, arclight, alice);
    assert_eq!(
        matching.len(),
        1,
        "the third matching spell should enable Arclight"
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Arclight Phoenix's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Arclight Phoenix's trigger should resolve");

    let returned = game
        .find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .expect("Arclight should remain identifiable after moving zones");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(returned), alice);
}
