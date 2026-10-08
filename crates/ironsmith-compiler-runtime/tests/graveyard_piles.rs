//! cf8 p01 round 3: you separate a graveyard pool into two piles and an
//! opponent picks the pile that is exiled (CR 700.3). Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn death_or_glory_uses_a_real_pile_split() {
    for definition in support::definitions("Death or Glory") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Death or Glory", &text);
        assert!(text.contains("separate all creature cards in your graveyard into two piles"), "{text}");
        assert!(text.contains("exile the pile of an opponent's choice and return the other to the battlefield"), "{text}");
    }
}
