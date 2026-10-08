//! UNVALIDATED implementation-first coverage: threshold static "this creature
//! gets +1/+1, is black, and has \"<activated ability>\"" (CR 613.1e layer-5
//! color, layer-6 ability grant, layer-7c pump), all under the threshold
//! condition.
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn possessed_cycle_keeps_pump_color_and_granted_ability() {
    let rows = common::rows(include_str!("../../../fixtures/possessed_threshold_grants.json.fixture"));
    assert_eq!(rows.len(), 4);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let color = match name {
            "Possessed Aven" => "blue",
            "Possessed Barbarian" => "red",
            "Possessed Centaur" => "green",
            "Possessed Nomad" => "white",
            other => panic!("unexpected cohort member {other}"),
        };
        for definition in common::definitions(row) {
            let ids = common::static_ids(&definition);
            assert!(ids.contains(&StaticAbilityId::Anthem), "{name}: {ids:?}");
            assert!(ids.contains(&StaticAbilityId::SetColors), "{name}: {ids:?}");
            let lines = common::rendered(&definition);
            assert!(lines.contains(&format!("Destroy target {color} creature")), "{name}: {lines}");
            assert!(lines.contains("seven or more cards"), "{name}: {lines}");
        }
    }
}
