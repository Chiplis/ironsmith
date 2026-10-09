//! UNVALIDATED implementation-first coverage (cf8 p09): "Target player dealt
//! damage by this creature this turn" restricts the target to players this
//! exact source dealt positive damage to during the current turn.
use ironsmith::target::PlayerFilter;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn wicked_akuba_targets_players_it_damaged_this_turn() {
    let rows = common::rows(include_str!("../../../fixtures/damaged_by_source_this_turn.json.fixture"));
    let expected = format!(
        "{:?}",
        PlayerFilter::was_dealt_damage_by_source_this_turn(PlayerFilter::Any)
    );
    for definition in common::definitions(common::row(&rows, "Wicked Akuba")) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains(&expected), "{debug}");
    }
}
