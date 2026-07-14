use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

const ASCEND_SPELLS: [&str; 5] = [
    "Expel from Orazca",
    "Golden Demise",
    "Pride of Conquerors",
    "Secrets of the Golden City",
    "Vona's Hunger",
];

#[test]
pub(super) fn frozen_ascend_spells_use_the_typed_resolution_action() {
    for name in ASCEND_SPELLS {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("Ascend card should be a spell")
            .flattened_default_effects();
        assert!(
            effects.first().is_some_and(|effect| effect
                .downcast_ref::<crate::effects::AscendEffect>()
                .is_some()),
            "{name} should begin with the typed Ascend effect: {effects:#?}"
        );

        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert!(compiled.starts_with("Ascend."), "{name}: {compiled}");
        assert!(
            !compiled.to_ascii_lowercase().contains("emblem"),
            "the city's blessing is a designation, not a synthetic emblem: {compiled}"
        );
    }
}

#[test]
pub(super) fn vonas_hunger_keeps_each_opponents_rounded_up_half_choice() {
    let definition = parse_oracle_card_definition("Vona's Hunger");
    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("HalfRoundedDown"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");

    let compiled = canonical_compiled_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "Each opponent sacrifices half the creatures they control of their choice, rounded up"
        ),
        "{compiled}"
    );
}

#[test]
pub(super) fn distributed_damage_cards_keep_one_or_two_targets_surface() {
    for name in [
        "Arc Mage",
        "Chandra's Pyrohelix",
        "Fire",
        "Forked Bolt",
        "Twin Bolt",
    ] {
        assert_oracle_card_parses_strict(name);
        let compiled = canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n");
        assert!(
            compiled.contains("damage divided as you choose among one or two targets"),
            "{name}: {compiled}"
        );
        assert!(!compiled.contains("any targets"), "{name}: {compiled}");
    }
}

#[test]
pub(super) fn investigate_twice_cards_keep_keyword_action_surface() {
    for name in [
        "Armed with Proof",
        "Detective's Satchel",
        "Ezrim, Agency Chief",
        "Wavesifter",
    ] {
        assert_oracle_card_parses_strict(name);
        let compiled = canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n");
        assert!(compiled.contains("investigate twice"), "{name}: {compiled}");
        assert!(!compiled.contains("you investigates"), "{name}: {compiled}");
        assert!(!compiled.contains("2 times"), "{name}: {compiled}");
    }
}
