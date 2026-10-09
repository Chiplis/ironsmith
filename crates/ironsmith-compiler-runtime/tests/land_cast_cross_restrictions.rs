//! Rock Jockey: "You can't cast Rock Jockey if you've played a land this
//! turn." / "You can't play lands if this creature was cast this turn."
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const ROCK_JOCKEY: &str = "Mana cost: {2}{R}\nType: Creature — Goblin\nPower/Toughness: 3/3\nYou can't cast Rock Jockey if you've played a land this turn.\nYou can't play lands if this creature was cast this turn.";

#[test]
fn rock_jockey_lowers_both_cross_restrictions() {
    for definition in support::definitions("Rock Jockey", ROCK_JOCKEY) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("PlayerPlayedLandThisTurn"), "{debug}");
        assert!(debug.contains("Not("), "castable only when no land was played: {debug}");
        assert!(debug.contains("PlayLandsMatching"), "{debug}");
        assert!(debug.contains("SourceWasCast"), "{debug}");
        assert!(debug.contains("SourceEnteredBattlefieldThisTurn"), "{debug}");
    }
}
