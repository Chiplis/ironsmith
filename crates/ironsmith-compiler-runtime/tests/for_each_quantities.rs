//! cf8/p07: shared "for each" count vocabulary additions:
//! "for each of its colors" and "for each poison counter they have".
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::DiscardEffect;
use ironsmith::CounterType;

const FIXTURE: &str = include_str!("../../../fixtures/for_each_quantities.json.fixture");

#[test]
fn might_of_the_nephilim_scales_by_the_targets_colors() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Might of the Nephilim");
    assert_eq!(row["oracle_id"], "4fda9b70-8da5-4292-b05a-0a0e5a2ad809");
    for definition in support::definitions(row) {
        let debug = format!("{:?}", support::spell_effects(&definition));
        assert!(debug.contains("ColorsOf"), "the bonus counts colors: {debug}");
        assert!(debug.contains("Scaled"), "+2/+2 per color: {debug}");
        assert!(!debug.contains("Fixed(2), toughness: Fixed(2)"), "{debug}");
    }
}

#[test]
fn whispering_specter_discards_per_poison_counter_of_that_player() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Whispering Specter");
    assert_eq!(row["oracle_id"], "18d270bf-d0d3-4fac-a8d2-5f920e789bb5");
    for definition in support::definitions(row) {
        let all: Vec<_> = support::triggered(&definition)
            .into_iter()
            .flat_map(support::triggered_effects)
            .collect();
        let discards = support::find::<DiscardEffect>(&all);
        assert_eq!(discards.len(), 1);
        assert!(
            matches!(discards[0].count.unhinted(), Value::PlayerCounters(_, CounterType::Poison)),
            "{:?}",
            discards[0].count
        );
    }
}
