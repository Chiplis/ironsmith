//! UNVALIDATED implementation-first coverage (cf8 p09): "Put one into your
//! hand and exile the rest." partitions the revealed cards between hand and
//! exile.
use ironsmith::effects::MoveToZoneEffect;
use ironsmith::zone::Zone;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn eye_of_yawgmoth_exiles_the_rest() {
    let rows = common::rows(include_str!("../../../fixtures/hand_and_exile_partition.json.fixture"));
    for definition in common::definitions(common::row(&rows, "Eye of Yawgmoth")) {
        let effects = common::all_effects(&definition);
        let zones = effects
            .iter()
            .filter_map(|effect| effect.downcast_ref::<MoveToZoneEffect>())
            .map(|moved| moved.zone)
            .collect::<Vec<_>>();
        assert!(zones.contains(&Zone::Hand), "{zones:?}");
        assert!(zones.contains(&Zone::Exile), "{zones:?}");
    }
}
