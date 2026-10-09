//! Coordinated action arms: shared object, blight arm, self-source damage
//! sibling (p12-other). Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

use ironsmith::effects::{DrawCardsEffect, GainLifeEffect, GoadEffect};

#[test]
fn besmirch_untaps_and_goads_the_same_creature() {
    for definition in support::definitions("Besmirch") {
        let effects = support::effects(&definition);
        let goad = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<GoadEffect>())
            .expect("goad arm");
        let untap = effects
            .iter()
            .find(|effect| format!("{effect:?}").contains("Untap"))
            .expect("untap arm with an operand");
        let untap_debug = format!("{untap:?}");
        assert!(
            untap_debug.contains("Tagged"),
            "the bare untap arm shares the goaded creature: {untap_debug}"
        );
        assert!(format!("{:?}", goad.target).contains("Tagged"));
    }
}

#[test]
fn sinister_gnarlbark_draws_then_blights() {
    for definition in support::definitions("Sinister Gnarlbark") {
        let effects = support::effects(&definition);
        assert!(effects.iter().any(|effect| effect.downcast_ref::<DrawCardsEffect>().is_some()));
        assert!(
            effects.iter().any(|effect| format!("{effect:?}").contains("Blight")),
            "blight is its own executable arm"
        );
    }
}

#[test]
fn inspired_ultimatum_keeps_all_three_actions() {
    for definition in support::definitions("Inspired Ultimatum") {
        let effects = support::effects(&definition);
        let gain = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<GainLifeEffect>())
            .expect("life gain arm");
        assert!(format!("{gain:?}").contains("Fixed(5)"));
        assert!(effects.iter().any(|effect| format!("{effect:?}").contains("DealDamage")));
        let draw = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<DrawCardsEffect>())
            .expect("draw arm");
        assert_eq!(draw.count.unhinted(), &ironsmith::effect::Value::Fixed(5));
    }
}
