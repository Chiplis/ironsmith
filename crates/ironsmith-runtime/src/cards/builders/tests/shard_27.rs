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
    let compiled_lower = compiled.to_ascii_lowercase();
    assert!(
        compiled_lower.contains("each opponent sacrifices half the creatures they control")
            && compiled_lower.contains("rounded up"),
        "{compiled}"
    );
}

#[test]
pub(super) fn direct_fractional_sacrifices_keep_typed_rounding_surface() {
    let expected = [
        (
            "Curse of the Cabal",
            &["Target player sacrifices half the permanents of their choice, rounded down"][..],
        ),
        (
            "Rakdos the Defiler",
            &[
                "sacrifice half the non-demon permanents you control, rounded up",
                "that player sacrifices half the non-demon permanents they control of their choice, rounded up",
            ][..],
        ),
        (
            "Tectonic Split",
            &["sacrifice half the lands you control, rounded up"][..],
        ),
    ];

    for (name, expected_surfaces) in expected {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let debug = format!("{definition:#?}");
        assert!(debug.contains("HalfRoundedDown"), "{name}: {debug}");

        let compiled = canonical_compiled_lines(&definition).join("\n");
        for expected_surface in expected_surfaces {
            assert!(
                compiled.contains(expected_surface),
                "{name} should preserve its typed fractional sacrifice surface: {compiled}"
            );
        }
    }
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

#[test]
pub(super) fn top_of_library_collection_cards_keep_counts_tags_and_followups() {
    for name in [
        "Hazoret's Undying Fury",
        "Lord of the Void",
        "Magmatic Channeler",
        "Urza, Lord High Artificer",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        let debug = format!("{definition:#?}");

        assert!(
            debug.contains("ExileTopOfLibraryEffect"),
            "{name} must retain a typed top-library exile: {debug}"
        );
        match name {
            "Hazoret's Undying Fury" => {
                assert!(debug.contains("ForEachObject"), "{debug}");
                assert!(
                    compiled.contains(
                        "Shuffle your library, then exile the top four cards. You may cast any number of spells with mana value 5 or less from among them without paying their mana costs"
                    ),
                    "{compiled}\n{debug}"
                );
            }
            "Lord of the Void" => {
                assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
                assert!(debug.contains("ForEachTaggedEffect"), "{debug}");
                assert!(
                    compiled.contains(
                        "exile the top seven cards of that player's library, then put a creature card from among them onto the battlefield under your control"
                    ),
                    "{compiled}\n{debug}"
                );
            }
            "Magmatic Channeler" => {
                assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
                assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
                assert!(
                    compiled.contains(
                        "Exile the top two cards of your library. Choose one of them. Until end of turn, you may play that card"
                    ),
                    "{compiled}"
                );
            }
            "Urza, Lord High Artificer" => {
                assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
                assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
                assert!(
                    compiled.contains(
                        "Shuffle your library, then exile the top card. Until end of turn, you may play that card without paying its mana cost"
                    ),
                    "{compiled}"
                );
            }
            _ => unreachable!(),
        }
    }
}
