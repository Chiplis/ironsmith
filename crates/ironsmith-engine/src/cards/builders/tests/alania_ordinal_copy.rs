#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::oracle_text_by_name;
use super::*;

const ORACLE: &str = "Whenever you cast a spell, if it's the first instant spell, the first sorcery spell, or the first Otter spell other than Alania you've cast this turn, you may have target opponent draw a card. If you do, copy that spell. You may choose new targets for the copy.";

#[test]
fn alania_preserves_ordinal_union_optional_opponent_draw_and_copy_followup() {
    let parse_input = format!(
        "Mana cost: {{3}}{{U}}{{R}}\nType: Legendary Creature — Otter Wizard\nFirst printed set: Bloomburrow\nPower/Toughness: 3/5\n{ORACLE}"
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), "Alania, Divergent Storm")
        .parse_text(parse_input)
        .expect("the authoritative metadata-backed payload should parse");
    let debug = format!("{:#?}", definition.abilities);

    assert_eq!(debug.matches("ValueComparison").count(), 3, "{debug}");
    assert_eq!(
        debug.matches("before_triggering_spell: true").count(),
        3,
        "{debug}"
    );
    assert!(debug.contains("Instant"), "{debug}");
    assert!(debug.contains("Sorcery"), "{debug}");
    assert!(debug.contains("Otter"), "{debug}");
    assert!(debug.contains("MayEffect"), "{debug}");
    assert!(debug.contains("CopySpellEffect"), "{debug}");
    let oracle = oracle_text_by_name()
        .get("Alania, Divergent Storm")
        .expect("Alania should be present in cards.json");
    assert_eq!(oracle, ORACLE);
    assert_eq!(canonical_compiled_lines(&definition), vec![oracle.clone()]);
}
