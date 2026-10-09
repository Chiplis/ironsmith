//! Mana Maze: players can't cast spells sharing a color with the spell most
//! recently cast this turn. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const MANA_MAZE: &str = "Mana cost: {1}{U}\nType: Enchantment\nPlayers can't cast spells that share a color with the spell most recently cast this turn.";

#[test]
fn mana_maze_lowers_a_last_spell_color_cast_restriction() {
    for definition in support::definitions("Mana Maze", MANA_MAZE) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("CastSpellsMatching"), "{debug}");
        assert!(debug.contains("shares_color_with_last_spell_cast_this_turn: true"), "{debug}");
    }
}
