//! UNVALIDATED implementation-first coverage: "for each kind of counter on
//! target permanent, put another counter of that kind on it or remove one
//! from it" is a per-counter-kind choice, not an object iteration.
use ironsmith::effects::ForEachCounterKindPutOrRemoveEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn counter_kind_iteration_reaches_its_own_effect() {
    let rows = common::rows(include_str!("../../../fixtures/counter_kind_put_or_remove.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let effects = common::all_effects(&definition);
            assert!(effects.iter().any(|e| e.downcast_ref::<ForEachCounterKindPutOrRemoveEffect>().is_some()), "{name}");
        }
    }
}
