//! cf8/p07: "each opponent who doesn't control an Elf loses 1 life" applies
//! the effect to each opponent for whom "controls an Elf" is false.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Condition;
use ironsmith::effects::{ConditionalEffect, LoseLifeEffect};

const FIXTURE: &str = include_str!("../../../fixtures/negated_participant_control.json.fixture");

#[test]
fn thornbow_archer_drains_only_opponents_without_an_elf() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Thornbow Archer");
    assert_eq!(row["oracle_id"], "a9b35457-0f1c-495c-a508-6f6693835051");
    for definition in support::definitions(row) {
        let all: Vec<_> = support::triggered(&definition)
            .into_iter()
            .flat_map(support::triggered_effects)
            .collect();
        let conditionals = support::find::<ConditionalEffect>(&all);
        assert_eq!(conditionals.len(), 1);
        let Condition::Not(inner) = &conditionals[0].condition else {
            panic!("negated control condition: {:?}", conditionals[0].condition);
        };
        assert!(matches!(inner.as_ref(), Condition::PlayerControls { .. }), "{inner:?}");
        assert!(conditionals[0].if_false.is_empty());
        let mut guarded = Vec::new();
        for effect in &conditionals[0].if_true {
            support::collect(effect, &mut guarded);
        }
        assert_eq!(support::find::<LoseLifeEffect>(&guarded).len(), 1);
    }
}
