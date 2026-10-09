//! UNVALIDATED implementation-first coverage (cf8 p09): "If it's a land or
//! double-faced card" (CR 712.1) reads a typed double-faced filter fact on
//! the union arm it qualifies.
#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn invasion_of_pyrulea_checks_land_or_double_faced() {
    let rows = common::rows(include_str!("../../../fixtures/double_faced_predicates.json.fixture"));
    for definition in common::definitions(common::row(&rows, "Invasion of Pyrulea")) {
        let debug = format!("{:?}", common::all_effects(&definition));
        assert!(debug.contains("double_faced: true"), "{debug}");
    }
}
