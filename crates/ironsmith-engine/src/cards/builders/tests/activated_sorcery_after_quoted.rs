#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn trailing_sorcery_speed_after_quoted_effect_is_typed_and_rendered() {
    let names = [
        "Expendable Lackey",
        "Dragonbroods' Relic",
        "Foul Roads",
        "Country Roads",
        "Rocky Roads",
        "Reef Roads",
        "Wild Roads",
    ];

    for name in names {
        let definition = parse_oracle_card_definition(name);
        let sorcery_speed = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated)
                    if activated.timing == crate::ability::ActivationTiming::SorcerySpeed =>
                {
                    Some(activated)
                }
                _ => None,
            })
            .count();
        assert_eq!(
            sorcery_speed, 1,
            "{name} should have exactly one sorcery-speed activated ability"
        );

        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert!(
            compiled.contains("Activate only as a sorcery."),
            "{name} did not render the typed restriction: {compiled}"
        );
    }

    let expendable = parse_oracle_card_definition("Expendable Lackey");
    assert_eq!(
        canonical_compiled_lines(&expendable).join("\n"),
        "{1}{U}, Exile this card from your graveyard: Create a 1/1 blue Fish creature token with \"This token can't be blocked.\" Activate only as a sorcery."
    );
}
