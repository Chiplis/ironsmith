//! "A deck can have up to N cards named X." (Nazgûl, Seven Dwarves): a
//! bounded exception to the CR 100.2a four-copy rule, kept as a typed
//! deck-construction rule (no game-time effect).
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};

#[path = "p02_line_families/compile.rs"]
mod compile;

const CARDS: &[(&str, &str, &str, u32)] = &[
    (
        "Nazgûl",
        "Mana cost: {2}{B}\nType: Creature — Wraith Knight\nPower/Toughness: 1/2\nDeathtouch\nWhen this creature enters, the Ring tempts you.\nWhenever the Ring tempts you, put a +1/+1 counter on each Wraith you control.\nA deck can have up to nine cards named Nazgûl.",
        "nine",
        9,
    ),
    (
        "Seven Dwarves",
        "Mana cost: {1}{R}\nType: Creature — Dwarf\nPower/Toughness: 2/2\nThis creature gets +1/+1 for each other creature named Seven Dwarves you control.\nA deck can have up to seven cards named Seven Dwarves.",
        "seven",
        7,
    ),
];

#[test]
fn bounded_named_deck_rule_is_a_deck_construction_rule_on_both_routes() {
    for &(name, text, count, maximum) in CARDS {
        for definition in compile::compile_both(name, text) {
            let rules = compile::statics(&definition, StaticAbilityId::DeckConstructionRuleText);
            assert_eq!(rules.len(), 1, "{name}");
            let display = rules[0].display().to_ascii_lowercase();
            assert_eq!(rules[0].deck_copy_limit(), Some(Some(maximum)), "{name}");
            assert!(
                display.starts_with(&format!("a deck can have up to {count} cards named")),
                "{name}: {display}"
            );
        }
    }
}

#[test]
fn copy_limit_survives_compilation_and_label_only_loading() {
    for (quantity, maximum) in [
        ("up to eleven", Some(11)),
        ("up to twenty-one", Some(21)),
        ("up to twenty one", Some(21)),
        ("up to one hundred", Some(100)),
        ("any number of", None),
    ] {
        let rule = format!("A deck can have {quantity} cards named Limit Probe.");
        let label_only = StaticAbility::from_compiler_model_parts(
            Some(StaticAbilityId::DeckConstructionRuleText),
            rule.clone(),
        )
        .unwrap();
        assert_eq!(label_only.deck_copy_limit(), Some(maximum), "{rule}");
        let text = format!("Type: Artifact\n{rule}");
        for definition in compile::compile_both("Limit Probe", &text) {
            let rules = compile::statics(&definition, StaticAbilityId::DeckConstructionRuleText);
            assert_eq!(rules.len(), 1, "{rule}");
            assert_eq!(rules[0].deck_copy_limit(), Some(maximum), "{rule}");
        }
    }
}

#[test]
fn incomplete_or_unrelated_limits_do_not_relax_deck_validation() {
    for rule in [
        "A deck can have up to nine.",
        "A deck can have up to nine cards named.",
        "A deck can have up to nine cards named ,.",
        "A deck can have up to nine basic lands.",
        "A deck can have up to zero cards named Limit Probe.",
        "A deck can have any number of cards named.",
        "A deck can have up to twice cards named Limit Probe.",
    ] {
        assert_eq!(
            StaticAbility::deck_construction_rule_text(rule).deck_copy_limit(),
            None,
            "{rule}"
        );
    }
}
