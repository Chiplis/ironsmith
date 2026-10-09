//! cf8/p07: "discards another card at random" is one more random discard.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::DiscardEffect;

const FIXTURE: &str = include_str!("../../../fixtures/discard_another_card.json.fixture");

#[test]
fn flay_discards_twice_at_random_the_second_unless_paid() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Flay");
    assert_eq!(row["oracle_id"], "987b74f4-02cb-4d47-94e5-d58a3c8e92e4");
    for definition in support::definitions(row) {
        let all = support::spell_effects(&definition);
        let discards = support::find::<DiscardEffect>(&all);
        assert_eq!(discards.len(), 2);
        for discard in &discards {
            assert_eq!(discard.count, Value::Fixed(1));
            assert!(discard.random);
        }
        let debug = format!("{all:?}");
        assert!(debug.contains("Unless"), "the second discard is avoidable by paying {{1}}: {debug}");
    }
}
