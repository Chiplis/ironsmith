//! cf8/p07: `<trigger>, if <condition>, choose one —` keeps the condition as
//! a CR 603.4 intervening-if on the triggered ability instead of reading it
//! as a resolution-time prefix effect. Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effects::ChooseModeEffect;
use ironsmith::effect::Value;

const FIXTURE: &str = include_str!("../../../fixtures/modal_trigger_intervening_if.json.fixture");

#[test]
fn modal_triggers_keep_their_intervening_if_on_both_routes() {
    let rows = support::rows(FIXTURE);
    assert_eq!(rows.len(), 3);
    for (name, oracle_id) in [
        ("Tetsuo, Imperial Champion", "1111e042-0bce-4819-ab42-9912aed4d930"),
        ("Vision, Synthezoid Avenger", "2f4360f6-9ac7-499f-b0a5-769550ea15ba"),
        ("Wardens of the Cycle", "fa7d09aa-2ebc-4571-81da-836cd80cf554"),
    ] {
        let row = support::row(&rows, name);
        assert_eq!(row["oracle_id"], oracle_id);
        for definition in support::definitions(row) {
            let modal_triggers: Vec<_> = support::triggered(&definition)
                .into_iter()
                .filter(|triggered| {
                    support::find::<ChooseModeEffect>(&support::triggered_effects(triggered))
                        .len()
                        == 1
                })
                .collect();
            assert_eq!(modal_triggers.len(), 1, "{name}");
            let triggered = modal_triggers[0];
            assert!(
                triggered.intervening_if.is_some(),
                "{name}: the `if` clause is an intervening-if condition (CR 603.4)"
            );
            let all = support::triggered_effects(triggered);
            let modal = &support::find::<ChooseModeEffect>(&all)[0];
            assert_eq!(modal.modes.len(), 2, "{name}");
            assert_eq!(modal.choose_count, Value::Fixed(1), "{name}");
            assert_eq!(modal.min_choose_count, Value::Fixed(1), "{name}");
            // The condition is not duplicated as a resolution-time gate.
            assert!(
                !all.iter().any(|effect| effect
                    .downcast_ref::<ironsmith::effects::ConditionalEffect>()
                    .is_some()),
                "{name}: no resolution-time conditional wrapper"
            );
        }
    }
}
