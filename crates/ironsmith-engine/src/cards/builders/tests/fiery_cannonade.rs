#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn durable_creature(name: &str, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build()
}

#[test]
fn fiery_cannonade_keeps_the_non_pirate_filter_in_text_and_gameplay() {
    let name = "Fiery Cannonade";
    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let definition = parse_oracle_card_definition(name);
    let compiled = unprocessed_compiled_lines(&definition);
    let debug = format!("{definition:#?}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(
        debug.contains("excluded_subtypes: [\n") && debug.contains("Pirate"),
        "{debug}"
    );
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert_eq!(similarity, 1.0, "compiled={compiled:?}");
    assert!(!mismatch, "compiled={compiled:?}");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let friendly_pirate = game.create_object_from_definition(
        &durable_creature("Friendly Pirate", Subtype::Pirate),
        alice,
        Zone::Battlefield,
    );
    let opposing_pirate = game.create_object_from_definition(
        &durable_creature("Opposing Pirate", Subtype::Pirate),
        bob,
        Zone::Battlefield,
    );
    let opposing_human = game.create_object_from_definition(
        &durable_creature("Opposing Human", Subtype::Human),
        bob,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(crate::game_state::StackEntry::new(spell, alice));
    crate::game_loop::resolve_stack_entry(&mut game).expect("Fiery Cannonade should resolve");

    assert_eq!(game.damage_on(friendly_pirate), 0);
    assert_eq!(game.damage_on(opposing_pirate), 0);
    assert_eq!(game.damage_on(opposing_human), 2);
}
