use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

const CASES: &[(&str, usize, &str)] = &[
    (
        "Brightling",
        4,
        "{W}: This creature gains vigilance until end of turn.\n{W}: This creature gains lifelink until end of turn.\n{W}: Return this creature to its owner's hand.\n{1}: This creature gets +1/-1 or -1/+1 until end of turn.",
    ),
    (
        "Death-Hood Cobra",
        2,
        "{1}{G}: This creature gains reach until end of turn.\n{1}{G}: This creature gains deathtouch until end of turn.",
    ),
    (
        "Endling",
        4,
        "{B}: This creature gains menace until end of turn.\n{B}: This creature gains deathtouch until end of turn.\n{B}: This creature gains undying until end of turn.\n{1}: This creature gets +1/-1 or -1/+1 until end of turn.",
    ),
    (
        "Glorifier of Dusk",
        2,
        "Pay 2 life: This creature gains flying until end of turn.\nPay 2 life: This creature gains vigilance until end of turn.",
    ),
    (
        "Golem Artisan",
        2,
        "{2}: Target artifact creature gets +1/+1 until end of turn.\n{2}: Target artifact creature gains your choice of flying, trample, or haste until end of turn.",
    ),
    (
        "Icatian Infantry",
        2,
        "{1}: This creature gains first strike until end of turn.\n{1}: This creature gains banding until end of turn.",
    ),
    (
        "Skyship Stalker",
        3,
        "Flying\n{R}: This creature gets +1/+0 until end of turn.\n{R}: This creature gains first strike until end of turn.\n{R}: This creature gains haste until end of turn.",
    ),
    (
        "Thornling",
        5,
        "{G}: This creature gains haste until end of turn.\n{G}: This creature gains trample until end of turn.\n{G}: This creature gains indestructible until end of turn.\n{1}: This creature gets +1/-1 until end of turn.\n{1}: This creature gets -1/+1 until end of turn.",
    ),
    (
        "Truefire Paladin",
        2,
        "Vigilance\n{R}{W}: This creature gets +2/+0 until end of turn.\n{R}{W}: This creature gains first strike until end of turn.",
    ),
    (
        "Viper, Cruel Conspirator",
        2,
        "{B}: Target creature that's attacking alone gets +1/+1 until end of turn.\n{B}: Target creature that's attacking alone gains your choice of deathtouch or lifelink until end of turn.",
    ),
    (
        "Mantis Engine",
        2,
        "{2}: This creature gains flying until end of turn.\n{2}: This creature gains first strike until end of turn.",
    ),
    (
        "Multiform Wonder",
        2,
        "When this creature enters, you get {E}{E}{E}.\nPay {E}: This creature gains your choice of flying, vigilance, or lifelink until end of turn.\nPay {E}: This creature gets +2/-2 or -2/+2 until end of turn.",
    ),
];

#[test]
fn identical_cost_activated_abilities_keep_authored_boundaries_and_text() {
    for (name, expected_activated_count, expected_text) in CASES {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let activated = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            activated.len(),
            *expected_activated_count,
            "{name} must retain one runtime ability per authored activation"
        );
        assert!(
            activated
                .iter()
                .all(|ability| !ability.effects.flattened_default_effects().is_empty()),
            "{name} must give every authored activation its own nonempty resolution program"
        );
        assert_eq!(
            compiled_text_lines(&definition).join("\n"),
            *expected_text,
            "{name} must not merge adjacent abilities merely because their costs match"
        );
    }
}

#[test]
fn viper_activated_targets_keep_attacking_alone_legality() {
    let definition = parse_oracle_card_definition("Viper, Cruel Conspirator");
    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(activated.len(), 2);
    for ability in activated {
        let debug = format!("{:#?}", ability.effects);
        assert!(debug.contains("attacking_alone: true"), "{debug}");
    }
}

#[test]
fn fixed_pt_alternative_activations_keep_both_resolution_choices() {
    for name in ["Brightling", "Endling", "Multiform Wonder"] {
        let definition = parse_oracle_card_definition(name);
        let choice_ability = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .find(|ability| {
                let debug = format!("{:#?}", ability.effects);
                debug.contains("ChooseModeEffect") && debug.contains("ModifyPowerToughness")
            })
            .unwrap_or_else(|| panic!("{name} must retain its inline P/T alternative as a choice"));
        let debug = format!("{:#?}", choice_ability.effects);

        assert_eq!(
            debug.matches("ModifyPowerToughness").count(),
            2,
            "{name} must lower both P/T alternatives into separate mode effects: {debug}"
        );
        assert!(
            compiled_text_lines(&definition)
                .iter()
                .any(|line| line.contains(" or ") && line.contains("until end of turn")),
            "{name} must render both authored P/T alternatives inline"
        );
    }
}

#[test]
fn identical_cost_keyword_activations_have_disjoint_resolution_programs() {
    let definition = parse_oracle_card_definition("Death-Hood Cobra");
    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect::<Vec<_>>();
    let first = format!("{:#?}", activated[0].effects).to_ascii_lowercase();
    let second = format!("{:#?}", activated[1].effects).to_ascii_lowercase();

    assert!(
        first.contains("reach") && !first.contains("deathtouch"),
        "{first}"
    );
    assert!(
        second.contains("deathtouch") && !second.contains("reach"),
        "{second}"
    );
}
