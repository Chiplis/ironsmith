//! cf8/p07: "counter <spell> if it has the same mana value as the discarded /
//! revealed card": a resolution-time comparison (CR 608.2c), not a targeting
//! restriction. Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

const FIXTURE: &str = include_str!("../../../fixtures/same_mana_value_counters.json.fixture");

#[test]
fn hisoka_counters_only_a_spell_matching_the_discarded_cards_mana_value() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Hisoka, Minamo Sensei");
    assert_eq!(row["oracle_id"], "b69701d6-8e46-4184-991b-9f5e95a5e16d");
    for definition in support::definitions(row) {
        let activated = support::activated(&definition);
        let debug = format!("{:?}", support::activated_effects(activated.last().unwrap()));
        assert!(debug.contains("SameManaValueAsTagged"), "{debug}");
        assert!(debug.contains("Counter"), "{debug}");
        assert!(debug.contains("Discard"), "{debug}");
    }
}

#[test]
fn counterbalance_compares_the_cast_spell_with_the_revealed_card() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Counterbalance");
    assert_eq!(row["oracle_id"], "088dfd02-c0e0-41d1-9923-efe023fb2ad1");
    for definition in support::definitions(row) {
        let triggers = support::triggered(&definition);
        assert_eq!(triggers.len(), 1);
        let debug = format!("{:?}", support::triggered_effects(triggers[0]));
        assert!(debug.contains("SameManaValueAsTagged"), "{debug}");
        assert!(debug.contains("Reveal"), "{debug}");
    }
}
