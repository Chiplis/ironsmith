use super::*;

fn render_card(name: &str, card_type: CardType, oracle: &str) -> String {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(vec![card_type])
        .parse_text(oracle)
        .unwrap_or_else(|error| panic!("{name} should compile: {error}"));
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn land_animations_keep_compact_characteristics_duration_and_type_retention() {
    for (name, card_type, oracle) in [
        (
            "Avalanche Caller",
            CardType::Creature,
            "{2}: Target snow land you control becomes a 4/4 Elemental creature with hexproof and haste until end of turn. It's still a land.",
        ),
        (
            "Silvanus's Invoker",
            CardType::Creature,
            "Conjure Elemental — {8}: Untap target land you control. It becomes an 8/8 Elemental creature with trample and haste until end of turn. It's still a land.",
        ),
        (
            "Spawning Pool",
            CardType::Land,
            "This land enters tapped.\n{T}: Add {B}.\n{1}{B}: This land becomes a 1/1 black Skeleton creature with \"{B}: Regenerate this creature\" until end of turn. It's still a land.",
        ),
    ] {
        assert_eq!(render_card(name, card_type, oracle), oracle, "{name}");
    }
}
