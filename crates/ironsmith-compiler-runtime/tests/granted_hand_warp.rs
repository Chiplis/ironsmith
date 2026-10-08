//! "Artifact cards and red creature cards in your hand have warp {2}{R}."
//! (Tannuk, Steadfast Second): CR 702.185 warp granted to hand cards as a
//! typed alternative cast grant. Source-authored, deliberately unrun.
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p02_line_families/compile.rs"]
mod compile;

const TANNUK: &str = "Mana cost: {2}{R}{R}\nType: Legendary Creature — Kavu Pilot\nPower/Toughness: 3/5\nOther creatures you control have haste.\nArtifact cards and red creature cards in your hand have warp {2}{R}. (You may cast a card from your hand for its warp cost. Exile that permanent at the beginning of the next end step, then you may cast it from exile on a later turn.)";

#[test]
fn tannuk_grants_warp_to_matching_hand_cards() {
    for definition in compile::compile_both("Tannuk, Steadfast Second", TANNUK) {
        let grants = compile::statics(&definition, StaticAbilityId::Grants);
        let warp = grants
            .iter()
            .map(|ability| format!("{ability:?}"))
            .find(|debug| debug.contains("Warp"))
            .expect("a granted warp alternative cast");
        assert!(warp.contains("zone: Hand"), "{warp}");
        assert!(warp.contains("Artifact"), "{warp}");
        assert!(warp.contains("Creature"), "{warp}");
        assert!(warp.contains("Red"), "{warp}");
        assert!(!warp.contains("PlayFrom"), "{warp}");
    }
}
