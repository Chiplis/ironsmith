#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str =
    "Trample\nWhenever Wyleth attacks, draw a card for each Aura and Equipment attached to it.";

fn attachment(name: &str, subtype: Subtype) -> CardDefinition {
    let card_type = if subtype == Subtype::Aura {
        CardType::Enchantment
    } else {
        CardType::Artifact
    };
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .subtypes(vec![subtype])
        .build()
}

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn wyleth_draw(
    definition: &CardDefinition,
) -> (&crate::triggers::AttacksTrigger, &DrawCardsEffect) {
    definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return None;
            };
            let trigger = triggered
                .trigger
                .downcast_ref::<crate::triggers::AttacksTrigger>()?;
            let draw = triggered
                .effects
                .flattened_default_effects()
                .into_iter()
                .find_map(|effect| effect.downcast_ref::<DrawCardsEffect>())?;
            Some((trigger, draw))
        })
        .expect("Wyleth should have one typed attack-and-draw ability")
}

#[test]
fn wyleth_public_payload_keeps_the_shared_attachment_relation_on_both_union_arms() {
    let definition = parse_oracle_card_definition("Wyleth, Soul of Steel");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let (trigger, draw) = wyleth_draw(&definition);
    assert!(trigger.filter.source);
    let crate::effect::Value::Count(filter) = draw.count.unhinted() else {
        panic!("Wyleth's draw count should remain a typed object count: {draw:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [Subtype::Aura])
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [Subtype::Equipment])
    );
    let attachment_tags = filter
        .any_of
        .iter()
        .map(|branch| {
            let [constraint] = branch.tagged_constraints.as_slice() else {
                panic!("each union arm must retain one attachment relation: {branch:#?}");
            };
            assert_eq!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
            );
            constraint.tag.clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        attachment_tags[0], attachment_tags[1],
        "Aura and Equipment must be attached to the same triggering attacker"
    );
    assert_eq!(
        attachment_tags[0].as_str(),
        "__it__",
        "the authored singular `it` reference must remain available to the renderer"
    );
}

#[test]
fn wyleth_draws_only_for_auras_and_equipment_attached_to_the_attacker() {
    let definition = parse_oracle_card_definition("Wyleth, Soul of Steel");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let wyleth = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other_host =
        game.create_object_from_definition(&creature("Other Host"), alice, Zone::Battlefield);

    let attached_aura = game.create_object_from_definition(
        &attachment("Attached Aura", Subtype::Aura),
        alice,
        Zone::Battlefield,
    );
    let attached_equipment = game.create_object_from_definition(
        &attachment("Attached Equipment", Subtype::Equipment),
        alice,
        Zone::Battlefield,
    );
    let other_aura = game.create_object_from_definition(
        &attachment("Other Aura", Subtype::Aura),
        alice,
        Zone::Battlefield,
    );
    let _unattached_equipment = game.create_object_from_definition(
        &attachment("Unattached Equipment", Subtype::Equipment),
        alice,
        Zone::Battlefield,
    );
    assert!(game.attach_object_to_target(
        attached_aura,
        crate::object::AttachmentTarget::Object(wyleth),
    ));
    assert!(game.attach_object_to_target(
        attached_equipment,
        crate::object::AttachmentTarget::Object(wyleth),
    ));
    assert!(game.attach_object_to_target(
        other_aura,
        crate::object::AttachmentTarget::Object(other_host),
    ));

    for index in 0..4 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), format!("Library Card {index}"))
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Library,
        );
    }
    let hand_before = game.player(alice).expect("Alice exists").hand.len();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(
            wyleth,
            crate::events::combat::AttackEventTarget::Player(bob),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == wyleth)
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1);
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Wyleth's attack trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Wyleth's attack trigger should resolve");

    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        hand_before + 2,
        "an Aura on another permanent and an unattached Equipment must not be counted"
    );
}
