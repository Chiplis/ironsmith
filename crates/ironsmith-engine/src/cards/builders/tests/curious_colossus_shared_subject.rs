use crate::PlayerId;
use crate::card::PowerToughness;
use crate::cards::builders::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::static_abilities::StaticAbilityId;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn coordinated_continuous_chain_uses_the_original_opponents_creature_set() {
    let source_definition = CardDefinitionBuilder::new(CardId::new(), "Continuous Chain")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "When this creature enters, each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1.",
        )
        .expect("shared-subject continuous chain should parse");
    let flying_creature = CardDefinitionBuilder::new(CardId::new(), "Flying Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text("Flying")
        .expect("flying creature should parse");
    let vanilla_creature = CardDefinitionBuilder::new(CardId::new(), "Vanilla Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 5))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alice_creature =
        game.create_object_from_definition(&flying_creature, alice, Zone::Battlefield);
    let bob_flying = game.create_object_from_definition(&flying_creature, bob, Zone::Battlefield);
    let bob_vanilla = game.create_object_from_definition(&vanilla_creature, bob, Zone::Battlefield);
    let source = game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);

    let enters = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &enters)
        .into_iter()
        .filter(|entry| entry.source == source)
    {
        queue.add(entry);
    }
    assert_eq!(queue.entries.len(), 1, "expected one enter trigger");

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("enter trigger should choose the opponent and go on the stack");
    assert_eq!(
        game.stack.last().map(|entry| entry.targets.as_slice()),
        Some(&[crate::game_state::Target::Player(bob)][..]),
        "the controller qualifier must create one target-opponent slot"
    );
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("enter trigger should resolve");

    for id in [bob_flying, bob_vanilla] {
        let characteristics = game
            .calculated_characteristics(id)
            .expect("affected creature should have calculated characteristics");
        assert_eq!(
            (characteristics.power, characteristics.toughness),
            (Some(1), Some(1)),
            "{characteristics:#?}"
        );
        assert!(
            characteristics.subtypes.contains(&Subtype::Coward),
            "{characteristics:#?}"
        );
        assert!(
            characteristics.abilities.is_empty() && characteristics.static_abilities.is_empty(),
            "{characteristics:#?}"
        );
    }

    let alice_characteristics = game
        .calculated_characteristics(alice_creature)
        .expect("unaffected creature should have calculated characteristics");
    assert_eq!(
        (alice_characteristics.power, alice_characteristics.toughness),
        (Some(3), Some(4)),
        "{alice_characteristics:#?}"
    );
    assert!(
        !alice_characteristics.subtypes.contains(&Subtype::Coward),
        "{alice_characteristics:#?}"
    );
    assert!(
        alice_characteristics
            .static_abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::Flying),
        "{alice_characteristics:#?}"
    );

    let late_creature =
        game.create_object_from_definition(&flying_creature, bob, Zone::Battlefield);
    let late_characteristics = game
        .calculated_characteristics(late_creature)
        .expect("late creature should have calculated characteristics");
    assert_eq!(
        (late_characteristics.power, late_characteristics.toughness),
        (Some(3), Some(4)),
        "{late_characteristics:#?}"
    );
    assert!(
        !late_characteristics.subtypes.contains(&Subtype::Coward),
        "{late_characteristics:#?}"
    );
    assert!(
        late_characteristics
            .static_abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::Flying),
        "{late_characteristics:#?}"
    );
}
