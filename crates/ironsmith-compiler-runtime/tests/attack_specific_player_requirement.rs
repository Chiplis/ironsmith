//! cf8 p05: "<creature> attacks that player this combat if able" and
//! "attacks a player each combat if able" as a rule-effect requirement to
//! attack a specific player (CR 508.1d), Restriction::MustAttackPlayer.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "attack_specific_player_requirement";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 4);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn requirements_name_the_player_and_their_duration() {
    for name in ["Ruhan of the Fomori", "Raving Dead", "Ursine Monstrosity"] {
        support::assert_markers(CLUSTER, name, &["MustAttackPlayer", "EndOfCombat"]);
    }
    support::assert_markers(
        CLUSTER,
        "Nahiri, the Unforgiving",
        &["MustAttackPlayer", "YourNextTurn", "Opponent"],
    );
}
