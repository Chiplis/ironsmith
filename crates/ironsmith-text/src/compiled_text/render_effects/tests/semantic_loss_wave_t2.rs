//! Semantic-loss fix wave (2026-10-21), package T2: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn t2_parse(name: &str, card_types: Vec<CardType>, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(text)
        .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"))
}

fn t2_debug(definition: &crate::cards::CardDefinition) -> String {
    format!("{definition:?}")
}

// --- Activated abilities of *sources* are taxed in every zone -------------

#[test]
fn source_named_ability_tax_is_not_limited_to_the_battlefield() {
    let definition = t2_parse(
        "Skyseer's Chariot",
        vec![CardType::Artifact],
        "As this artifact enters, choose a nonland card name.\nActivated abilities of sources with the chosen name cost {2} more to activate.",
    );
    let debug = t2_debug(&definition);
    assert!(
        debug.contains("ActivatedAbilityCostIncrease { filter: ObjectFilter { zone: None"),
        "a source-named tax must cover cycling/graveyard abilities too: {debug}"
    );
}

#[test]
fn permanent_type_ability_tax_still_covers_only_the_battlefield() {
    let definition = t2_parse(
        "Taxing Relic",
        vec![CardType::Artifact],
        "Activated abilities of creatures cost {2} more to activate.",
    );
    let debug = t2_debug(&definition);
    assert!(
        debug.contains("ActivatedAbilityCostIncrease { filter: ObjectFilter { zone: Some(Battlefield)"),
        "{debug}"
    );
}

#[test]
fn source_named_ability_tax_renders_sources() {
    let definition = t2_parse(
        "Anointed Peacekeeper",
        vec![CardType::Creature],
        "As this creature enters, choose a card name.\nActivated abilities of sources with the chosen name cost {2} more to activate unless they're mana abilities.",
    );
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("Activated abilities of sources with"), "{text}");
    assert!(!text.contains("permanents with"), "{text}");
}

// --- Mourner's Shield: one chosen source, limited by the relative clause ----

#[test]
fn prevention_from_a_source_of_your_choice_with_a_relative_clause_keeps_the_choice() {
    let definition = t2_parse(
        "Mourner's Shield",
        vec![CardType::Artifact],
        "Imprint — When this artifact enters, you may exile target card from a graveyard.\n{2}, {T}: Prevent all damage that would be dealt this turn by a source of your choice that shares a color with the exiled card.",
    );
    let debug = t2_debug(&definition);
    assert!(
        debug.contains("source_of_your_choice: true"),
        "the shield must protect against one chosen source, not every sharing source: {debug}"
    );
    assert!(debug.contains("Color"), "the shares-a-color limit must remain: {debug}");
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        text.contains("by a source of your choice that shares a color with the exiled card"),
        "{text}"
    );
}

// --- Jadzi: "you may cast it" stays optional --------------------------------

#[test]
fn may_cast_revealed_card_by_paying_alternative_cost_is_optional() {
    let definition = t2_parse(
        "Jadzi Probe",
        vec![CardType::Creature],
        "Whenever you cast an instant or sorcery spell, reveal the top card of your library. If it's a nonland card, you may cast it by paying {1} rather than paying its mana cost.",
    );
    let debug = t2_debug(&definition);
    let may = debug.find("MayEffect").expect("the cast must be wrapped in an optional choice");
    let cast = debug.find("CastTaggedEffect").expect("cast effect");
    assert!(may < cast, "MayEffect must enclose the CastTaggedEffect: {debug}");
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(text.contains("you may cast it by paying {1}"), "{text}");
}

// --- Mutated Cultist: per-counter cost reduction ----------------------------

#[test]
fn next_spell_reduction_scales_by_counters_removed_this_way() {
    let definition = t2_parse(
        "Mutated Cultist",
        vec![CardType::Creature],
        "When you cast this spell, remove all counters from up to one target permanent or opponent. The next spell you cast this turn costs {1} less to cast for each counter removed this way.",
    );
    let debug = t2_debug(&definition);
    assert!(
        debug.contains("generic_reduction: Some("),
        "the reduction must be a counted value, not a flat {{1}}: {debug}"
    );
    let reduction = &debug[debug.find("generic_reduction: Some(").unwrap()..];
    assert!(reduction.contains("Removed"), "{reduction}");
}

// --- Kate Stewart: X counts time counters, not permanents -------------------

#[test]
fn where_x_counters_among_permanents_inside_if_you_do_counts_counters() {
    let definition = t2_parse(
        "Kate Probe",
        vec![CardType::Creature],
        "Whenever this creature attacks, you may pay {8}. If you do, attacking creatures get +X/+X until end of turn, where X is the number of time counters among permanents you control.",
    );
    let debug = t2_debug(&definition);
    assert!(
        debug.contains("CountersOn(All(") && debug.contains("Some(Time)"),
        "X must be the time-counter total across your permanents: {debug}"
    );
    let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        text.contains("the number of time counters among permanents you control"),
        "{text}"
    );
}

// --- Vastwood Hydra: distribute as many counters as it had ------------------

#[test]
fn distribute_a_number_equal_to_counters_on_source_is_not_one_counter() {
    let definition = t2_parse(
        "Vastwood Hydra",
        vec![CardType::Creature],
        "When this creature dies, you may distribute a number of +1/+1 counters equal to the number of +1/+1 counters on this creature among any number of creatures you control.",
    );
    let debug = t2_debug(&definition);
    assert!(debug.contains("distributed: true"), "{debug}");
    assert!(
        !debug.contains("counter_type: PlusOnePlusOne, amount: Fixed(1)"),
        "the article must not become the amount: {debug}"
    );
    assert!(
        debug.contains("counter_type: PlusOnePlusOne, amount: CountersOn(")
            || debug.contains("counter_type: PlusOnePlusOne, amount: CountersOnSource("),
        "{debug}"
    );
}
