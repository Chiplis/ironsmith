#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const LIEUTENANT_LINE: &str = "Lieutenant — As long as you control your commander, this creature gets +2/+2 and creatures you control have vigilance.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn angelic_field_marshal_keeps_one_labeled_conditional_static_group() {
    let definition = parse_oracle_card_definition("Angelic Field Marshal");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec!["Flying".to_string(), LIEUTENANT_LINE.to_string()]
    );
    let debug = format!("{:#?}", definition.abilities);
    assert!(
        debug.contains("SourceLineStaticGroup") && debug.contains("Lieutenant"),
        "the two continuous effects need source-line and label provenance: {debug}"
    );
    assert_eq!(
        debug.matches("you control your commander").count(),
        2,
        "{debug}"
    );
}

#[test]
fn angelic_field_marshal_condition_controls_both_bonus_and_vigilance() {
    let definition = parse_oracle_card_definition("Angelic Field Marshal");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let marshal = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally = game.create_object_from_definition(&creature("Ally"), alice, Zone::Battlefield);
    let enemy = game.create_object_from_definition(&creature("Enemy"), bob, Zone::Battlefield);
    let base_power = game.calculated_power(marshal).expect("Marshal has power");
    let base_toughness = game
        .calculated_toughness(marshal)
        .expect("Marshal has toughness");
    assert!(!game.object_has_static_ability_id(ally, StaticAbilityId::Vigilance));

    let commander =
        game.create_object_from_definition(&creature("Commander"), alice, Zone::Battlefield);
    game.set_as_commander(commander, alice);
    assert_eq!(game.calculated_power(marshal), Some(base_power + 2));
    assert_eq!(game.calculated_toughness(marshal), Some(base_toughness + 2));
    assert!(game.object_has_static_ability_id(ally, StaticAbilityId::Vigilance));
    assert!(game.object_has_static_ability_id(marshal, StaticAbilityId::Vigilance));
    assert!(
        !game.object_has_static_ability_id(enemy, StaticAbilityId::Vigilance),
        "the shared condition must not widen the affected creature filter"
    );

    game.move_object_by_effect(commander, Zone::Command)
        .expect("commander can leave the battlefield");
    assert_eq!(game.calculated_power(marshal), Some(base_power));
    assert!(!game.object_has_static_ability_id(ally, StaticAbilityId::Vigilance));
}
