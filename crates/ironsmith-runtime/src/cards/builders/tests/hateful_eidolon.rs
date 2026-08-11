#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn aura_definition(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .build()
}

#[test]
fn hateful_eidolon_draws_for_each_aura_you_controlled_attached_to_the_dead_creature() {
    let definition = parse_oracle_card_definition("Hateful Eidolon");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let eidolon = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let victim_definition = CardDefinitionBuilder::new(CardId::new(), "Enchanted Victim")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let victim = game.create_object_from_definition(&victim_definition, bob, Zone::Battlefield);

    for (name, controller) in [
        ("Alice Aura One", alice),
        ("Alice Aura Two", alice),
        ("Bob Aura", bob),
    ] {
        let aura = game.create_object_from_definition(
            &aura_definition(name),
            controller,
            Zone::Battlefield,
        );
        assert!(
            game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(victim),)
        );
    }

    game.move_object_by_sba(victim, Zone::Graveyard)
        .expect("the enchanted creature should die");
    assert!(
        crate::rules::state_based::apply_state_based_actions(&mut game),
        "the unattached Auras should then be put into their owners' graveyards"
    );

    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert_eq!(
        queue
            .entries
            .iter()
            .filter(|entry| entry.source == eidolon)
            .count(),
        1,
        "Hateful Eidolon should trigger once from the real LTB event"
    );

    let draw_card = CardDefinitionBuilder::new(CardId::new(), "Eidolon Draw")
        .card_types(vec![CardType::Instant])
        .build();
    for _ in 0..2 {
        game.create_object_from_definition(&draw_card, alice, Zone::Library);
    }

    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Hateful Eidolon's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Hateful Eidolon's trigger should resolve");

    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        2,
        "only the two Auras Alice controlled and that were attached in LKI should be counted"
    );
}
