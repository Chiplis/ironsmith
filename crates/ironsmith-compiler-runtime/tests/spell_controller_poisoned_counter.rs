//! cf8/p07: "Counter target spell if its controller is poisoned." The copula
//! belongs to the predicate surface; the condition is the target spell's
//! controller having one or more poison counters. Source-authored, unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Condition;
use ironsmith::effects::{ConditionalEffect, CounterEffect};

const FIXTURE: &str = include_str!("../../../fixtures/spell_controller_poisoned_counter.json.fixture");

#[test]
fn corrupted_resolve_counters_only_a_poisoned_controllers_spell() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Corrupted Resolve");
    assert_eq!(row["oracle_id"], "87837a94-05db-48fe-b810-26d8f92cfc58");
    for definition in support::definitions(row) {
        let all = support::spell_effects(&definition);
        let conditionals = support::find::<ConditionalEffect>(&all);
        assert_eq!(conditionals.len(), 1);
        assert_eq!(conditionals[0].condition, Condition::TargetSpellControllerIsPoisoned);
        assert!(conditionals[0].if_false.is_empty());
        let mut guarded = Vec::new();
        for effect in &conditionals[0].if_true {
            support::collect(effect, &mut guarded);
        }
        assert_eq!(support::find::<CounterEffect>(&guarded).len(), 1);
        assert!(support::find::<CounterEffect>(&guarded)[0].target.is_target());
    }
}
