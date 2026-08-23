#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn damage_event(
    source: ObjectId,
    target: crate::events::DamageTarget,
    amount: u32,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            target,
            amount,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn put_matching_trigger_on_stack(
    game: &mut crate::GameState,
    ghyrson: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> usize {
    let matching = crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == ghyrson)
        .collect::<Vec<_>>();
    let count = matching.len();
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in matching {
        queue.add(entry);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, &mut queue)
            .expect("Ghyrson's damage trigger should go on the stack");
    }
    count
}

fn setup() -> (
    crate::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let definition = parse_oracle_card_definition("Ghyrson Starn, Kelermorph");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let ghyrson = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Pinger")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let pinger = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Damage Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 8))
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);
    (game, alice, bob, ghyrson, pinger, target)
}

#[test]
fn ghyrson_keeps_exact_amount_and_damaged_recipient_provenance() {
    let definition = parse_oracle_card_definition("Ghyrson Starn, Kelermorph");
    assert_eq!(
        unprocessed_compiled_lines(&definition).join("\n"),
        "Ward {2}\nThree Autostubs — Whenever another source you control deals exactly 1 damage to a permanent or player, Ghyrson Starn deals 2 damage to that permanent or player."
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::DealsExactDamageToObjectOrPlayerTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Three Autostubs should use the typed exact-damage trigger");
    let debug = format!("{:#?}", triggered.effects);
    assert!(debug.contains("TagTriggeringDamageTargetEffect"), "{debug}");
    assert!(debug.contains("DamagedPlayer"), "{debug}");
}

#[test]
fn ghyrson_hits_the_exact_object_or_player_and_rejects_near_misses() {
    let (mut object_game, _alice, bob, ghyrson, pinger, target) = setup();
    let object_event = damage_event(pinger, crate::events::DamageTarget::Object(target), 1);
    assert_eq!(
        put_matching_trigger_on_stack(&mut object_game, ghyrson, &object_event),
        1
    );
    crate::game_loop::resolve_stack_entry(&mut object_game)
        .expect("Ghyrson should deal 2 to the damaged permanent");
    assert_eq!(object_game.damage_on(target), 2);
    assert_eq!(object_game.life_total(bob), 20);

    let (mut player_game, _alice, bob, ghyrson, pinger, target) = setup();
    let player_event = damage_event(pinger, crate::events::DamageTarget::Player(bob), 1);
    assert_eq!(
        put_matching_trigger_on_stack(&mut player_game, ghyrson, &player_event),
        1
    );
    crate::game_loop::resolve_stack_entry(&mut player_game)
        .expect("Ghyrson should deal 2 to the damaged player");
    assert_eq!(player_game.life_total(bob), 18);
    assert_eq!(player_game.damage_on(target), 0);

    let (mut amount_game, _alice, bob, ghyrson, pinger, _target) = setup();
    let two_damage = damage_event(pinger, crate::events::DamageTarget::Player(bob), 2);
    assert_eq!(
        put_matching_trigger_on_stack(&mut amount_game, ghyrson, &two_damage),
        0,
        "two damage must not satisfy exactly one"
    );

    let (mut self_game, _alice, bob, ghyrson, _pinger, _target) = setup();
    let self_damage = damage_event(ghyrson, crate::events::DamageTarget::Player(bob), 1);
    assert_eq!(
        put_matching_trigger_on_stack(&mut self_game, ghyrson, &self_damage),
        0,
        "the ability source is not another source"
    );
}
