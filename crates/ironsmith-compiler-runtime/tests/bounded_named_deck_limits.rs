//! "A deck can have up to N cards named X." (Nazgûl, Seven Dwarves): a
//! bounded exception to the CR 100.2a four-copy rule, kept as a typed
//! deck-construction rule (no game-time effect). Source-authored, unrun.
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p02_line_families/compile.rs"]
mod compile;

const CARDS: &[(&str, &str, &str)] = &[
    (
        "Nazgûl",
        "Mana cost: {2}{B}\nType: Creature — Wraith Knight\nPower/Toughness: 1/2\nDeathtouch\nWhen this creature enters, the Ring tempts you.\nWhenever the Ring tempts you, put a +1/+1 counter on each Wraith you control.\nA deck can have up to nine cards named Nazgûl.",
        "nine",
    ),
    (
        "Seven Dwarves",
        "Mana cost: {1}{R}\nType: Creature — Dwarf\nPower/Toughness: 2/2\nThis creature gets +1/+1 for each other creature named Seven Dwarves you control.\nA deck can have up to seven cards named Seven Dwarves.",
        "seven",
    ),
];

#[test]
fn bounded_named_deck_rule_is_a_deck_construction_rule_on_both_routes() {
    for &(name, text, count) in CARDS {
        for definition in compile::compile_both(name, text) {
            let rules = compile::statics(&definition, StaticAbilityId::DeckConstructionRuleText);
            assert_eq!(rules.len(), 1, "{name}");
            let display = rules[0].display().to_ascii_lowercase();
            // Deck validation (wasm pregame) reads the limit from this text.
            assert!(
                display.starts_with(&format!("a deck can have up to {count} cards named")),
                "{name}: {display}"
            );
        }
    }
}
