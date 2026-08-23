#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Menace\nWhenever Don & Raph attack, the next noncreature spell you cast this turn has affinity for artifacts.";

#[test]
fn don_and_raph_grant_affinity_only_to_the_next_noncreature_spell() {
    let definition = parse_oracle_card_definition("Don & Raph, Hard Science");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Don & Raph should have an attack trigger");
    let debug = format!("{:#?}", triggered.effects);
    assert!(debug.contains("GrantNextSpellAbilityEffect"), "{debug}");
    assert!(debug.contains("AffinityForArtifacts"), "{debug}");
    assert!(
        debug.contains("excluded_card_types") && debug.contains("Creature"),
        "{debug}"
    );
    assert!(
        debug.contains("cast_by: Some") && debug.contains("You"),
        "{debug}"
    );
}
