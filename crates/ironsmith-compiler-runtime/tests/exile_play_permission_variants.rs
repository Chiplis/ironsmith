//! cf8 p05: play-from-exile permission variants on the existing
//! GrantPlayTagged / CastTagged machinery — free price with an exile
//! lifetime in either order, "lands and cast spells from among cards exiled
//! this way" (CR 305.1, 601.1), "a card exiled with <source>" (one shared
//! play), "cards you own exiled with <source>", "until the beginning of your
//! next upkeep" (no priority in the untap step, CR 502.4), the singular
//! any-type mana rider, and a game-state condition on a hideaway play.
//! Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

const CLUSTER: &str = "exile_play_permission_variants";

#[test]
fn every_cluster_card_compiles_strictly_on_both_routes() {
    let rows = support::rows(CLUSTER);
    assert_eq!(rows.len(), 8);
    for row in &rows {
        support::definitions(row);
    }
}

#[test]
fn permission_shapes_lower_to_their_typed_grants() {
    support::assert_markers(CLUSTER, "Raphael, Most Attitude", &["max_plays: Some(1)", "UntilEndOfTurn"]);
    support::assert_markers(CLUSTER, "Kayla's Music Box", &["UntilEndOfTurn", "owner: Some("]);
    support::assert_markers(CLUSTER, "Extract Power", &["ForAsLongAsExiled"]);
    support::assert_markers(CLUSTER, "Magus of the Mind", &["UntilEndOfTurn"]);
    support::assert_markers(CLUSTER, "Elkin Bottle", &["UntilYourNextTurnStart"]);
    support::assert_markers(CLUSTER, "Klaw, Master of Sound", &["ForAsLongAsExiled", "AnyType"]);
    support::assert_markers(CLUSTER, "Howltooth Hollow", &["MaxCardsInHand"]);
}
