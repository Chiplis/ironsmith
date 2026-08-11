#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn resolve_festering_newt_trigger(
    definition: &CardDefinition,
    control_bogbrew_witch: bool,
) -> (i32, i32) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let newt = game.create_object_from_definition(definition, alice, Zone::Battlefield);
    if control_bogbrew_witch {
        let witch = CardDefinitionBuilder::new(CardId::new(), "Bogbrew Witch")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_definition(&witch, alice, Zone::Battlefield);
    }
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Newt Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(6, 6))
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);

    let newt_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(newt).expect("Festering Newt should exist"),
        &game,
    );
    game.move_object_by_effect(newt, Zone::Graveyard)
        .expect("Festering Newt should die");
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            newt,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(newt_snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(vec![newt_snapshot]);

    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|trigger| trigger.source == newt)
    {
        queue.add(trigger);
    }
    assert_eq!(queue.entries.len(), 1, "Festering Newt should trigger once");

    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Festering Newt's trigger should select the opponent's creature");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Festering Newt's trigger should resolve");

    (
        game.current_power(target)
            .expect("the target should have power"),
        game.current_toughness(target)
            .expect("the target should have toughness"),
    )
}

#[test]
fn festering_newt_uses_only_the_conditionally_replaced_pt_modifier() {
    let definition = parse_oracle_card_definition("Festering Newt");
    assert_eq!(
        unprocessed_compiled_lines(&definition).join("\n"),
        "When this creature dies, target creature an opponent controls gets -1/-1 until end of turn. That creature gets -4/-4 instead if you control a creature named Bogbrew Witch."
    );
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Festering Newt should have a dies trigger");
    let [segment] = triggered.effects.segments.as_slice() else {
        panic!("Festering Newt should lower to one replacement segment: {triggered:#?}");
    };
    let [replacement] = segment.self_replacements.as_slice() else {
        panic!("Festering Newt should have one typed self-replacement: {segment:#?}");
    };
    assert!(
        replacement.condition_after_replacement,
        "the authored `instead if` order must survive lowering: {replacement:#?}"
    );
    assert!(
        format!("{:?}", replacement.condition)
            .to_ascii_lowercase()
            .contains("bogbrew witch"),
        "Festering Newt must retain its named-creature condition: {replacement:#?}"
    );

    assert_eq!(
        resolve_festering_newt_trigger(&definition, false),
        (5, 5),
        "without Bogbrew Witch, only the default -1/-1 branch should apply"
    );
    assert_eq!(
        resolve_festering_newt_trigger(&definition, true),
        (2, 2),
        "with Bogbrew Witch, -4/-4 must replace rather than stack with -1/-1"
    );
}
