//! cf8 p05: "you may reselect which player or permanent <attacking creature>
//! is attacking" lowers to ReselectAttackTargetEffect: the creature stays
//! attacking and the effect's controller picks among what it could attack
//! (CR 508.1b). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "reselect_attack_target";

#[test]
fn every_cluster_card_reselects_an_attack() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 3);
    for row in &rows {
        support::assert_markers(
            CLUSTER,
            row["name"].as_str().unwrap(),
            &["ReselectAttackTargetEffect", "players_only: false"],
        );
    }
}
