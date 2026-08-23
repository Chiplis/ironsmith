#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_exact_oracle(name: &str, definition: &CardDefinition, expected: &str) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        expected,
        "{name} must keep its authoritative Oracle surface: {definition:#?}"
    );
}

fn effect_tree_contains<T: 'static>(effect: &crate::effect::Effect) -> bool {
    if effect.downcast_ref::<T>().is_some() {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= effect_tree_contains::<T>(child));
    found
}

fn effect_tree_has_each_non_army_damage(effect: &crate::effect::Effect) -> bool {
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
        && let ChooseSpec::Object(filter) | ChooseSpec::All(filter) = damage.target.unhinted()
        && filter
            .excluded_subtypes
            .contains(&crate::types::Subtype::Army)
        && filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Each)
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| found |= effect_tree_has_each_non_army_damage(child));
    found
}

#[test]
fn summons_of_saruman_keeps_the_milled_set_optional_free_cast() {
    let definition = parse_oracle_card_definition("Summons of Saruman");
    assert_exact_oracle(
        "Summons of Saruman",
        &definition,
        "Amass Orcs X. Mill X cards. You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost.\nFlashback—{3}{U}{R}, Exile X cards from your graveyard.",
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Summons should retain a spell program");
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_contains::<crate::effects::AmassEffect>)),
        "the first instruction must remain executable amass: {program:#?}"
    );
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_contains::<crate::effects::MillEffect>)),
        "the milled collection must have an executable producer: {program:#?}"
    );
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_contains::<crate::effects::ChooseObjectsEffect>)),
        "the optional single card from the milled set must be represented as a choice: {program:#?}"
    );
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_contains::<crate::effects::CastTaggedEffect>)),
        "the chosen milled card must feed the free cast: {program:#?}"
    );
}

#[test]
fn thunderfoot_baloth_keeps_one_lieutenant_source_line() {
    let definition = parse_oracle_card_definition("Thunderfoot Baloth");
    assert_exact_oracle(
        "Thunderfoot Baloth",
        &definition,
        "Trample\nLieutenant — As long as you control your commander, this creature gets +2/+2 and other creatures you control get +2/+2 and have trample.",
    );

    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("SourceLineStaticGroup"), "{debug}");
    assert!(debug.contains("Lieutenant"), "{debug}");
    assert_eq!(
        debug.matches("you control your commander").count(),
        3,
        "all three executable static members must retain the same condition: {debug}"
    );
}

#[test]
fn syr_konrad_keeps_all_three_disjunctive_graveyard_events() {
    let definition = parse_oracle_card_definition("Syr Konrad, the Grim");
    assert_exact_oracle(
        "Syr Konrad, the Grim",
        &definition,
        "Whenever another creature dies, or a creature card is put into a graveyard from anywhere other than the battlefield, or a creature card leaves your graveyard, Syr Konrad deals 1 damage to each opponent.\n{1}{B}: Each player mills a card.",
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Syr Konrad should retain its damage trigger");
    let debug = format!("{:#?}", triggered.trigger);
    assert!(debug.contains("OrTrigger"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("Battlefield"), "{debug}");
}

#[test]
fn widespread_brutality_uses_the_amassed_army_against_each_non_army() {
    let definition = parse_oracle_card_definition("Widespread Brutality");
    assert_exact_oracle(
        "Widespread Brutality",
        &definition,
        "Amass Zombies 2, then the Army you amassed deals damage equal to its power to each non-Army creature.",
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Widespread Brutality should retain a spell program");
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_contains::<crate::effects::AmassEffect>)),
        "the producer must retain executable amass: {program:#?}"
    );
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_contains::<crate::effects::ExecuteWithSourceEffect>)),
        "the amassed Army must remain the executable damage source: {program:#?}"
    );
    assert!(
        program.segments.iter().any(|segment| segment
            .default_effects
            .iter()
            .any(effect_tree_has_each_non_army_damage)),
        "damage must reach every non-Army creature: {program:#?}"
    );
}
