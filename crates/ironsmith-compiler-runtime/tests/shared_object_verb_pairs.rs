//! "Each opponent chooses a creature they control. Tap and goad the chosen
//! creatures." (Fell Beast's Shriek): both verbs act on the chosen set, in
//! order (tap, then goad CR 701.38). Source-authored, deliberately unrun.

#[path = "p02_line_families/compile.rs"]
mod compile;

const FELL_BEASTS_SHRIEK: &str = "Mana cost: {U}{R}\nType: Sorcery\nEach opponent chooses a creature they control. Tap and goad the chosen creatures. (Until your next turn, those creatures attack each combat if able and attack a player other than you if able.)\nSplice onto instant or sorcery {2}{U}{R} (As you cast an instant or sorcery spell, you may reveal this card from your hand and pay its splice cost. If you do, add this card's effects to that spell.)";

#[test]
fn fell_beasts_shriek_taps_then_goads_the_chosen_creatures() {
    for definition in compile::compile_both("Fell Beast's Shriek", FELL_BEASTS_SHRIEK) {
        let debug = format!("{:?}", definition.spell_effect);
        let tap = debug.find("TapEffect").expect("tap");
        let goad = debug.find("Goad").expect("goad");
        assert!(tap < goad, "tap precedes goad: {debug}");
        assert!(!debug.contains("Target("), "no targets are announced: {debug}");
    }
}
