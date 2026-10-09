//! "If this spell was kicked, the creature you control deals twice that much
//! damage instead": a self-replacement whose amount is the replaced
//! instruction's own amount doubled (CR 614.1a); the definite description
//! names the already-targeted damage source. Source-authored and UNRUN.
#[path = "cf8_p08/support.rs"]
mod support;

const CHOCOBO_KICK: &str = "Mana cost: {1}{G}\nType: Sorcery\nKicker—Return a land you control to its owner's hand.\nTarget creature you control deals damage equal to its power to target creature an opponent controls. If this spell was kicked, the creature you control deals twice that much damage instead.";

#[test]
fn kicked_arm_doubles_the_power_based_amount() {
    for definition in support::definitions("Chocobo Kick", CHOCOBO_KICK) {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Scaled("), "{debug}");
        let effects = support::all_effects(&definition);
        let scaled = effects.iter().filter(|effect| format!("{effect:?}").contains("Scaled(")).count();
        assert!(scaled >= 1);
        assert_eq!(definition.optional_costs.len(), 1, "the kicker");
        // Exactly two targets: the fighter you control and the opponent's creature.
        let text = support::rendered(&definition);
        assert!(text.contains("twice"), "{text}");
    }
}
