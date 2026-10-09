//! cf8 p05: "choose target player and any number of target attacking
//! creatures their opponents control. Those creatures are now attacking that
//! player." The creature targets are linked to the earlier target player
//! (their controller must be one of that player's opponents, CR 115.1), and
//! each targeted creature is redirected to attack that player with no choice
//! (CR 506.4). Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

use ironsmith::effects::ReselectAttackTargetEffect;
use ironsmith::target::PlayerFilter;

const CLUSTER: &str = "forced_attack_reassignment";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 1);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn portal_manipulator_redirects_targeted_attackers_to_the_target_player() {
    for definition in support::definitions(&support::row(CLUSTER, "Portal Manipulator")) {
        let reselects = support::effects_of::<ReselectAttackTargetEffect>(&definition);
        assert_eq!(reselects.len(), 1, "{reselects:?}");
        let reselect = &reselects[0];
        assert!(reselect.players_only);
        assert!(
            matches!(&reselect.attacked_player, Some(PlayerFilter::Target(_))),
            "the creatures now attack the declared target player: {:?}",
            reselect.attacked_player
        );
        let debug = support::debug(&definition);
        // "their opponents control" is relative to the target player, not
        // an iterated player.
        assert!(debug.contains("OpponentOf(Target("), "{debug}");
        assert!(!debug.contains("OpponentOf(IteratedPlayer)"), "{debug}");
    }
}
