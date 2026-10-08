//! cf8 p05: "The flashback cost is equal to that card's mana cost." completes
//! the flashback grant like "its mana cost". Source-authored; deliberately unrun.
#[path = "cf8_p05_support/mod.rs"]
mod support;

#[test]
fn sphinx_grants_flashback_at_the_cards_mana_cost() {
    support::assert_markers(
        "flashback_cost_reference",
        "Sphinx of Forgotten Lore",
        &["FlashbackFromCardManaCost"],
    );
}
