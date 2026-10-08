//! cf8/p07: counter-removal references.
//! - "remove them" / "remove them all" / "remove all of them from it" after a
//!   counter-threshold condition removes every counter of the named kind from
//!   the condition's holder (CR 122.8 counters are removed by kind).
//! - "remove any number of [kind] counters from X" is an up-to removal bounded
//!   by every counter of that kind X (or the "from among" set) has.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::cards::CardDefinition;
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::{RemoveCountersEffect, RemoveUpToAnyCountersEffect, RemoveUpToCountersEffect};
use ironsmith::target::ChooseSpec;

const FIXTURE: &str = include_str!("../../../fixtures/counter_removal_references.json.fixture");

fn all_effects(definition: &CardDefinition) -> Vec<Effect> {
    let mut all = Vec::new();
    for triggered in support::triggered(definition) {
        all.extend(support::triggered_effects(triggered));
    }
    for activated in support::activated(definition) {
        all.extend(support::activated_effects(activated));
    }
    all
}

#[test]
fn them_removals_bind_the_threshold_counter_kind() {
    let rows = support::rows(FIXTURE);
    for (name, oracle_id, kind) in [
        ("Heirloom Mirror // Inherited Fiend", "b33390dd-f2f8-4059-83a9-45c0edccd5e4", "ritual"),
        ("Smoldering Egg // Ashmouth Dragon", "7493d78c-26c4-402e-bec5-6449773d0344", "ember"),
        ("Strixhaven Stadium", "2ed0c6fc-b8b4-47da-a04b-1d995bdeecb0", "point"),
        ("Lightning Coils", "bbb16f23-fdb1-444b-bef8-ce3e177c2678", "charge"),
    ] {
        let row = support::row(&rows, name);
        assert_eq!(row["oracle_id"], oracle_id);
        for definition in support::definitions(row) {
            let removals = support::find::<RemoveCountersEffect>(&all_effects(&definition));
            assert_eq!(removals.len(), 1, "{name}: {removals:?}");
            let removal = &removals[0];
            assert_eq!(removal.counter_type.description(), kind, "{name}");
            // Every counter of that kind on the same holder, not a fixed count.
            match removal.count.unhinted() {
                Value::CountersOnSource(counter_type) => {
                    assert_eq!(*counter_type, removal.counter_type, "{name}")
                }
                Value::CountersOn(_, Some(counter_type)) => {
                    assert_eq!(*counter_type, removal.counter_type, "{name}");
                }
                other => panic!("{name}: unexpected removal amount {other:?}"),
            }
            assert!(!removal.target.is_target(), "{name}: removal is not targeted");
        }
    }
}

#[test]
fn any_number_removals_are_bounded_by_the_holders_counters() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Galloping Lizrog");
    assert_eq!(row["oracle_id"], "3561cd52-961f-4098-9287-58ee43970f0f");
    for definition in support::definitions(row) {
        let removals = support::find::<RemoveUpToCountersEffect>(&all_effects(&definition));
        assert_eq!(removals.len(), 1);
        let removal = &removals[0];
        assert_eq!(removal.counter_type, ironsmith::CounterType::PlusOnePlusOne);
        assert!(matches!(removal.target.unhinted(), ChooseSpec::All(_)), "from among creatures you control");
        let Value::CountersOn(holder, Some(counter_type)) = removal.max_count.unhinted() else {
            panic!("any-number bound: {:?}", removal.max_count);
        };
        assert_eq!(*counter_type, ironsmith::CounterType::PlusOnePlusOne);
        assert_eq!(holder.unhinted(), removal.target.unhinted());
    }

    let row = support::row(&rows, "Rhys, the Evermore");
    assert_eq!(row["oracle_id"], "9f013b5d-fff0-4c50-8621-158b6dc34834");
    for definition in support::definitions(row) {
        let removals = support::find::<RemoveUpToAnyCountersEffect>(&all_effects(&definition));
        assert_eq!(removals.len(), 1);
        let removal = &removals[0];
        assert!(removal.up_to, "any number includes zero");
        assert!(removal.target.is_target());
        let Value::CountersOn(holder, None) = removal.max_count.unhinted() else {
            panic!("any-number bound over all kinds: {:?}", removal.max_count);
        };
        assert_eq!(holder.unhinted(), removal.target.unhinted());
    }
}
