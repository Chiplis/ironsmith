#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ATTACK_TRIGGER_LINE: &str = "Whenever an opponent attacks one or more planeswalkers you control, put a loyalty counter on each planeswalker you control.";

fn attack_event(
    attacker: ObjectId,
    target: crate::events::combat::AttackEventTarget,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(attacker, target),
        crate::provenance::ProvNodeId::default(),
    )
}

fn mila_triggers(
    game: &crate::game_state::GameState,
    mila: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == mila)
        .collect()
}

#[test]
fn mila_only_triggers_when_an_opponent_attacks_your_planeswalker() {
    let definition = parse_oracle_card_definition("Mila, Crafty Companion");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == ATTACK_TRIGGER_LINE),
        "Mila must retain the planeswalker-only attack target: {rendered:#?}"
    );
    let attack_matcher = definition.abilities.iter().find_map(|ability| {
        let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
            return None;
        };
        triggered
            .trigger
            .downcast_ref::<crate::triggers::PlayerAttacksOneOrMoreTrigger>()
    });
    let attack_matcher =
        attack_matcher.expect("Mila must compile a typed player-attack trigger matcher");
    assert_eq!(attack_matcher.attacker, PlayerFilter::Opponent);
    assert_eq!(
        attack_matcher.target,
        ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You),
        "Mila must retain a planeswalker-only target restriction"
    );

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
    let mila = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let walker_definition = CardDefinitionBuilder::new(CardId::new(), "Test Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .build();
    let first_walker =
        game.create_object_from_definition(&walker_definition, alice, Zone::Battlefield);
    let second_walker =
        game.create_object_from_definition(&walker_definition, alice, Zone::Battlefield);
    let other_walker =
        game.create_object_from_definition(&walker_definition, charlie, Zone::Battlefield);

    let attacker_definition = CardDefinitionBuilder::new(CardId::new(), "Test Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bob_attacker =
        game.create_object_from_definition(&attacker_definition, bob, Zone::Battlefield);
    let second_bob_attacker =
        game.create_object_from_definition(&attacker_definition, bob, Zone::Battlefield);
    let alice_attacker =
        game.create_object_from_definition(&attacker_definition, alice, Zone::Battlefield);

    let battle_definition = CardDefinitionBuilder::new(CardId::new(), "Test Battle")
        .card_types(vec![CardType::Battle])
        .defense(3)
        .build();
    let battle = game.create_object_from_definition(&battle_definition, alice, Zone::Battlefield);

    for (label, event) in [
        (
            "Mila's controller",
            attack_event(
                bob_attacker,
                crate::events::combat::AttackEventTarget::Player(alice),
            ),
        ),
        (
            "another player",
            attack_event(
                bob_attacker,
                crate::events::combat::AttackEventTarget::Player(charlie),
            ),
        ),
        (
            "a Battle",
            attack_event(
                bob_attacker,
                crate::events::combat::AttackEventTarget::Battle(battle),
            ),
        ),
        (
            "another player's planeswalker",
            attack_event(
                bob_attacker,
                crate::events::combat::AttackEventTarget::Planeswalker(other_walker),
            ),
        ),
        (
            "your planeswalker by your own creature",
            attack_event(
                alice_attacker,
                crate::events::combat::AttackEventTarget::Planeswalker(first_walker),
            ),
        ),
    ] {
        assert!(
            mila_triggers(&game, mila, &event).is_empty(),
            "Mila must not trigger for an attack against {label}"
        );
    }

    let first_before = game.counter_count(first_walker, CounterType::Loyalty);
    let second_before = game.counter_count(second_walker, CounterType::Loyalty);
    let other_before = game.counter_count(other_walker, CounterType::Loyalty);

    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.remove_summoning_sickness(bob_attacker);
    game.remove_summoning_sickness(second_bob_attacker);
    let mut combat = crate::combat_state::CombatState::default();
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[
            crate::decision::AttackerDeclaration {
                creature: bob_attacker,
                target: crate::combat_state::AttackTarget::Planeswalker(first_walker),
            },
            crate::decision::AttackerDeclaration {
                creature: second_bob_attacker,
                target: crate::combat_state::AttackTarget::Planeswalker(second_walker),
            },
        ],
    )
    .expect("opponent planeswalker attacks should be legal");
    let triggers = queue
        .entries
        .iter()
        .filter(|entry| entry.source == mila)
        .count();
    assert_eq!(
        triggers, 1,
        "one opponent attacking two of your planeswalkers must trigger Mila exactly once"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
        .expect("Mila's trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Mila's trigger should resolve");

    assert_eq!(
        game.counter_count(first_walker, CounterType::Loyalty),
        first_before + 1
    );
    assert_eq!(
        game.counter_count(second_walker, CounterType::Loyalty),
        second_before + 1
    );
    assert_eq!(
        game.counter_count(other_walker, CounterType::Loyalty),
        other_before,
        "Mila only adds loyalty counters to planeswalkers its controller controls"
    );
}
