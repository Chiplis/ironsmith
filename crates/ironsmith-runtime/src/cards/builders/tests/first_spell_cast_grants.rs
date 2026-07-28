use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn wild_magic_sorcerer_preserves_first_origin_and_turn_scope() {
    assert_eq!(
        compiled_card_text("Wild-Magic Sorcerer"),
        "The first spell you cast from exile each turn has cascade."
    );
}

#[test]
fn unqualified_and_origin_qualified_grants_remain_distinct() {
    let all_from_exile = CardDefinitionBuilder::new(CardId::new(), "All Exile Grants")
        .parse_text("Spells you cast from exile have cascade.")
        .expect("unqualified exile grant should parse");
    assert_eq!(
        canonical_compiled_lines(&all_from_exile).join("\n"),
        "Spells you cast from exile have cascade."
    );

    let first_each_turn = CardDefinitionBuilder::new(CardId::new(), "First Spell Grant")
        .parse_text("The first spell you cast each turn has cascade.")
        .expect("first-spell grant should parse");
    assert_eq!(
        canonical_compiled_lines(&first_each_turn).join("\n"),
        "The first spell you cast each turn has cascade."
    );
}
