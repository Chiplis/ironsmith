//! cf8/p07: "<player> loses all [kind] counters" removes every counter of
//! that kind (or of every kind) from that player. Source-authored, unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effects::{ChooseModeEffect, RemoveUpToAnyCountersEffect};

const FIXTURE: &str = include_str!("../../../fixtures/lose_all_player_counters.json.fixture");

#[test]
fn final_act_strips_every_counter_from_each_opponent() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Final Act");
    assert_eq!(row["oracle_id"], "335a6c6a-030f-45d4-806c-467a22962eed");
    for definition in support::definitions(row) {
        let all = support::spell_effects(&definition);
        let modal = &support::find::<ChooseModeEffect>(&all)[0];
        assert_eq!(modal.modes.len(), 5);
        let last = support::flatten(modal.modes[4].effects.iter());
        let removals = support::find::<RemoveUpToAnyCountersEffect>(&last);
        assert_eq!(removals.len(), 1, "one all-kinds removal per opponent");
        assert!(!removals[0].up_to, "every counter, not up to");
        let debug = format!("{last:?}");
        assert!(debug.contains("IteratedPlayer"), "the iterated opponent: {debug}");
    }
}
