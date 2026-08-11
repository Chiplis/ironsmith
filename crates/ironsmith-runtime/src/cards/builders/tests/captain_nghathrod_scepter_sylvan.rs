#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn exact_public_lines(name: &str) -> (CardDefinition, Vec<String>) {
    let definition = parse_oracle_card_definition(name);
    let expected = oracle_text_by_name()
        .get(name)
        .expect("regression card should be present")
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    (definition, expected)
}

#[test]
fn captain_nghathrod_keeps_library_to_graveyard_history_in_its_target() {
    let (definition, expected) = exact_public_lines("Captain N'ghathrod");
    assert_eq!(canonical_compiled_lines(&definition), expected);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("entered_graveyard_from_library_this_turn: true")
            && debug.contains("zone: Some(\n")
            && debug.contains("Graveyard")
            && debug.contains("owner: Some(\n")
            && debug.contains("Opponent"),
        "Captain's target must retain current graveyard, opponent ownership, and library-origin history: {debug}"
    );
    assert!(
        !debug.contains("any_of: [\n                            ObjectFilter")
            || !debug.contains("zone: Some(\n                                    Library"),
        "the origin library must not become an alternative current zone: {debug}"
    );
}

#[test]
fn scepter_of_empires_keeps_one_target_and_exact_named_artifact_replacement() {
    let (definition, expected) = exact_public_lines("Scepter of Empires");
    assert_eq!(canonical_compiled_lines(&definition), expected);

    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("condition_after_replacement: true")
            && debug.contains("name: Some(\n")
            && debug.contains("crown of empires")
            && debug.contains("throne of empires")
            && debug.contains("amount: Fixed(\n")
            && debug.contains("3"),
        "Scepter must retain its typed 1-to-3 replacement and both named-artifact gates: {debug}"
    );
}

#[test]
fn sylvan_offering_keeps_both_opponent_choices_and_shared_token_controllers() {
    let (definition, expected) = exact_public_lines("Sylvan Offering");
    assert_eq!(canonical_compiled_lines(&definition), expected);

    let debug = format!("{definition:#?}");
    assert_eq!(
        debug.matches("ChoosePlayerEffect").count(),
        2,
        "each authored line must retain its own opponent choice: {debug}"
    );
    assert_eq!(
        debug.matches("controller: TaggedPlayer").count(),
        2,
        "the chosen opponent must control one Treefolk and one Elf Warrior batch: {debug}"
    );
    assert!(
        debug.matches("controller: You").count() >= 2
            && debug.contains("subtypes: [\n                                                                Treefolk")
            && debug.contains("Elf")
            && debug.contains("Warrior"),
        "the caster and chosen opponent must receive the exact paired token families: {debug}"
    );
}
