#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "This spell costs {X} less to cast, where X is the greatest power among creatures you control.\nWhenever one or more nontoken creatures you control die, create a green Fungus Dinosaur creature token with base power and toughness each equal to the total power of those creatures.\n{2}, {T}: Double target creature's power until end of turn.";

fn creature(name: &str, power: i32, token: bool) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, power));
    if token {
        builder = builder.token();
    }
    builder.build()
}

fn skullspore_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Skullspore Nexus should retain its one-or-more dies trigger")
}

#[test]
fn skullspore_public_payload_keeps_dynamic_batch_power_and_exact_surface() {
    let definition = parse_oracle_card_definition("The Skullspore Nexus");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let triggered = skullspore_trigger(&definition);
    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .expect("typed zone-change trigger");
    assert_eq!(
        zone_change.count_mode,
        crate::triggers::zone_changes::CountMode::OneOrMore
    );
    assert!(zone_change.object_filter.nontoken);
    assert_eq!(zone_change.object_filter.card_types, [CardType::Creature]);
    assert_eq!(
        zone_change.object_filter.controller,
        Some(PlayerFilter::You)
    );

    let set_pt = triggered
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::SetBasePowerToughnessEffect>())
        .expect("created token should receive typed dynamic base power and toughness");
    for value in [&set_pt.power, &set_pt.toughness] {
        let crate::effect::Value::TotalPower(filter) = value.unhinted() else {
            panic!("Skullspore's dynamic P/T should use total power: {value:#?}");
        };
        assert_eq!(filter.zone, None);
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == ironsmith_core::ZONE_CHANGE_GROUP_TAG
        }));
    }
}

#[test]
fn skullspore_uses_only_matching_nontoken_creature_lki_from_the_death_batch() {
    let definition = parse_oracle_card_definition("The Skullspore Nexus");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let nexus = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let alice_two = game.create_object_from_definition(
        &creature("Alice Two", 2, false),
        alice,
        Zone::Battlefield,
    );
    let alice_three = game.create_object_from_definition(
        &creature("Alice Three", 3, false),
        alice,
        Zone::Battlefield,
    );
    let alice_token = game.create_object_from_definition(
        &creature("Alice Token", 11, true),
        alice,
        Zone::Battlefield,
    );
    let bob_seven = game.create_object_from_definition(
        &creature("Bob Seven", 7, false),
        bob,
        Zone::Battlefield,
    );
    let snapshots = [alice_two, alice_three, alice_token, bob_seven]
        .into_iter()
        .map(|id| {
            crate::snapshot::ObjectSnapshot::from_object(
                game.object(id).expect("batch creature exists"),
                &game,
            )
        })
        .collect::<Vec<_>>();
    let moved = [alice_two, alice_three, alice_token, bob_seven]
        .into_iter()
        .map(|id| {
            game.move_object_by_effect(id, Zone::Graveyard)
                .expect("batch creature should move")
        })
        .collect::<Vec<_>>();
    game.take_pending_trigger_events();
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::batch_with_snapshots(
            moved,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            snapshots.clone(),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(snapshots);
    let triggered = skullspore_trigger(&definition);
    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .expect("typed zone-change trigger");
    let ctx = crate::triggers::TriggerContext::for_source(nexus, alice, &game);
    assert!(
        triggered.trigger.matches(&event, &ctx),
        "the OneOrMore matcher should accept the matching subset: {zone_change:#?}"
    );
    assert_eq!(
        zone_change
            .matching_batch_snapshots(
                event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .expect("batch zone-change event"),
                &ctx,
            )
            .len(),
        2,
        "LKI matching should exclude the token and opponent's creature"
    );
    let entries = crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == nexus)
        .collect::<Vec<_>>();
    assert_eq!(
        entries.len(),
        1,
        "the simultaneous deaths should trigger once"
    );
    let captured = entries[0]
        .tagged_objects
        .get(&crate::tag::TagKey::from(
            ironsmith_core::ZONE_CHANGE_GROUP_TAG,
        ))
        .expect("the matched LKI group should be captured at trigger time");
    assert_eq!(
        captured.len(),
        2,
        "token and opponent's creature are excluded"
    );
    assert_eq!(
        captured
            .iter()
            .filter_map(|snapshot| snapshot.power)
            .sum::<i32>(),
        5
    );

    let mut queue = crate::triggers::TriggerQueue::new();
    queue.add(entries.into_iter().next().expect("one Skullspore trigger"));
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Skullspore's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Skullspore's trigger should resolve");

    let token = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token && object.name == "Fungus Dinosaur"
            })
        })
        .expect("Skullspore should create its Fungus Dinosaur token");
    assert_eq!(game.current_power(token), Some(5));
    assert_eq!(game.current_toughness(token), Some(5));
}
