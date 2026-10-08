//! cf8/p07: "the number of +1/+1 counters on him" names the legendary
//! source's own counters, exactly as "on it" does. Source-authored, unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::DealDamageEffect;
use ironsmith::CounterType;

const FIXTURE: &str = include_str!("../../../fixtures/gendered_counter_references.json.fixture");

#[test]
fn red_hulk_deals_damage_equal_to_its_own_counters() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Red Hulk");
    assert_eq!(row["oracle_id"], "a14a838a-49f4-45e3-9ec0-3c803af45a68");
    for definition in support::definitions(row) {
        let all: Vec<_> = support::triggered(&definition)
            .into_iter()
            .flat_map(support::triggered_effects)
            .collect();
        let damage = support::find::<DealDamageEffect>(&all);
        assert_eq!(damage.len(), 1, "the reflexive damage instruction");
        match damage[0].amount.unhinted() {
            Value::CountersOnSource(CounterType::PlusOnePlusOne) => {}
            Value::CountersOn(holder, Some(CounterType::PlusOnePlusOne))
                if matches!(holder.base(), ironsmith::target::ChooseSpec::Source) => {}
            other => panic!("source +1/+1 counter count: {other:?}"),
        }
    }
}
