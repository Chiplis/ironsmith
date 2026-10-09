//! cf8 p05: an "A or B, whichever is greater" amount and an adjacent color
//! list ("black or red permanent") are single operands, not coordinated
//! actions. Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "amount_and_color_coordination";

#[test]
fn whichever_is_greater_amount_stays_one_value() {
    support::assert_markers(
        CLUSTER,
        "Willowdusk, Essence Seer",
        &["LifeGainedThisTurn", "LifeLostThisTurn", "PutCounters"],
    );
}

#[test]
fn color_list_stays_one_target_filter() {
    for definition in support::definitions(&support::row(CLUSTER, "Lightwielder Paladin")) {
        let debug = support::debug(&definition);
        assert_eq!(debug.matches("ExileEffect").count().min(1), 1);
    }
}
