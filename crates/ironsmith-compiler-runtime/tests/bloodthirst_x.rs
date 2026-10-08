//! cf8/p07: "Bloodthirst X" (CR 702.54c) — enters with X +1/+1 counters, where
//! X is the total damage your opponents have been dealt this turn.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

const FIXTURE: &str = include_str!("../../../fixtures/bloodthirst_x.json.fixture");

#[test]
fn petrified_wood_kin_enters_with_opponent_damage_counters() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Petrified Wood-Kin");
    assert_eq!(row["oracle_id"], "3bc9649c-4350-4c6a-8441-dc76a5f833af");
    for definition in support::definitions(row) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("DamageDealtToPlayersThisTurn(Opponent)"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{debug}");
        assert!(!debug.contains("KeywordFallbackText"), "{debug}");
    }
}
