//! "Look at defending player's hand. You may put a creature card from it onto
//! the battlefield under your control tapped and attacking that player or a
//! planeswalker they control." (Zara): the put's source is the looked-at hand.
//! Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::{MoveToZoneAttackTargetMode, MoveToZoneEffect};
use ironsmith::target::PlayerFilter;
use ironsmith::Zone;

const ZARA: &str = "Mana cost: {3}{U}{R}\nType: Legendary Creature — Human Pirate\nPower/Toughness: 4/3\nFlying\nWhenever Zara attacks, look at defending player's hand. You may put a creature card from it onto the battlefield under your control tapped and attacking that player or a planeswalker they control. Return that creature to its owner's hand at the beginning of the next end step.";

#[test]
fn zara_puts_a_creature_from_the_defending_hand_attacking_that_player() {
    for definition in support::definitions("Zara, Renegade Recruiter", ZARA) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("LookAtHand"), "{debug}");
        let moves = support::find_all::<MoveToZoneEffect>(&definition);
        let entry = moves
            .iter()
            .find(|effect| effect.zone == Zone::Battlefield)
            .unwrap_or_else(|| panic!("{moves:#?}"));
        assert!(entry.enters_tapped && entry.enters_attacking);
        assert_eq!(
            entry.attack_target_mode,
            Some(MoveToZoneAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                PlayerFilter::Defending
            ))
        );
        assert!(debug.contains("Hand"), "{debug}");
        assert!(
            moves.iter().any(|effect| effect.zone == Zone::Hand),
            "the delayed return to its owner's hand: {moves:#?}"
        );
    }
}
