//! Static abilities that function only during a step of another player's
//! turn: "During each opponent's end step, you may cast spells as though they
//! had flash." (CR 604.2, 611.3a). Source-authored, deliberately unrun.
#[path = "p02_line_families/compile.rs"]
mod compile;

const PHANTOM: &str = "Mana cost: {2}{U}\nType: Creature — Spirit Detective\nPower/Toughness: 1/4\nFlash\nFlying\nDuring each opponent's end step, you may cast spells as though they had flash.";

#[test]
fn final_word_phantom_flash_permission_is_gated_on_an_opponents_end_step() {
    for definition in compile::compile_both("Final-Word Phantom", PHANTOM) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("OpponentsEndStep"), "{debug}");
        assert!(debug.contains("Conditional"), "{debug}");
        assert!(debug.contains("Grants"), "{debug}");
    }
}
