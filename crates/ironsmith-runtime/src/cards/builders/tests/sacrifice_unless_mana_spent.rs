#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn etb_sacrifice_unless_mana_spent_cards_keep_the_authored_surface() {
    let cases = [
        ("Crypt Champion", "{R}"),
        ("Patagia Viper", "{U}"),
        ("Azorius Herald", "{U}"),
        ("Plaxmanta", "{G}"),
        ("Squealing Devil", "{B}"),
        ("Court Hussar", "{W}"),
    ];

    for (name, symbol) in cases {
        let definition = parse_oracle_card_definition(name);
        let lines = canonical_compiled_lines(&definition);
        let expected = format!(
            "When this creature enters, sacrifice it unless {symbol} was spent to cast it."
        );
        assert_eq!(
            lines.last().map(String::as_str),
            Some(expected.as_str()),
            "{name}: {lines:#?}"
        );
    }
}
