use super::*;

fn assert_archetype_compiles_exactly(name: &str, keyword: &str) {
    let oracle = format!(
        "Creatures you control have {keyword}.\nCreatures your opponents control lose {keyword} and can't have or gain {keyword}."
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .parse_text(&oracle)
        .unwrap_or_else(|error| panic!("{name} should parse: {error}"));

    assert_eq!(
        unprocessed_compiled_lines(&definition).join("\n"),
        oracle,
        "{name} should preserve its exact ability prohibition"
    );
    assert!(
        format!("{definition:#?}").contains("LoseAndCantHaveOrGain"),
        "{name} should retain a structural ability-gain prohibition"
    );
}

#[test]
pub(super) fn archetype_of_aggression_compiles_exactly() {
    assert_archetype_compiles_exactly("Archetype of Aggression", "trample");
}

#[test]
pub(super) fn archetype_of_courage_compiles_exactly() {
    assert_archetype_compiles_exactly("Archetype of Courage", "first strike");
}

#[test]
pub(super) fn archetype_of_endurance_compiles_exactly() {
    assert_archetype_compiles_exactly("Archetype of Endurance", "hexproof");
}

#[test]
pub(super) fn archetype_of_finality_compiles_exactly() {
    assert_archetype_compiles_exactly("Archetype of Finality", "deathtouch");
}

#[test]
pub(super) fn archetype_of_imagination_compiles_exactly() {
    assert_archetype_compiles_exactly("Archetype of Imagination", "flying");
}
