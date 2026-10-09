//! "Draw X cards, then you get half X rad counters, rounded up."
//! (Contaminated Drink): a rounded half count for player counters (CR 107.1a
//! rounding as stated). Source-authored, deliberately unrun.

#[path = "p02_line_families/compile.rs"]
mod compile;

const CONTAMINATED_DRINK: &str = "Mana cost: {X}{U}{B}\nType: Instant\nDraw X cards, then you get half X rad counters, rounded up.";

#[test]
fn contaminated_drink_gives_half_x_rounded_up_rad_counters() {
    for definition in compile::compile_both("Contaminated Drink", CONTAMINATED_DRINK) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("DrawCardsEffect"), "{debug}");
        assert!(debug.contains("HalfRoundedDown(Add(X, Fixed(1)))"), "rounded up: {debug}");
        assert!(debug.to_ascii_lowercase().contains("rad"), "{debug}");
    }
}
