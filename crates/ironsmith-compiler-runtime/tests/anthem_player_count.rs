//! UNVALIDATED implementation-first coverage: "gets +1/+0 for each opponent
//! you have" scales by a live player count (CR 613.4c, CR 102.2 opponents).
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn blazing_sunsteel_scales_by_opponents() {
    let rows = common::rows(include_str!("../../../fixtures/anthem_player_count.json.fixture"));
    let row = common::row(&rows, "Blazing Sunsteel");
    for definition in common::definitions(row) {
        let ids = common::static_ids(&definition);
        assert!(ids.contains(&StaticAbilityId::Anthem), "{ids:?}");
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("CountPlayers(Opponent)"), "the scale is the live opponent count: {debug}");
        let lines = common::rendered(&definition);
        assert!(lines.contains("opponent"), "{lines}");
        assert!(lines.contains("Equip {4}"), "{lines}");
    }
}
