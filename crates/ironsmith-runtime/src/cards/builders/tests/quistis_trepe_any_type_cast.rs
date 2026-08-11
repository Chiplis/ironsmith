#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Blue Magic — When Quistis Trepe enters, you may cast target instant or sorcery card from a graveyard, and mana of any type can be spent to cast that spell. If that spell would be put into a graveyard, exile it instead.";

#[test]
fn quistis_keeps_the_exact_target_cast_permission_and_replacement() {
    let definition = parse_oracle_card_definition("Quistis Trepe");
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);

    let debug = format!("{definition:#?}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
    assert!(debug.contains("mana_spend_mode: AnyType"), "{debug}");
    assert!(
        debug.contains("RegisterFutureZoneReplacementEffect"),
        "{debug}"
    );
    assert!(debug.contains("replacement_zone: Exile"), "{debug}");
}
