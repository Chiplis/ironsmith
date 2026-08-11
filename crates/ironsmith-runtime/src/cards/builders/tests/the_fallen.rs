#![cfg(ironsmith_runtime_parser_tests)]

use super::*;

const ORACLE: &str = "At the beginning of your upkeep, this creature deals 1 damage to each opponent and planeswalker it has dealt damage to this game.";

#[test]
fn the_fallen_keeps_the_mixed_full_game_damage_recipient_semantics() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "The Fallen")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(ORACLE)
        .expect("The Fallen should parse");
    let debug = format!("{definition:#?}");

    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![ORACLE.to_string()]
    );
    assert!(debug.contains("WasDealtDamageBySourceThisGame"), "{debug}");
    assert!(
        debug.contains("was_dealt_damage_by_source_this_game: true"),
        "{debug}"
    );
    assert_eq!(debug.matches("ForPlayersEffect").count(), 1, "{debug}");
    assert_eq!(debug.matches("ForEachObject").count(), 1, "{debug}");
}
