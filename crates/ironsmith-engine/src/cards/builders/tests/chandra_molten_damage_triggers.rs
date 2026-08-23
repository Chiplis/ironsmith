#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn queue_matching_triggers(
    game: &mut crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> usize {
    let matching = crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    let count = matching.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    if count > 0 {
        let mut decisions = crate::decision::SelectFirstDecisionMaker;
        crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut decisions)
            .expect("matching trigger should go on the stack");
    }
    count
}

#[test]
fn chandra_grouped_loyalty_removal_uses_the_removed_amount_and_only_her_counters() {
    let definition = parse_oracle_card_definition("Chandra, Fire Artisan");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let chandra = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Other Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let removal_event = |object, amount| {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::other::MarkersChangedEvent::removed(
                crate::object::CounterType::Loyalty,
                object,
                amount,
                None,
                None,
            ),
            crate::provenance::ProvNodeId::default(),
        )
    };

    let unrelated = removal_event(other, 3);
    assert_eq!(queue_matching_triggers(&mut game, chandra, &unrelated), 0);

    let matching = removal_event(chandra, 3);
    assert_eq!(queue_matching_triggers(&mut game, chandra, &matching), 1);
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Chandra should deal the exported removed-counter amount");
    assert_eq!(game.life_total(bob), 17);
}

fn damage_event(source: ObjectId, player: PlayerId, combat: bool) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(player),
            1,
            combat,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn molten_setup() -> (
    crate::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let definition = parse_oracle_card_definition("Molten Lavamancer");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    let molten = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let pinger_definition = CardDefinitionBuilder::new(CardId::new(), "Pinger")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let alice_pinger =
        game.create_object_from_definition(&pinger_definition, alice, Zone::Battlefield);
    let bob_pinger = game.create_object_from_definition(&pinger_definition, bob, Zone::Battlefield);
    (game, alice, bob, molten, alice_pinger, bob_pinger)
}

#[test]
fn molten_lavamancer_keeps_every_damage_trigger_qualifier() {
    let definition = parse_oracle_card_definition("Molten Lavamancer");
    assert_eq!(
        unprocessed_compiled_lines(&definition).join("\n"),
        "Prowess\nWhenever a source you control deals noncombat damage to one or more of your opponents during your turn, you create a 1/1 red Elemental creature token. This ability triggers only once each turn."
    );
    let trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::DealsDamageTrigger>()
                .map(|matcher| (triggered, matcher)),
            _ => None,
        })
        .expect("Molten Lavamancer should use the typed damage trigger");
    assert!(trigger.1.noncombat_only);
    assert_eq!(trigger.1.damaged_player, Some(PlayerFilter::Opponent));
    assert!(trigger.1.damaged_player_one_or_more);
    assert_eq!(trigger.1.during_turn, Some(PlayerFilter::You));
    assert_eq!(trigger.1.filter.controller, Some(PlayerFilter::You));
    assert!(matches!(
        trigger.0.intervening_if,
        Some(
            crate::ConditionExpr::MaxTimesEachTurn(1)
                | crate::ConditionExpr::DoThisMaxTimesEachTurn(1)
        )
    ));
}

#[test]
fn molten_lavamancer_triggers_only_for_qualified_damage_during_its_controllers_turn() {
    let (mut game, alice, bob, molten, alice_pinger, _bob_pinger) = molten_setup();
    let matching = damage_event(alice_pinger, bob, false);
    assert_eq!(queue_matching_triggers(&mut game, molten, &matching), 1);
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Molten Lavamancer should create its Elemental token");
    let elemental_tokens = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| {
            object.kind == crate::object::ObjectKind::Token
                && object.subtypes.contains(&Subtype::Elemental)
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(elemental_tokens, 1);
    assert_eq!(queue_matching_triggers(&mut game, molten, &matching), 0);

    for reason in [
        "combat damage",
        "damage to its controller",
        "an opponent-controlled source",
        "damage outside its controller's turn",
    ] {
        let (
            mut near_miss_game,
            near_alice,
            near_bob,
            near_molten,
            near_alice_pinger,
            near_bob_pinger,
        ) = molten_setup();
        let event = match reason {
            "combat damage" => damage_event(near_alice_pinger, near_bob, true),
            "damage to its controller" => damage_event(near_alice_pinger, near_alice, false),
            "an opponent-controlled source" => damage_event(near_bob_pinger, near_bob, false),
            "damage outside its controller's turn" => {
                near_miss_game.turn.active_player = near_bob;
                damage_event(near_alice_pinger, near_bob, false)
            }
            _ => unreachable!(),
        };
        assert_eq!(
            queue_matching_triggers(&mut near_miss_game, near_molten, &event),
            0,
            "{reason} must not trigger Molten Lavamancer"
        );
    }
}
