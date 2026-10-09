//! "put ... onto the battlefield tapped and attacking that opponent": the
//! entering attacker attacks the named player itself, never one of their
//! planeswalkers (CR 508.4). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::{MoveToZoneAttackTargetMode, MoveToZoneEffect};
use ironsmith::target::PlayerFilter;
use ironsmith::Zone;

const KAALIA: &str = "Mana cost: {1}{R}{W}{B}\nType: Legendary Creature — Human Cleric\nPower/Toughness: 2/2\nFlying\nWhenever Kaalia attacks an opponent, you may put an Angel, Demon, or Dragon creature card from your hand onto the battlefield tapped and attacking that opponent.";

#[test]
fn kaalia_puts_the_creature_onto_the_battlefield_attacking_that_opponent_only() {
    for definition in support::definitions("Kaalia of the Vast", KAALIA) {
        let moves = support::find_all::<MoveToZoneEffect>(&definition);
        let entry = moves
            .iter()
            .find(|effect| effect.zone == Zone::Battlefield)
            .unwrap_or_else(|| panic!("{moves:#?}"));
        assert!(entry.enters_tapped);
        assert!(entry.enters_attacking);
        assert_eq!(
            entry.attack_target_mode,
            Some(MoveToZoneAttackTargetMode::Player(PlayerFilter::Defending)),
            "the defending opponent itself, not their planeswalkers"
        );
    }
}
