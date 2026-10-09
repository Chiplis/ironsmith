//! UNVALIDATED implementation-first coverage: "becomes unprepared" removes the
//! prepared designation (the inverse of the Prepared keyword action).
use ironsmith::effects::PrepareEffect;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn unprepare_lowers_to_the_inverse_prepare_effect() {
    let rows = common::rows(include_str!("../../../fixtures/unprepare_effects.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let prepares: Vec<_> = common::all_effects(&definition).iter()
                .filter_map(|effect| effect.downcast_ref::<PrepareEffect>().cloned())
                .collect();
            assert!(prepares.iter().any(|prepare| prepare.unprepare), "{name}");
            if name == "Biblioplex Tomekeeper" {
                assert!(prepares.iter().any(|prepare| !prepare.unprepare), "{name}: both modes");
            }
            let lines = common::rendered(&definition);
            assert!(lines.contains("unprepared"), "{name}: {lines}");
        }
    }
}
