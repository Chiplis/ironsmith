//! cf8/p07: a creature's own keyword with a defined X ("<Name> has bushido X,
//! where X is ...", "<Name> has soulshift X, where X is ..."). Bushido X:
//! whenever it blocks or becomes blocked, it gets +X/+X until end of turn
//! (CR 702.45a); soulshift X returns a Spirit card with mana value X or less
//! (CR 702.46a). X is read as the triggered ability resolves.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::effect::Value;
use ironsmith::effects::ModifyPowerToughnessEffect;

const FIXTURE: &str = include_str!("../../../fixtures/defined_x_keywords.json.fixture");

#[test]
fn fumiko_has_bushido_equal_to_the_number_of_attacking_creatures() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Fumiko the Lowblood");
    assert_eq!(row["oracle_id"], "cba58942-b98c-45aa-8651-7dc666e31c93");
    for definition in support::definitions(row) {
        let debug = format!("{:?}", definition.abilities);
        assert!(!debug.contains("KeywordFallbackText"), "{debug}");
        let pumps: Vec<ModifyPowerToughnessEffect> = support::triggered(&definition)
            .into_iter()
            .flat_map(support::triggered_effects)
            .filter_map(|effect| effect.downcast_ref::<ModifyPowerToughnessEffect>().cloned())
            .collect();
        assert_eq!(pumps.len(), 1, "{debug}");
        let pump_debug = format!("{:?}", pumps[0]);
        assert!(pump_debug.contains("Count"), "{pump_debug}");
        assert!(pump_debug.contains("attacking"), "{pump_debug}");
        assert!(debug.contains("Blocks") || debug.contains("blocks"), "{debug}");
    }
}

#[test]
fn kodama_of_the_center_tree_has_soulshift_equal_to_its_spirits() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Kodama of the Center Tree");
    assert_eq!(row["oracle_id"], "88188cc1-3211-4bf8-a591-613b66192695");
    for definition in support::definitions(row) {
        let debug = format!("{:?}", definition.abilities);
        assert!(!debug.contains("KeywordFallbackText"), "{debug}");
        assert!(debug.contains("Soulshift"), "{debug}");
        assert!(debug.contains("LessThanOrEqualExpr"), "{debug}");
        assert!(debug.contains("Spirit"), "{debug}");
        let _ = Value::X;
    }
}
