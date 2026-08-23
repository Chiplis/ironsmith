#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn plaza_of_heroes_preserves_the_dynamic_legendary_color_scope() {
    let definition = parse_oracle_card_definition("Plaza of Heroes");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line == "{T}: Add one mana of any color among legendary permanents you control."
        }),
        "Plaza should retain its dynamic legendary-permanent color scope; got {lines:#?}"
    );
    assert!(
        format!("{:#?}", definition.abilities).contains("AddOneManaOfAnyColorAmongEffect"),
        "Plaza should lower through the typed dynamic filtered mana effect"
    );
}

#[test]
fn mox_amber_preserves_the_dynamic_legendary_creature_and_planeswalker_scope() {
    let definition = parse_oracle_card_definition("Mox Amber");
    let lines = unprocessed_compiled_lines(&definition);

    assert_eq!(
        lines,
        ["{T}: Add one mana of any color among legendary creatures and planeswalkers you control."]
    );
    assert!(
        format!("{:#?}", definition.abilities).contains("AddOneManaOfAnyColorAmongEffect"),
        "Mox Amber should lower through the typed dynamic filtered mana effect"
    );
}

#[test]
fn the_grey_havens_preserves_the_dynamic_graveyard_color_scope() {
    let definition = parse_oracle_card_definition("The Grey Havens");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line
                == "{T}: Add one mana of any color among legendary creature cards in your graveyard."
        }),
        "The Grey Havens should retain its legendary graveyard-card color scope; got {lines:#?}"
    );
    assert!(
        format!("{:#?}", definition.abilities).contains("AddOneManaOfAnyColorAmongEffect"),
        "The Grey Havens should lower through the typed dynamic filtered mana effect"
    );
}

#[test]
fn urzas_workshop_preserves_the_explicit_land_noun_in_its_scaled_mana_count() {
    let definition = parse_oracle_card_definition("Urza's Workshop");
    let lines = unprocessed_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line
                == "Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts."
        }),
        "Urza's Workshop should retain the authored land noun; got {lines:#?}"
    );
}

#[test]
fn hydraulic_helper_preserves_the_negative_nonartifact_cast_restriction() {
    let definition = parse_oracle_card_definition("Hydraulic Helper");
    let lines = canonical_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line == "{T}: Add {U}. This mana can't be spent to cast a nonartifact spell."
        }),
        "Hydraulic Helper should preserve the authored negative restriction; got {lines:#?}"
    );
}

#[test]
fn vhal_preserves_the_hand_origin_cast_restriction() {
    let definition = parse_oracle_card_definition("Vhal, Candlekeep Researcher");
    let lines = canonical_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line
                == "{T}: Add an amount of {C} equal to Vhal's toughness. This mana can't be spent to cast spells from your hand."
        }),
        "Vhal should preserve the typed hand-origin restriction; got {lines:#?}"
    );
}

#[test]
fn mightstone_and_weakstone_preserves_plural_negative_cast_restriction() {
    let definition = parse_oracle_card_definition("The Mightstone and Weakstone");
    let lines = canonical_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line == "{T}: Add {C}{C}. This mana can't be spent to cast nonartifact spells."
        }),
        "The Mightstone and Weakstone should pluralize its two-mana restriction; got {lines:#?}"
    );
}

#[test]
fn steelswarm_operator_preserves_artifact_source_activation_restriction() {
    let definition = parse_oracle_card_definition("Steelswarm Operator");
    let lines = canonical_compiled_lines(&definition);

    assert!(
        lines.iter().any(|line| {
            line
                == "{T}: Add {U}{U}. Spend this mana only to activate abilities of artifact sources."
        }),
        "Steelswarm Operator should preserve the artifact-source activation scope; got {lines:#?}"
    );
}
