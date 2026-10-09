//! Spy Network: one look over a target player's hand, the top card of their
//! library and their face-down creatures. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

const SPY_NETWORK: &str = "Mana cost: {U}\nType: Instant\nLook at target player's hand, the top card of that player's library, and any face-down creatures they control. Look at the top four cards of your library, then put them back in any order.";

#[test]
fn spy_network_looks_at_hand_top_card_and_face_down_creatures() {
    for definition in support::definitions("Spy Network", SPY_NETWORK) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("LookAtHand"), "{debug}");
        assert!(debug.contains("face_down: true"), "{debug}");
        assert!(debug.contains("LookAtTopCards"), "{debug}");
    }
}
