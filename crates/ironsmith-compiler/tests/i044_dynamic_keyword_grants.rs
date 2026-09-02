use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::types::CardType;

const EXECUTABLE_MARKER_BACKED_KEYWORDS: &[&str] = &[
    "afterlife 2",
    "fabricate 2",
    "prowess",
    "storm",
    "toxic 2",
    "battle cry",
    "dethrone",
    "evolve",
    "ingest",
    "mentor",
    "training",
    "riot",
    "renown 2",
    "modular 2",
    "graft 2",
    "soulbond",
    "soulshift 2",
    "outlast {1}{W}",
    "unearth {1}{B}",
    "eternalize {2}{B}",
    "ninjutsu {1}{U}",
    "extort",
    "sunburst",
    "firebending 2",
    "fading 2",
    "vanishing 2",
    "rampage 2",
    "bushido 2",
    "annihilator 2",
];

#[test]
fn gameplay_keyword_grants_compile_to_executable_typed_abilities() {
    for keyword in EXECUTABLE_MARKER_BACKED_KEYWORDS {
        let printed = CardDefinitionBuilder::new(CardId::new(), "Printed Keyword Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(*keyword)
            .unwrap_or_else(|error| panic!("printed {keyword} should compile: {error}"));
        let printed_debug = format!("{:#?}", printed.abilities);
        assert!(
            !printed_debug.is_empty()
                && (printed_debug.contains("Triggered(")
                    || printed_debug.contains("Activated(")
                    || printed_debug.contains("EntersWithCounters")
                    || printed_debug.contains("GrantObjectAbilityForFilter")),
            "printed {keyword} must include executable semantics: {printed_debug}"
        );

        let granted = CardDefinitionBuilder::new(CardId::new(), "Dynamic Keyword Grant Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(format!(
                "Target creature gains {keyword} until end of turn."
            ))
            .unwrap_or_else(|error| panic!("dynamic {keyword} grant should compile: {error}"));
        let granted_debug = format!("{:#?}", granted.spell_effect);
        assert!(
            granted_debug.contains("AddAbilityGeneric") || granted_debug.contains("AddAbility("),
            "dynamic {keyword} must lower to an executable ability modification: {granted_debug}"
        );
    }
}
