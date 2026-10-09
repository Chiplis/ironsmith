//! UNVALIDATED implementation-first coverage: the -1/-0 counter type
//! (CR 122.1a: a P/T-modifying counter).
use ironsmith::effects::PutCountersEffect;
use ironsmith::object::CounterType;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn jabaris_influence_puts_a_minus_one_minus_zero_counter() {
    assert_eq!(CounterType::MinusOneMinusZero.pt_delta(), Some((-1, 0)));
    let rows = common::rows(include_str!("../../../fixtures/minus_one_minus_zero_counters.json.fixture"));
    let row = common::row(&rows, "Jabari's Influence");
    for definition in common::definitions(row) {
        let effects = common::all_effects(&definition);
        assert!(effects.iter().any(|effect| effect.downcast_ref::<PutCountersEffect>()
            .is_some_and(|put| put.counter_type == CounterType::MinusOneMinusZero)));
    }
}
