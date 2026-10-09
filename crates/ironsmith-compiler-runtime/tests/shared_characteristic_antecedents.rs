//! cf8 p01 round 5: the antecedent of a shared-characteristic comparison is
//! the object the ability already named — the earlier target ("that
//! permanent") or the card an exile-from-hand cost exiled ("the card exiled
//! this way") — never the triggering object. Unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn reveal_until_shares_a_card_type_with_the_earlier_target() {
    for name in ["Reality Scramble", "Wild Magic Surge"] {
        for definition in support::definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("ConsultTopOfLibrary"), "{name}: {debug}");
            assert!(debug.contains("SharesCardType"), "{name}: {debug}");
            assert!(!debug.contains("\"triggering\""), "{name}: {debug}");
            let text = support::rendered(&definition);
            support::assert_no_internal_markers(name, &text);
            assert!(text.contains("shares a card type"), "{name}: {text}");
        }
    }
}

#[test]
fn holistic_wisdom_compares_with_the_card_its_cost_exiled() {
    for definition in support::definitions("Holistic Wisdom") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("__cost_exiled_from_hand__"), "{debug}");
        assert!(debug.contains("SharesCardType"), "{debug}");
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Holistic Wisdom", &text);
        assert!(text.contains("shares a card type"), "{text}");
    }
}
