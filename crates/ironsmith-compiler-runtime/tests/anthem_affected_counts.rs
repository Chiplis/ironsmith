//! UNVALIDATED implementation-first coverage (cf8 p09): anthem counts for
//! players who have lost the game (CR 104.3) and chroma (CR 702.49: mana
//! symbols of a color in the affected creature's own mana cost).
#[path = "p09_common/mod.rs"]
mod common;

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/anthem_affected_counts.json.fixture"))
}

#[test]
fn frogantua_counts_players_who_lost() {
    for definition in common::definitions(common::row(&rows(), "Rampant Frogantua")) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("PlayersLostGame"), "{debug}");
    }
}

#[test]
fn light_from_within_counts_white_symbols_of_each_creature() {
    for definition in common::definitions(common::row(&rows(), "Light from Within")) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("ManaSymbolsOfColorInAffectedCost(White)"), "{debug}");
    }
}
