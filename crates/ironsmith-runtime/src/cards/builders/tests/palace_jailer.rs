#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};

const EXILE_LINE: &str = "When this creature enters, exile target creature an opponent controls until an opponent becomes the monarch.";

#[test]
fn palace_jailer_parser_backed_exile_ends_only_when_an_opponent_becomes_monarch() {
    let definition = parse_oracle_card_definition("Palace Jailer");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == EXILE_LINE),
        "Palace Jailer's compiled text must preserve the monarch-event duration: {rendered:#?}"
    );

    let exile_trigger = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| format!("{:#?}", triggered.effects).contains("OpponentBecomesMonarch"))
        .expect("Palace Jailer should compile a monarch-event exile trigger");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let jailer = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let victim_definition = CardDefinitionBuilder::new(CardId::new(), "Jailer's Prisoner")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let victim = game.create_object_from_definition(&victim_definition, bob, Zone::Battlefield);

    let mut ctx = ExecutionContext::new_default(jailer, alice)
        .with_targets(vec![ResolvedTarget::Object(victim)]);
    for effect in exile_trigger.effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Palace Jailer's exile trigger should resolve");
    }
    assert!(game.exile.iter().any(|id| {
        game.object(*id)
            .is_some_and(|object| object.name == "Jailer's Prisoner")
    }));

    game.move_object_by_effect(jailer, Zone::Graveyard);
    game.set_monarch(Some(alice));
    assert!(
        game.exile.iter().any(|id| game
            .object(*id)
            .is_some_and(|object| object.name == "Jailer's Prisoner")),
        "neither Palace Jailer leaving nor its controller becoming monarch ends the duration"
    );

    game.set_monarch(Some(bob));
    assert!(game.exile.is_empty());
    let returned = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Jailer's Prisoner")
        })
        .expect("the prisoner should return when an opponent becomes monarch");
    assert_eq!(game.controller_of_id(returned), Some(bob));
}
