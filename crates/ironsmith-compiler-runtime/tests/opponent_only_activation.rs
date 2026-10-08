//! cf8 p05: "Only your opponents may activate this ability [and only as a
//! sorcery]." (CR 602.1, 602.5d): a typed activator-relative timing.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;
use ironsmith::ability::{AbilityKind, ActivationTiming};

#[test]
fn detention_vortex_is_activated_only_by_opponents_at_sorcery_speed() {
    for definition in support::definitions(&support::row("opponent_only_activation", "Detention Vortex")) {
        let activated = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .unwrap();
        assert_eq!(activated.timing, ActivationTiming::SorcerySpeedByOpponents);
        assert!(activated.allows_any_player_to_activate());
        assert!(
            !activated
                .additional_restrictions
                .iter()
                .any(|restriction| restriction.contains("opponents")),
            "the typed timing owns the permission; no stringly restriction"
        );
    }
}

#[test]
fn opponent_only_timing_sentences_are_typed() {
    for (text, timing) in [
        ("{1}: Draw a card. Only your opponents may activate this ability.", ActivationTiming::AnyTimeByOpponents),
        (
            "{1}: Draw a card. Only your opponents may activate this ability and only as a sorcery.",
            ActivationTiming::SorcerySpeedByOpponents,
        ),
    ] {
        let source = format!("Mana cost: {{1}}\nType: Artifact\n{text}");
        let definition =
            ironsmith_compiler_runtime::compile_to_runtime_definition("Opponent activation probe", &source, false)
                .unwrap();
        assert!(definition.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Activated(activated) if activated.timing == timing
        )), "{text}");
    }
}
