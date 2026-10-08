//! UNVALIDATED implementation-first coverage: land animations that keep the
//! land type ("... creatures that are still lands", CR 205.1b; layers 4/5/7b,
//! CR 613.1d-e, 613.4b) and the Llama creature type (CR 205.3m).
#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn still_a_land_animations_compile() {
    let rows = common::rows(include_str!("../../../fixtures/animation_still_lands.json.fixture"));
    assert_eq!(rows.len(), 4);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let (pt, extra) = match name {
            "Rude Awakening" => ("2/2", None),
            "Restless Prairie" => ("3/3", Some("Llama")),
            "Hunting Wilds" => ("3/3", Some("haste")),
            "Primal Adversary" => ("3/3", Some("Wolf")),
            other => panic!("unexpected cohort member {other}"),
        };
        for definition in common::definitions(row) {
            let lines = common::rendered(&definition);
            assert!(lines.contains(pt), "{name}: {lines}");
            assert!(lines.contains("still"), "{name}: the land type is kept: {lines}");
            if let Some(extra) = extra {
                assert!(lines.contains(extra), "{name}: {lines}");
            }
        }
    }
}

#[test]
fn llama_is_a_creature_type() {
    assert!(ironsmith::Subtype::all_creature_types().contains(&ironsmith::Subtype::Llama));
}
