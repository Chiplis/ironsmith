#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn attack_event(
    attacker: ObjectId,
    target: crate::events::combat::AttackEventTarget,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(attacker, target),
        crate::provenance::ProvNodeId::default(),
    )
}

fn source_triggers(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn attacker_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Test Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn curse_of_predation_counters_only_the_creature_attacking_enchanted_player() {
    let definition = parse_oracle_card_definition("Curse of Predation");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Enchant player\nWhenever a creature attacks enchanted player, put a +1/+1 counter on it."
    );

    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::AttacksTrigger>(),
            _ => None,
        })
        .expect("Curse of Predation should compile to a typed attack trigger");
    let enchanted = PlayerFilter::TaggedPlayer(crate::tag::TagKey::from("enchanted"));
    assert_eq!(
        trigger
            .filter
            .attacking_player_or_planeswalker_controlled_by,
        Some(enchanted.clone())
    );
    assert_eq!(trigger.filter.targets_only_player, Some(enchanted));
    assert_eq!(trigger.filter.card_types, vec![CardType::Creature]);
    assert!(!trigger.one_or_more, "the Curse triggers once per attacker");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let curse = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(curse, crate::object::AttachmentTarget::Player(bob)));
    let attacker =
        game.create_object_from_definition(&attacker_definition(), charlie, Zone::Battlefield);
    let bystander =
        game.create_object_from_definition(&attacker_definition(), charlie, Zone::Battlefield);

    let entries = source_triggers(
        &game,
        curse,
        &attack_event(
            attacker,
            crate::events::combat::AttackEventTarget::Player(bob),
        ),
    );
    assert_eq!(entries.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Curse of Predation's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Curse of Predation's trigger should resolve");

    assert_eq!(
        game.counter_count(attacker, CounterType::PlusOnePlusOne),
        1,
        "the triggering attacker should get the counter"
    );
    assert_eq!(
        game.counter_count(bystander, CounterType::PlusOnePlusOne),
        0,
        "an unrelated creature should not get a counter"
    );
}

#[test]
fn curse_of_predation_ignores_other_players_and_the_enchanted_players_planeswalkers() {
    let definition = parse_oracle_card_definition("Curse of Predation");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let curse = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(curse, crate::object::AttachmentTarget::Player(bob)));
    let attacker =
        game.create_object_from_definition(&attacker_definition(), charlie, Zone::Battlefield);
    let walker_definition = CardDefinitionBuilder::new(CardId::new(), "Bob Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .build();
    let walker = game.create_object_from_definition(&walker_definition, bob, Zone::Battlefield);

    for (label, event) in [
        (
            "a different player",
            attack_event(
                attacker,
                crate::events::combat::AttackEventTarget::Player(alice),
            ),
        ),
        (
            "a planeswalker controlled by the enchanted player",
            attack_event(
                attacker,
                crate::events::combat::AttackEventTarget::Planeswalker(walker),
            ),
        ),
    ] {
        assert!(
            source_triggers(&game, curse, &event).is_empty(),
            "Curse of Predation must not trigger when the creature attacks {label}"
        );
    }
}
