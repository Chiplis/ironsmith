use super::shard_16::parse_oracle_card_definition;
use super::*;

fn compiled_lower(name: &str) -> (CardDefinition, String) {
    let definition = parse_oracle_card_definition(name);
    let compiled = compiled_text_lines(&definition)
        .join("\n")
        .to_ascii_lowercase();
    (definition, compiled)
}

#[test]
fn counter_scaled_anthems_keep_the_counter_source_or_counter_set() {
    for (name, expected, rejected) in [
        (
            "Boon of the Spirit Realm",
            "for each blessing counter on this enchantment",
            "for each enchantment",
        ),
        (
            "Gavel of the Righteous",
            "for each counter on this equipment",
            "for each equipment",
        ),
        (
            "Gleam of Authority",
            "for each +1/+1 counter on other creatures you control",
            "for each other creature you control",
        ),
    ] {
        let (definition, compiled) = compiled_lower(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("CountersOnSource")
                || debug.contains("CountersAmong")
                || debug.contains("CountersOn("),
            "{name} must retain a structured counter-derived anthem: {debug}"
        );
        assert!(compiled.contains(expected), "{name}: {compiled}");
        assert!(!compiled.contains(rejected), "{name}: {compiled}");
    }
}

#[test]
fn draw_for_each_creature_with_a_counter_counts_creatures() {
    for (name, expected) in [
        (
            "Armorcraft Judge",
            "for each creature you control with a +1/+1 counter on it",
        ),
        (
            "Gamma Grotesque",
            "for each creature you control with a counter on it",
        ),
        (
            "Inspiring Call",
            "for each creature you control with a +1/+1 counter on it",
        ),
    ] {
        let (definition, compiled) = compiled_lower(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("Count(") && debug.contains("with_counter: Some"),
            "{name} must lower to a counted counter-filtered creature set: {debug}"
        );
        assert!(
            !debug.contains("CountersOnSource"),
            "{name} must not count counters on the source: {debug}"
        );
        assert!(compiled.contains(expected), "{name}: {compiled}");
    }
}

#[test]
fn number_of_counters_supports_named_sources_and_filtered_sets() {
    let (instrument, instrument_compiled) = compiled_lower("Instrument of the Bards");
    let instrument_debug = format!("{instrument:#?}");
    assert!(
        instrument_debug.contains("CountersOn("),
        "{instrument_debug}"
    );
    assert!(
        instrument_compiled.contains(
            "mana value equal to the number of harmony counters on instrument of the bards"
        ),
        "{instrument_compiled}"
    );

    let (toph, toph_compiled) = compiled_lower("Toph, the Blind Bandit");
    let toph_debug = format!("{toph:#?}");
    assert!(
        toph_debug.contains("CountersOn(") && toph_debug.contains("All("),
        "Toph must total counters across a filtered land set: {toph_debug}"
    );
    assert!(
        toph_compiled.contains("number of +1/+1 counters on lands you control"),
        "{toph_compiled}"
    );
}

#[test]
fn scalar_counter_counts_survive_repeat_and_payment_lowering() {
    for name in ["Quarantine Field", "Smokestack"] {
        let (definition, compiled) = compiled_lower(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("RepeatEffectsEffect") && debug.contains("CountersOn"),
            "{name} must repeat its effect once per source counter: {debug}"
        );
        assert!(compiled.contains("for each"), "{name}: {compiled}");
    }

    let (rogue, rogue_compiled) = compiled_lower("Rogue Skycaptain");
    let rogue_debug = format!("{rogue:#?}");
    assert!(
        rogue_debug.contains("PayManaEffect")
            && rogue_debug.contains("x_value: Some")
            && rogue_debug.contains("CountersOn"),
        "Rogue Skycaptain must retain the counter-scaled aggregate payment: {rogue_debug}"
    );
    assert!(
        rogue_compiled.contains("pay {2} for each wage counter"),
        "{rogue_compiled}"
    );
}

#[test]
fn token_characteristic_values_retain_source_counter_counts() {
    for (name, counter_name) in [("Gutter Grime", "slime"), ("Saproling Burst", "fade")] {
        let (definition, compiled) = compiled_lower(name);
        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("CountersOn(") || debug.contains("CountersOnSource"),
            "{name} token characteristics must retain their source counter value: {debug}"
        );
        assert!(
            compiled.contains(&format!("number of {counter_name} counters")),
            "{name}: {compiled}"
        );
    }
}
