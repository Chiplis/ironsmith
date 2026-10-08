//! cf8 p05: "Exile a card from your graveyard for each 1 damage prevented
//! this way" repeats the follow-up once per point prevented (reusing the
//! prevention follow-up static). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

#[test]
fn immortal_coil_exiles_once_per_point_prevented() {
    support::assert_markers(
        "prevention_follow_up_repeat",
        "Immortal Coil",
        &["PreventMatchingDamageWithFollowUp", "Repeat", "Prevented"],
    );
}
